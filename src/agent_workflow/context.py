"""Explicit reference selection; canonical records remain the authority."""
from pathlib import Path
import json
import tomllib
from . import records, artifacts
from .storage import ROOT, OperationalError, canonical, digest, read
from .runtime import current

def root():
    selected = current()
    return selected.project if selected else ROOT


def request(run_id, request_id):
    with records.connect() as db:
        row = db.execute('SELECT payload FROM app_requests WHERE run_id=? AND request_id=?',
                         (run_id, request_id)).fetchone()
    if not row:
        raise OperationalError(f'Missing source request: {run_id}/{request_id}')
    return json.loads(row['payload'])


def response(run_id, request_id):
    value = records.response(run_id, request_id)
    if not value:
        raise OperationalError(f'Missing source answer: {run_id}/{request_id}')
    exported = records.run_dir(run_id)/'artifacts'/f'decision-{records.name(request_id)}.json'
    if exported.exists() and read(exported) != value:
        raise OperationalError('Source response export disagrees with canonical record')
    return value


def project_file(path):
    path = Path(path).resolve()
    if not path.is_relative_to(root().resolve()) or not path.is_file():
        raise OperationalError('Context documents/selection must be existing project-local files')
    return path


def resolve(selection_path, exclude_run=None):
    path = project_file(selection_path)
    selection = tomllib.loads(path.read_text())
    if selection.get('schema_version') != 1 or not selection.get('scope'):
        raise OperationalError('Context selection requires version 1 and explicit scope')
    project = records.name(selection['project_id'])
    entries, seen, dependencies = [], set(), {}
    for spec in selection.get('entries', []):
        if not spec.get('reason') or spec.get('scope') != selection['scope']:
            raise OperationalError('Each context entry needs relevance and matching explicit scope')
        kind = spec['kind']
        base = {'kind':kind, 'scope':spec['scope'], 'relevance_reason':spec['reason']}
        if kind == 'document':
            doc = project_file(path.parent/spec['path'])
            # Resolve the supported canonical pointer once; do not duplicate AGENTS content.
            dependencies[str(doc)] = digest(doc.read_bytes())
            if doc.name == 'CLAUDE.md' and '[AGENTS.md](AGENTS.md)' in doc.read_text():
                doc = project_file(doc.parent/'AGENTS.md')
                dependencies[str(doc)] = digest(doc.read_bytes())
            entry = {**base,'qualified_id':'document:'+str(doc.relative_to(root())),
                     'source_path':str(doc),'source_record_digest':digest(doc.read_bytes()),'content':doc.read_text()}
            items = [entry]
        else:
            run = records.name(spec['run'])
            source = records.get_run(run)
            if source['status'] != 'approved':
                raise OperationalError('Selected source run is not approved')
            approval_id = spec.get('approval', 'A1')
            approval, final = response(run, approval_id), request(run, approval_id)
            if approval['outcome'] != 'approved' or approval['artifact_sha256'] != final['artifact_sha256'] or approval['decision_revision'] != final['decision_revision']:
                raise OperationalError('Source approval does not bind its final artifact/decisions')
            artifacts.content(run, final['artifact'])
            base.update(source_run=run, authority_ref=run+'/'+approval_id,
                        authority_digest=digest(canonical({'request':final,'response':approval})))
            if kind == 'decision':
                key=records.name(spec['id']); req=request(run,key); res=response(run,key)
                if req['kind'] != 'choice' or res['outcome'] != 'answered' or not any(d['request_id']==key and d['answer']==res['answer'] for d in final['decisions']):
                    raise OperationalError('Decision is not accepted in approved source context')
                if res['artifact_sha256'] != req['artifact_sha256'] or res['decision_revision'] != req['decision_revision']:
                    raise OperationalError('Source decision binding mismatch')
                entry={**base,'qualified_id':run+'/'+key,'topic_key':spec['topic_key'],
                       'source_record_digest':digest(canonical({'request':req,'response':res})),
                       'content':{'question':req['question'],'response':res}}
                items=[entry]
            elif kind == 'artifact':
                ref=read(records.run_dir(run)/'artifacts'/f"{records.name(spec['id'])}.json")['manifest']
                text=artifacts.content(run,ref)
                if ref != final['artifact']:
                    raise OperationalError('Selected artifact is not the approved source version')
                items=[{**base,'qualified_id':run+'/'+ref['id'],'artifact_sha256':ref['sha256'],
                        'source_record_digest':digest(canonical(ref)),'content':text}]
            elif kind == 'findings':
                findings=final['findings']
                ids=[k for k,v in findings.items() if v['status']=='open'] if spec.get('ids')==['open'] else spec['ids']
                items=[]
                for key in ids:
                    if key not in findings or findings[key]['status']!='open':
                        raise OperationalError('Selected finding is missing or no longer open')
                    f=findings[key]
                    origin=read(records.run_dir(run)/'artifacts'/f"{records.name(f['origin'])}.json")
                    artifacts.content(run,origin['manifest'])
                    ledger_path=records.run_dir(run)/'quality'/f"{records.name(f['origin'])}-finding-ledger.json"
                    if ledger_path.exists():
                        ledger=read(ledger_path)
                        if (ledger.get('result_sha256')!=digest(canonical(origin['result'])) or
                            not any(x.get('id')==key and x.get('evidence')==f['evidence']
                                    for x in ledger.get('new_findings',[]))):
                            raise OperationalError('Finding ledger does not support original evidence')
                    elif not any(x['id']==key and x['evidence']==f['evidence'] for x in origin['result']['findings']):
                        raise OperationalError('Finding origin does not support its evidence')
                    items.append({**base,'qualified_id':run+'/'+key,'source_record_digest':digest(canonical(f)),
                                  'content':{**f,'id':run+'/'+key,'source_id':key}})
                # Even an empty open-findings selection must notice changed source status.
                dependencies['findings:'+run]=digest(canonical(final))
            else:
                raise OperationalError('Unknown context entry kind')
        for entry in items:
            if entry['qualified_id'] in seen:
                if kind=='document': continue
                raise OperationalError('Duplicate context reference')
            seen.add(entry['qualified_id']); entries.append(entry)
    topics=[e['topic_key'] for e in entries if e['kind']=='decision']
    if len(set(topics)) != len(topics):
        raise OperationalError('Conflicting decisions selected for one topic')
    # An explicit approved supersession is discoverable from existing records, not a new ledger.
    if records.db_path().exists():
        with records.connect() as db:
            rows=db.execute("SELECT envelope, run_id FROM app_runs WHERE status='approved'").fetchall()
            for row in rows:
                if row["run_id"]==exclude_run: continue
                env=json.loads(row['envelope'])
                if env.get('context_snapshot',{}).get('project_id')!=project: continue
                answers=db.execute('SELECT payload FROM app_responses WHERE run_id=?',(row['run_id'],)).fetchall()
                for answer in answers:
                    if seen.intersection(json.loads(answer['payload']).get('supersedes',[])):
                        raise OperationalError('Selected context has an explicit approved supersession')
    snapshot={'schema_version':1,'project_id':project,'scope':selection['scope'],
              'selection_path':str(path),'selection_digest':digest(path.read_bytes()),
              'dependencies':dependencies,'entries':entries}
    selected = current()
    if selected and not selected.legacy:
        snapshot.update(schema_version=2, location_format=2, project_key=selected.key,
                        selection_path=str(path.relative_to(root())))
        snapshot['dependencies']={str(Path(p).relative_to(root())) if Path(p).is_absolute() else p: h
                                  for p,h in dependencies.items()}
        snapshot['entries']=[{**entry, **({'source_path':str(Path(entry['source_path']).relative_to(root()))}
                                 if 'source_path' in entry else {})} for entry in entries]
    if len(canonical(snapshot).encode())>48000:
        raise OperationalError('Selected context exceeds 48 KB; narrow explicit selection')
    return snapshot


def check(envelope):
    if envelope.get('workflow_version',1)!=2: return
    try:
        snapshot=envelope['context_snapshot']
        selected = current()
        if snapshot.get('schema_version')==2:
            if not selected or selected.key!=snapshot.get('project_key'):
                raise OperationalError('Project identity changed')
            selection = project_file(root()/snapshot['selection_path'])
        else:
            origin = selected.legacy_origin if selected and selected.legacy else None
            frozen = Path(snapshot['selection_path'])
            if origin:
                if not frozen.is_relative_to(origin):
                    raise OperationalError('Legacy selection lies outside the declared original root')
                selection = project_file(root()/frozen.relative_to(origin))
            else:
                selection = snapshot['selection_path']
        recomputed = resolve(selection,envelope.get('run_id'))
        if snapshot.get('schema_version')==1 and selected and selected.legacy_origin:
            old = str(selected.legacy_origin)
            new = str(selected.project)
            def frozen_namespace(value):
                if isinstance(value, dict):
                    return {((old + key[len(new):]) if key == new or key.startswith(new + '/') else key):
                            frozen_namespace(item) for key,item in value.items()}
                if isinstance(value, list):
                    return [frozen_namespace(item) for item in value]
                if isinstance(value, str) and (value == new or value.startswith(new + '/')):
                    return old + value[len(new):]
                return value
            recomputed = frozen_namespace(recomputed)
        if digest(canonical(snapshot))!=envelope['context_digest'] or recomputed!=snapshot:
            raise OperationalError('Selected sources or selection changed')
    except (OSError, KeyError, ValueError, OperationalError) as error:
        raise OperationalError(f'Stale context; no continuation: {error}') from error


def inherited(snapshot):
    return [{**e['content']['response'],'question':{**e['content']['question'],'topic_key':e['topic_key']},
             'qualified_id':e['qualified_id'],'source_run':e['source_run'],'scope':e['scope'],
             'source_record_digest':e['source_record_digest']} for e in snapshot['entries'] if e['kind']=='decision']


def effective_decisions(state):
    local=state.get('decisions',[])
    replaced={ref for d in local for ref in d.get('supersedes',[])}
    return [d for d in state.get('inherited_decisions',[]) if d['qualified_id'] not in replaced]+local


def evidence_ids(packet):
    ids={'brief'} | {e['qualified_id'] for e in packet['context_snapshot']['entries']}
    if packet.get('current_ref'): ids.add(packet['run_id']+'/'+packet['current_ref']['id'])
    if packet.get('critique'): ids.add(packet['run_id']+'/'+packet['critique']['artifact']['id'])
    ids.update(packet.get('finding_dispositions',{}))
    return sorted(ids)


QUESTION_FIELDS={'id','question','options','recommendation','consequences','topic_key',
                 'rationale','evidence_refs','related_decision_refs','reconsideration_reason'}


def validate_questions(result,packet):
    # A second well-formed question is retained, then operationally blocked by the graph.
    if len(result['questions'])>4: raise OperationalError('Question output exceeds bounded contract')
    known_refs={d.get('qualified_id',packet['run_id']+'/'+d['request_id']) for d in packet['decisions']}
    seen=set()
    for q in result['questions']:
        if set(q)!=QUESTION_FIELDS: raise OperationalError('Invalid version-2 question fields')
        for field in QUESTION_FIELDS-{'options','evidence_refs','related_decision_refs'}:
            if not isinstance(q[field],str) or len(q[field])>2000 or (not q[field].strip() and field!='reconsideration_reason'):
                raise OperationalError('Invalid question text/size')
        records.name(q['id'])
        if q['topic_key'] in seen: raise OperationalError('Duplicate question topic in result')
        seen.add(q['topic_key'])
        options=q['options']
        if not isinstance(options,list) or not 2<=len(options)<=3 or any(not isinstance(x,str) or not x.strip() or len(x)>200 for x in options) or len(set(options))!=len(options):
            raise OperationalError('Question needs 2–3 distinct options')
        for field,allowed in [('evidence_refs',set(evidence_ids(packet))),('related_decision_refs',known_refs)]:
            if not isinstance(q[field],list) or len(q[field])>12 or any(not isinstance(x,str) or x not in allowed for x in q[field]):
                raise OperationalError('Question refers to unknown evidence/decision')
        if not q['evidence_refs']: raise OperationalError('Question requires evidence')
        if bool(q['reconsideration_reason']) != bool(q['related_decision_refs']):
            raise OperationalError('Reconsideration requires prior decision references and a reason')
