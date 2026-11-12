"""Local release transport for installer checks; no account or network access."""
import hashlib
import json
from pathlib import Path
import sys


def release_transport(directory, assets, version):
    directory, assets = Path(directory), Path(assets)
    directory.mkdir(exist_ok=True)
    archive = f'waywr-{version}-linux-x86_64-gnu.tar.gz'
    metadata = {'tag_name': version, 'draft': False, 'prerelease': False, 'assets': [
        {'name': name, 'digest': 'sha256:' + hashlib.sha256((assets / name).read_bytes()).hexdigest()}
        for name in ['SHA256SUMS', archive]]}
    (assets / 'release.json').write_text(json.dumps(metadata))
    curl = directory / 'curl'
    curl.write_text(f'#!{sys.executable}\n' + '''import json, os, shutil, sys
from pathlib import Path
args = sys.argv[1:]
assert args[0] == '-q', 'Public download must ignore user curl configuration'
assert not any('authorization:' in arg.lower() or arg in ['--user', '-u', '--netrc', '-n', '--insecure', '-k'] for arg in args)
url = next(arg for arg in args if arg.startswith('https://'))
assert '--proto' in args and args[args.index('--proto') + 1] == '=https'
assert '--proto-redir' in args and args[args.index('--proto-redir') + 1] == '=https'
with open(os.environ['FIXTURE_REQUESTS'], 'a') as log:
    log.write(url + '\\n')
base = 'https://api.github.com/repos/example/wayward-wrangler/releases/'
version = os.environ['FIXTURE_VERSION']
if url in [base + 'latest', base + 'tags/' + version]:
    name = 'release.json'
else:
    prefix = 'https://github.com/example/wayward-wrangler/releases/download/' + version + '/'
    assert url.startswith(prefix), url
    name = url[len(prefix):]
    assert name in ['SHA256SUMS', 'waywr-' + version + '-linux-x86_64-gnu.tar.gz']
source = Path(os.environ['FIXTURE_ASSETS']) / name
if not source.is_file():
    print('Fixture: requested release asset unavailable', file=sys.stderr)
    sys.exit(22)
shutil.copyfile(source, args[args.index('--output') + 1])
''')
    curl.chmod(0o755)
    gh = directory / 'gh'
    gh.write_text('''#!/bin/sh
printf '%s\\n' 'Unexpected authenticated GitHub CLI call' >&2
exit 97
''')
    gh.chmod(0o755)
    return {'FIXTURE_ASSETS': str(assets), 'FIXTURE_VERSION': version,
            'FIXTURE_REQUESTS': str(directory / 'requests')}
