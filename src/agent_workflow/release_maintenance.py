"""Project-only release binding and quiescent lifecycle operations."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess

from .runtime import load
from .storage import OperationalError, alive, atomic, lock, read


def _sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def _release(catalog, release_id):
    if not release_id or '/' in release_id or '..' in release_id:
        raise OperationalError('Invalid release ID')
    root=(Path(catalog)/release_id).resolve(strict=True)
    if root.parent!=Path(catalog).resolve(strict=True):
        raise OperationalError('Release path escapes the installed catalog')
    manifest=read(root/'manifest.json')
    if (manifest.get('schema_version')!=1 or manifest.get('release_id')!=release_id or
        manifest.get('protocol_major')!=1 or manifest.get('state_schema_min',1)>1 or
        manifest.get('state_schema_max',1)<1 or not (root/'bin/waywr').is_file() or
        not (root/'venv/bin/python').is_file()):
        raise OperationalError('Target release is missing or incompatible with this project state')
    return root,_sha(root/'manifest.json')


def _quiescent(paths):
    db_path=paths.state/'state.sqlite'
    if db_path.exists():
        db=sqlite3.connect(db_path.as_uri()+'?mode=ro',uri=True,timeout=2)
        try:
            db.row_factory=sqlite3.Row
            db.execute('PRAGMA query_only=ON')
            if db.execute("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='app_runs'").fetchone()[0]:
                if db.execute('SELECT 1 FROM app_runs WHERE active=1 LIMIT 1').fetchone():
                    raise OperationalError('Selected project has an unresolved workflow')
            if db.execute("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='session_tasks'").fetchone()[0]:
                tasks=db.execute("SELECT * FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled')").fetchall()
                for row in tasks:
                    task=dict(row)
                    if task.get('status')=='blocked' and task.get('run_id') is None:
                        call=db.execute('SELECT * FROM session_coordinator_calls WHERE task_id=?',(task['task_id'],)).fetchone()
                        if call:
                            from .coordinator_recovery import verify
                            try:verify(paths,task,dict(call))
                            except (OperationalError,OSError,KeyError,TypeError,ValueError):pass
                            else:continue
                    raise OperationalError('Selected project has queued or unresolved tasks')
            if db.execute("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='session_coordinator_calls'").fetchone()[0]:
                if db.execute("SELECT 1 FROM session_coordinator_calls WHERE status='reserved' LIMIT 1").fetchone():
                    raise OperationalError('Selected project has an unresolved coordinator call')
        finally:db.close()
    for path in (paths.state/'session/ui-claims').glob('*.json'):
        claim=read(path)
        if claim.get('project_key')==paths.key and alive(claim['process']):
            raise OperationalError('Selected project UI is still active; detach it first')
    for path in (paths.state/'runs').glob('*/operations/*/claim.json'):
        operation=path.parent
        if (operation/'receipt.json').exists():continue
        claim=read(path)
        if alive(claim['runner']):
            raise OperationalError('Selected project has a live worker without a receipt')


def _unit(paths):return f'waywr-{paths.key}.service'


def pending_binding(paths):
    root=paths.state/'session/release-bindings'
    intents=sorted(root.glob('*-intent.json'),key=lambda p:int(p.name.split('-')[0]))
    for intent_path in intents:
        complete=root/intent_path.name.replace('-intent.json','-complete.json')
        if not complete.exists():
            return read(intent_path),complete
    return None,None


def _stop_unit(paths):
    unit=_unit(paths)
    status=subprocess.run(['systemctl','--user','is-active',unit],capture_output=True,text=True)
    if status.returncode==0:
        stopped=subprocess.run(['systemctl','--user','stop',unit],capture_output=True,text=True)
        if stopped.returncode:
            raise OperationalError(f'Could not stop selected project unit {unit}: {stopped.stderr.strip()}')
    return unit


def manage(action, project, state_dir, registry_dir, catalog, target_release=None):
    paths=load(project,state_dir,registry_dir)
    descriptor=paths.project/'.wayward-wrangler/release.json'
    with lock(paths.state/'session/maintenance.lock'):
        pending,complete=pending_binding(paths)
        if pending:
            existing=read(descriptor) if descriptor.exists() else None
            if (existing and existing.get('release_id')==pending['target_release_id'] and
                existing.get('manifest_sha256')==pending['target_manifest_sha256'] and
                existing.get('binding_generation')==pending['generation']):
                atomic(complete,{'schema_version':1,'binding':existing},immutable=True)
                pending=None
            elif action!='status' and (action!=pending['action'] or target_release!=pending['target_release_id']):
                raise OperationalError('Incomplete project release binding; repeat its exact target action to reconcile')
        if action=='status':
            binding=read(descriptor) if descriptor.exists() else None
            status=subprocess.run(['systemctl','--user','is-active',_unit(paths)],capture_output=True,text=True)
            session=None
            database=paths.state/'state.sqlite'
            if database.exists():
                with sqlite3.connect(database.as_uri()+'?mode=ro',uri=True,timeout=2) as db:
                    table=db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='session_metadata'").fetchone()
                    row=db.execute("SELECT value FROM session_metadata WHERE name='lifecycle_v1'").fetchone() if table else None
                    if row: session=json.loads(row[0])
            pointer=paths.state/'session/checkpoints/latest.json'
            return {'schema_version':1,'project_key':paths.key,'release_binding':binding,
                    'binding_recovery':pending,'service':'active' if status.returncode==0 else 'inactive',
                    'unit':_unit(paths),'session':session,
                    'checkpoint':read(pointer) if pointer.exists() else None}
        _quiescent(paths)
        if action=='stop':
            return {'schema_version':1,'project_key':paths.key,'unit':_stop_unit(paths),'service':'stopped'}
        if action not in ('bind','upgrade','rollback'):
            raise OperationalError('Unknown project maintenance action')
        if not target_release:
            raise OperationalError('An explicit installed release ID is required')
        _root,target_hash=_release(catalog,target_release)
        prior=read(descriptor) if descriptor.exists() else None
        if action=='bind' and prior:
            raise OperationalError('Project already has a release binding; use upgrade or rollback')
        if action!='bind' and not prior:
            raise OperationalError('Project has no release binding; use explicit bind')
        if prior and prior.get('project_key')!=paths.key:
            raise OperationalError('Project release binding identity mismatch')
        if prior and prior.get('release_id')==target_release:
            if prior.get('manifest_sha256',target_hash)!=target_hash:
                raise OperationalError('Bound release manifest changed')
            return {'schema_version':1,'project_key':paths.key,'release_binding':prior,'status':'already-bound'}
        _stop_unit(paths)
        _quiescent(paths)
        generation=(prior or {}).get('binding_generation',0)+1
        intent={'schema_version':1,'project_key':paths.key,'generation':generation,
                'action':action,'old':prior,'target_release_id':target_release,
                'target_manifest_sha256':target_hash}
        root=paths.state/'session/release-bindings'
        atomic(root/f'{generation}-intent.json',intent,immutable=True)
        binding={'schema_version':1,'project_key':paths.key,'release_id':target_release,
                 'manifest_sha256':target_hash,'binding_generation':generation}
        atomic(descriptor,binding)
        atomic(root/f'{generation}-complete.json',{'schema_version':1,'binding':binding},immutable=True)
        return {'schema_version':1,'project_key':paths.key,'release_binding':binding,
                'status':'bound','service':'stopped'}
