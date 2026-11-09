#!/usr/bin/python3
"""Isolated mock evaluator wrapper; this is test tooling, not an application backend."""
import hashlib,json,os,subprocess,sys
from pathlib import Path

def main():
 config=json.loads(Path(__file__).with_suffix('.json').read_text());args=sys.argv[1:]
 root=Path(config['evaluation_root']).resolve();selected=Path.cwd().resolve()
 explicit=None;route=[];i=0
 while i<len(args):
  token=args[i]
  if token=='--project' or token.startswith('--project='):
   if explicit is not None:raise RuntimeError('Select the evaluator project once')
   if token=='--project':
    i+=1
    if i==len(args) or args[i].startswith('-'):raise RuntimeError('--project requires a directory')
    value=args[i]
   else:value=token.partition('=')[2]
   if not value:raise RuntimeError('--project requires a directory')
   explicit=Path(value).resolve();selected=explicit
  elif token.startswith('-'):raise RuntimeError('Evaluator configuration overrides are unavailable; use --project only')
  else:route.append(token)
  i+=1
 is_init=bool(route) and route[0]=='init'
 if is_init:
  if len(route)>2:raise RuntimeError('Use init [DIRECTORY]')
  named=Path(route[1]).resolve() if len(route)==2 else selected
  if explicit and explicit!=named:raise RuntimeError('init directory conflicts with --project')
  selected=named
 elif route not in ([],['resume'],['status'],['session','status'],['session','stop']):
  raise RuntimeError('This isolated evaluator supports init, launch, resume and status/stop only')
 # Normalize supported selection to the release's established spaced syntax.
 args=(["--project",str(explicit)] if explicit is not None else [])+route
 if selected!=root and root not in selected.parents:raise RuntimeError(f'This mock launcher operates only under {root}; operator projects are protected')
 binary=Path(config['release'])/'bin/waywr';fixture=Path(config['fixture']);digest=hashlib.sha256(fixture.read_bytes()).hexdigest()
 if digest!=config['fixture_sha256']:raise RuntimeError('Mock fixture changed; candidate/fixture must be reverified together')
 env={**os.environ,'HOME':config['home'],'XDG_STATE_HOME':config['index'],'XDG_RUNTIME_DIR':config['runtime'],'PATH':'/usr/bin:/bin','DBUS_SESSION_BUS_ADDRESS':f'unix:path=/run/user/{os.getuid()}/bus','HERDR_ENV':'1','HERDR_SOCKET_PATH':str(root/'absent-herdr.sock')}
 for p in [config['home'],config['index'],config['runtime']]:Path(p).mkdir(parents=True,exist_ok=True)
 profile_path=selected/'.wayward-wrangler/coordinator.json';fresh=not (selected/'.wayward-wrangler/project.toml').exists()
 if not fresh:
  p=json.loads(profile_path.read_text());binding=json.loads((selected/'.wayward-wrangler/release.json').read_text())
  if p.get('mode')!='mock-cli' or p.get('profile')!='claude-proposal-v1' or p.get('binary')!=str(fixture) or binding['release_id']!=Path(config['release']).name:raise RuntimeError('Existing project is not bound to this exact mock evaluator; configuration preserved')
 if is_init:
  code=subprocess.call([str(binary),*args],env=env)
  if code:raise SystemExit(code)
  if fresh:
   p=json.loads(profile_path.read_text());p.update(mode='mock-cli',binary=str(fixture),auth_home=config['home'],auth_config_dir=str(Path(config['home'])/'.claude'))
   profile_path.write_text(json.dumps(p,indent=2)+'\n')
  return
 if fresh:raise RuntimeError('Initialize this disposable project first: waywr init .')
 os.execve(str(binary),[str(binary),*args],env)
if __name__=='__main__':
 try:main()
 except (RuntimeError,KeyError,IndexError) as e:print(f'Mock evaluator: {e}',file=sys.stderr);sys.exit(2)
