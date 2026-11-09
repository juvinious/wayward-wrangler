"""Offline paired release installer. No project state or service is changed."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def version_tuple(text):return tuple(int(piece) for piece in text.split('.')[:2])


def validate(bundle,manifest):
    if manifest.get('schema_version')!=1 or manifest.get('protocol_major')!=1:
        raise ValueError('Unsupported bundle manifest/protocol')
    machine=platform.machine();libc=platform.libc_ver()
    if manifest['platform']['system']!='Linux' or manifest['platform']['machine']!=machine:
        raise ValueError('Bundle targets another operating system or architecture')
    if libc[0]!=manifest['platform']['libc'] or version_tuple(libc[1])<version_tuple(manifest['platform']['libc_min']):
        raise ValueError('Host C library is older than the recorded bundle build baseline')
    if f'{sys.version_info.major}.{sys.version_info.minor}'!=manifest['python']:
        raise ValueError(f'Use Python {manifest["python"]} with venv and ensurepip to install this bundle')
    for name,expected in manifest['files'].items():
        path=(bundle/name).resolve(strict=True)
        if not path.is_relative_to(bundle) or sha(path)!=expected:
            raise ValueError(f'Bundle integrity mismatch: {name}')


def select_default(prefix,release_id):
    current=prefix/'current'
    aliases=[prefix/'bin'/name for name in ('waywr','wayward-wrangler','awp')]
    expected='../current/bin/waywr'
    for path in aliases:
        if path.exists() or path.is_symlink():
            if not path.is_symlink() or os.readlink(path)!=expected:
                raise ValueError(f'Unrelated executable or alias at {path}; default unchanged')
    if current.exists() or current.is_symlink():
        if not current.is_symlink() or not os.readlink(current).startswith('releases/'):
            raise ValueError('Default release path is not an owned link')
    (prefix/'bin').mkdir(parents=True,exist_ok=True)
    temporary=prefix/'.current-next'
    if temporary.exists() or temporary.is_symlink():raise ValueError('Pending default update exists; inspect it')
    temporary.symlink_to(f'releases/{release_id}')
    temporary.replace(current)
    for path in aliases:
        if not path.is_symlink():path.symlink_to(expected)


def main(argv=None):
    p=argparse.ArgumentParser()
    p.add_argument('--bundle',type=Path,required=True)
    p.add_argument('--prefix',type=Path,required=True)
    p.add_argument('--select-default',action='store_true')
    args=p.parse_args(argv)
    bundle=args.bundle.expanduser().resolve(strict=True)
    manifest=json.loads((bundle/'manifest.json').read_text())
    validate(bundle,manifest)
    release_id=manifest['release_id']
    if not release_id or '/' in release_id or '..' in release_id:
        raise ValueError('Invalid release ID')
    prefix=args.prefix.expanduser().resolve()
    destination=prefix/'releases'/release_id
    if destination.exists():
        if sha(destination/'manifest.json')!=sha(bundle/'manifest.json'):
            raise ValueError('Installed release ID has different contents')
        validate(destination,manifest)
        if not (destination/'venv/bin/python').is_file():
            raise ValueError('Installed release is incomplete; inspect it before repair')
        subprocess.run([str(destination/'venv/bin/python'),'-I','-m','pip','check'],check=True)
        subprocess.run([str(destination/'bin/waywr'),'--backend-info'],check=True,
                       stdout=subprocess.DEVNULL)
        print(f'Validated existing release: {destination}')
    else:
        destination.mkdir(parents=True)
        for name in manifest['files']:
            target=destination/name
            target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(bundle/name,target)
        shutil.copy2(bundle/'manifest.json',destination/'manifest.json')
        subprocess.run([sys.executable,'-m','venv',str(destination/'venv')],check=True)
        pip=destination/'venv/bin/python'
        subprocess.run([str(pip),'-I','-m','pip','install','--no-index','--only-binary=:all:',
                        '--require-hashes','--find-links',str(destination/'wheelhouse'),
                        '-r',str(destination/'requirements-hashed.txt')],check=True)
        subprocess.run([str(pip),'-I','-m','pip','check'],check=True)
        subprocess.run([str(destination/'bin/waywr'),'--backend-info'],check=True,
                       stdout=subprocess.DEVNULL)
    if args.select_default:select_default(prefix,release_id)
    print(json.dumps({'release':str(destination),'default_selected':args.select_default,
                      'aliases':[str(prefix/'bin'/name) for name in ('waywr','wayward-wrangler','awp')]}))


if __name__=='__main__':main()
