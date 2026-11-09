"""Assemble a paired, offline-installable Linux release from already built artifacts.

This build step never installs a package. Download/build the pinned wheelhouse
separately, then inspect its exact contents here.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import sys
import zipfile
from email.parser import Parser


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def wheel_identity(path):
    with zipfile.ZipFile(path) as archive:
        names=[name for name in archive.namelist() if name.endswith('.dist-info/METADATA')]
        if len(names)!=1:raise ValueError(f'Wheel has ambiguous metadata: {path}')
        metadata=Parser().parsestr(archive.read(names[0]).decode())
    return metadata['Name'].lower().replace('_','-'),metadata['Version']


def main(argv=None):
    p=argparse.ArgumentParser()
    for flag in ('--rust-binary','--backend-wheel','--dependency-wheelhouse','--output','--release-id'):
        p.add_argument(flag,required=True)
    args=p.parse_args(argv)
    target=Path(args.output).expanduser().resolve()
    if target.exists():raise SystemExit(f'Bundle already exists: {target}')
    wheels=[wheel for wheel in Path(args.dependency_wheelhouse).glob('*.whl')
            if wheel_identity(wheel)[0]!='wayward-wrangler']+[Path(args.backend_wheel)]
    identities={}
    for wheel in wheels:
        ident=wheel_identity(wheel)
        if ident[0] in identities:raise SystemExit(f'Duplicate wheel package {ident[0]}')
        identities[ident[0]]=(ident[1],wheel)
    root=Path(__file__).resolve().parent.parent
    locked={}
    for line in (root/'requirements.lock.txt').read_text().splitlines():
        if line.strip() and not line.startswith('#'):
            name,version=line.split('==',1)
            locked[name.lower().replace('_','-')]=version
    locked['wayward-wrangler']=wheel_identity(Path(args.backend_wheel))[1]
    if set(identities)!=set(locked):
        raise SystemExit(f'Wheelhouse differs from lock: missing={sorted(set(locked)-set(identities))}, extra={sorted(set(identities)-set(locked))}')
    for name,(version,_wheel) in identities.items():
        if version!=locked[name]:raise SystemExit(f'Wheel version mismatch for {name}: {version} != {locked[name]}')
    target.mkdir(parents=True)
    (target/'bin').mkdir()
    (target/'wheelhouse').mkdir()
    shutil.copy2(args.rust_binary,target/'bin/waywr')
    shutil.copy2(root/'packaging/install_bundle.py',target/'install.py')
    files={'bin/waywr':sha(target/'bin/waywr'),'install.py':sha(target/'install.py')}
    requirements=[]
    for name,(version,wheel) in sorted(identities.items()):
        dest=target/'wheelhouse'/wheel.name
        shutil.copy2(wheel,dest)
        files[str(dest.relative_to(target))]=sha(dest)
        requirements.append(f'{name}=={version} --hash=sha256:{sha(dest)}')
    (target/'requirements-hashed.txt').write_text('\n'.join(requirements)+'\n')
    files['requirements-hashed.txt']=sha(target/'requirements-hashed.txt')
    libc=platform.libc_ver()
    manifest={'schema_version':1,'release_id':args.release_id,'frontend':'0.4.0',
              'backend':identities['wayward-wrangler'][0], 'protocol_major':1,
              'state_schema_min':1,'state_schema_max':1,
              'platform':{'system':platform.system(),'machine':platform.machine(),
                          'libc':libc[0],'libc_min':libc[1]},
              'python':f'{sys.version_info.major}.{sys.version_info.minor}', 'files':files}
    (target/'manifest.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
    print(json.dumps({'bundle':str(target),'release_id':args.release_id,
                      'manifest_sha256':sha(target/'manifest.json'),'wheels':len(wheels)},indent=2))


if __name__=='__main__':main()
