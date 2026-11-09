#!/usr/bin/python3
"""Fixed-project acceptance guard. Never invokes Claude or grants an allowance."""
import hashlib,json,os,subprocess,sys,tomllib
from pathlib import Path
CONFIG=Path(__file__).with_suffix('.json')
def sha(path):
 with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def main():
 c=json.loads(CONFIG.read_text())
 if c.get('trial_authorized') is not True:raise RuntimeError('Native trial not authorized. Review docs/archive/continuous-coordinator/claude-continuous-live-acceptance.md; no project, service or call started.')
 p=Path(c['project']);r=Path(c['release']);args=sys.argv[1:]
 if args not in ([],['resume'],['init'],['init','.'],['status'],['session','status']):raise RuntimeError('Use init ., launch, resume or status; trial configuration overrides are unavailable')
 if sha(r/'manifest.json')!=c['manifest_sha256']:raise RuntimeError('Reviewed release manifest changed; stop')
 m=json.loads((r/'manifest.json').read_text())
 if m['release_id']!='claude-dialogue-v8':raise RuntimeError('Wrong reviewed release')
 for name,digest in m['files'].items():
  q=r/name
  if not q.is_file() or q.is_symlink() or sha(q)!=digest:raise RuntimeError('Reviewed release file changed: '+name)
 profile=c['profile']
 if sha(Path(profile['binary']))!=c['client_sha256']:raise RuntimeError('Pinned Claude client changed; stop')
 for q in [Path('/etc/claude-code/managed-settings.json'),Path('/etc/claude-code/managed-mcp.json'),Path(profile['auth_config_dir'])/'managed-settings.json']:
  if q.exists():raise RuntimeError('Managed configuration requires review; not bypassed: '+str(q))
 if not (Path(profile['auth_config_dir'])/'.credentials.json').is_file():raise RuntimeError('Existing saved login missing; no call made')
 d=p/'.wayward-wrangler';fresh=not (d/'project.toml').exists();init=bool(args) and args[0]=='init'
 if c['project_key'] is not None and fresh:raise RuntimeError('Retained trial state missing; no reinitialization or allowance renewal')
 if not fresh:
  key=tomllib.loads((d/'project.toml').read_text())['project_key'];binding=json.loads((d/'release.json').read_text())
  if c['project_key']!=key or binding['project_key']!=key or binding['release_id']!=m['release_id'] or binding['manifest_sha256']!=c['manifest_sha256']:raise RuntimeError('Trial identity/release changed; configuration preserved')
  if json.loads((d/'coordinator.json').read_text())!=profile:raise RuntimeError('Trial profile changed; no substitution')
 elif not init:raise RuntimeError('Initialize this new trial first: waywr-live init .')
 env={**os.environ,'HOME':profile['auth_home'],'XDG_STATE_HOME':c['index'],'XDG_RUNTIME_DIR':c['runtime'],'DBUS_SESSION_BUS_ADDRESS':f'unix:path=/run/user/{os.getuid()}/bus'}
 for path in [c['index'],c['runtime']]:Path(path).mkdir(parents=True,exist_ok=True)
 if fresh:p.mkdir(parents=True,exist_ok=True)
 os.chdir(p)
 if init:
  result=subprocess.run([str(r/'bin/waywr'),*args],env=env)
  if result.returncode:raise SystemExit(result.returncode)
  if fresh:
   if (d/'state/state.sqlite').exists():raise RuntimeError('Unexpected runtime state during init')
   (d/'coordinator.json').write_text(json.dumps(profile,indent=2)+'\n');c['project_key']=tomllib.loads((d/'project.toml').read_text())['project_key'];CONFIG.write_text(json.dumps(c,indent=2)+'\n')
  return
 os.execve(str(r/'bin/waywr'),[str(r/'bin/waywr'),*args],env)
if __name__=='__main__':
 try:main()
 except (RuntimeError,OSError,KeyError) as e:print('Continuous live check: '+str(e),file=sys.stderr);sys.exit(2)
