"""Real installed production TUI/automatic service, scripted mock CLI only."""
import argparse,fcntl,hashlib,json,os,pty,select,socket,sqlite3,stat,struct,subprocess,termios,time,uuid
from contextlib import contextmanager
from pathlib import Path
from claude_rendering_installed import Screen

@contextmanager
def unavailable_rust_source(evidence):
 # Reversible test-only read denial of current Rust sources/assets; no ignored
 # installations, operator state, Python legacy source or shared resources change.
 repo=Path(__file__).resolve().parent.parent
 directories=[repo/'backend',repo/'frontend']
 before={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for d in directories for p in d.rglob('*') if p.is_file()}
 modes={d:stat.S_IMODE(d.stat().st_mode) for d in directories}
 proof={'uid':os.getuid(),'directories':{str(d):mode for d,mode in modes.items()},'source_files':len(before),'restored':False}
 assert os.getuid()!=0
 try:
  for d in directories:d.chmod(0)
  for d in directories:
   try:(d/'Cargo.toml').read_bytes()
   except PermissionError:pass
   else:raise AssertionError('Rust source is still readable')
  proof['read_denial']='EACCES for both Rust source directories during installed init/start/resume/host execution'
  yield
 finally:
  for d,mode in modes.items():d.chmod(mode)
  after={p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in before}
  proof['restored']=before==after and all(stat.S_IMODE(d.stat().st_mode)==mode for d,mode in modes.items())
  evidence.parent.mkdir(parents=True,exist_ok=True);evidence.write_text(json.dumps(proof,indent=2)+'\n')
  assert proof['restored'],'Source permissions/bytes must be restored'

def main(root,launcher,review_only=False):
 assert not root.exists();root.mkdir(parents=True);config=json.loads((launcher.parent/'evaluator/launcher.json').read_text());release=Path(config['release']);runtime=Path(config['runtime']);resources=[];logs={};screens={};pid=fd=None;results={}
 def prep(name):
  p=root/name;p.mkdir();r=subprocess.run([str(launcher),'init','.'],cwd=p,capture_output=True,text=True,timeout=20);assert r.returncode==0,r.stderr
  d=p/'.wayward-wrangler';key=json.loads((d/'state/project-identity.json').read_text())['project_key'];item={'project':p,'key':key,'endpoint':runtime/'waywr'/f'{key}.sock','state':d/'state'};resources.append(item)
  assert not item['endpoint'].exists();assert json.loads((d/'coordinator.json').read_text())['mode']=='mock-cli';assert json.loads((d/'state/state-contract.json').read_text())['state_schema']==6
  before={str(v):v.read_bytes() for v in d.glob('*') if v.is_file()};r=subprocess.run([str(launcher),'init','.'],cwd=p,capture_output=True,text=True,timeout=20);assert r.returncode==0,r.stderr;assert all(Path(v).read_bytes()==data for v,data in before.items())
  return item
 def request(item,kind,body):
  with socket.socket(socket.AF_UNIX) as peer:
   peer.settimeout(2);peer.connect(str(item['endpoint']));stream=peer.makefile('rb')
   def call(k,b):peer.sendall((json.dumps({'v':1,'kind':k,'request_id':uuid.uuid4().hex,'body':b})+'\n').encode());return json.loads(stream.readline())['body']
   assert call('hello',{'protocol':{'major':1,'minor':4},'project_key':item['key'],'release_id':release.name,'channel':'command'})['accepted'];return call(kind,body)
 def snap(item):return request(item,'snapshot',{})
 def drain(name):
  if select.select([fd],[],[],.03)[0]:
   try:data=os.read(fd,65536);logs[name].extend(data);screens[name].feed(data)
   except OSError:pass
 def wait(name,label,fn,limit=30):
  until=time.monotonic()+limit
  while time.monotonic()<until:
   drain(name)
   try:
    v=fn()
    if v:return v
   except (OSError,ValueError,KeyError,IndexError):pass
   time.sleep(.03)
  (root/f'{name}-failure-screen.txt').write_bytes(screens[name].text());raise AssertionError(label)
 def screen(name,text):return text.encode() in screens[name].text()
 def press(name,data):
  with (root/(name+'-input.jsonl')).open('a') as out:out.write(json.dumps({'input_hex':data.hex(),'screen_before':screens[name].text().decode()})+'\n')
  os.write(fd,data);time.sleep(.09);drain(name)
 def launch(item,name,resume=False,width=120,height=36):
  nonlocal pid,fd
  pid,fd=pty.fork()
  if pid==0:
   fcntl.ioctl(0,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0));os.chdir(item['project']);env={**os.environ,'TERM':'xterm-256color'}
   os.execve('/usr/bin/strace',['strace','-f','-e','trace=file','-o',str(root/(name+'-files.trace')),str(launcher)]+(['resume'] if resume else []),env)
  logs[name]=bytearray();screens[name]=Screen(width,height)
  wait(name,'normal startup',lambda:screen(name,'MOCK CLI STREAM') and snap(item)['conversation']['scope_enabled'])
 def detach(name):
  nonlocal pid,fd
  press(name,b'\x11');press(name,b'\t');press(name,b'\r');end=time.monotonic()+10
  while time.monotonic()<end:
   drain(name);found,status=os.waitpid(pid,os.WNOHANG)
   if found:
    assert os.waitstatus_to_exitcode(status)==0;assert termios.tcgetattr(fd)[3]&termios.ICANON;assert b'\x1b[?1049l' in logs[name];os.close(fd);pid=fd=None;return
  raise AssertionError('terminal detach/restore')
 def exit_session(name):
  nonlocal pid,fd
  press(name,b'/exit\r');wait(name,'visible exit action',lambda:screen(name,'Exit this project session?'));press(name,b'\t');press(name,b'\r');end=time.monotonic()+15
  while time.monotonic()<end:
   drain(name);found,status=os.waitpid(pid,os.WNOHANG)
   if found:
    assert os.waitstatus_to_exitcode(status)==0;assert termios.tcgetattr(fd)[3]&termios.ICANON;assert b'\x1b[?1049l' in logs[name];os.close(fd);pid=fd=None;return
  raise AssertionError('exit/terminal restoration')
 def profile_allowance(item,name):
  press(name,b'\r');wait(name,'visible settings action',lambda:screen(name,'Authorize six'));press(name,b'\t');press(name,b'\r');wait(name,'saved mock allowance',lambda:snap(item)['conversation']['chat']['authorized']);time.sleep(.8);drain(name)
 def message(item,name,text,charge):
  press(name,text.encode()+b'\r');wait(name,'completed mock response',lambda:snap(item)['conversation']['budgets']['conversation']['charged']==charge and snap(item)['conversation']['turns'][0]['status']=='completed')
 def scope(item,task):return request(item,'task.get',{'task_id':task})['scope']
 def authorize_scope(item,name,task,revision):
  for step in range(1,5):
   wait(name,f'visible review step {step}',lambda:screen(name,f'Step {step} of 4') and any('│' in line and f' · revision {revision}' in line for line in screens[name].text().decode().splitlines()))
   assert not scope(item,task)['authorized'],'Opening or advancing review must not authorize'
   assert screen(name,'[Authorize scope]')==(step==4),'Final action only in final step'
   (root/f'{name}-scope-step-{step}-screen.txt').write_bytes(screens[name].text())
   press(name,b'\r')
  wait(name,'scope-only authorized',lambda:snap(item)['tasks'][0]['status']=='scope-authorized');assert scope(item,task)['authorized'];wait(name,'honest scope state',lambda:screen(name,'Scope authorized'))
 try:
  a=prep('project-a');b=None if review_only else prep('project-b');launch(a,'a-first');profile_allowance(a,'a-first')
  press('a-first',b'Discuss a fixed-sample weather page.\r');wait('a-first','stream while typing',lambda:snap(a)['conversation'].get('provisional'))
  press('a-first',b'unsent draft while coordinator works');wait('a-first','first completion',lambda:snap(a)['conversation']['turns'][0]['status']=='completed');assert not snap(a)['tasks'];assert screen('a-first','unsent draft')
  press('a-first',b'\x01\x7f');message(a,'a-first','Clarify the units before proposing scope.',2)
  wait('a-first','visible question action',lambda:screen('a-first','Answer question'));press('a-first',b'\r');wait('a-first','question panel',lambda:screen('a-first','Include the toggle'));press('a-first',b'\x1b[B\x1b[B');press('a-first',b'\t');press('a-first',b'\r')
  wait('a-first','saved answer no call',lambda:snap(a)['conversation']['questions'][0]['status']=='answered');assert snap(a)['conversation']['budgets']['conversation']['charged']==2
  wait('a-first','separate continue action',lambda:screen('a-first','Continue with coordinator'));press('a-first',b'\r');wait('a-first','saved-answer panel',lambda:screen('a-first','Answer saved'));assert not screen('a-first','Save answer');press('a-first',b'\t');press('a-first',b'\r')
  wait('a-first','durable proposal',lambda:len(snap(a)['tasks'])==1 and snap(a)['conversation']['turns'][0]['status']=='completed');task=snap(a)['tasks'][0]['task_id'];assert snap(a)['tasks'][0]['run_id'] is None
  wait('a-first','review scope visible',lambda:screen('a-first','[Review scope]'));press('a-first',b'\r');wait('a-first','scope panel',lambda:screen('a-first','Edit scope'));assert not scope(a,task)['authorized']
  press('a-first',b'\t');press('a-first',b'\r');wait('a-first','existing textarea editor',lambda:screen('a-first','Local scope edit'))
  press('a-first',b'\x01');press('a-first',b'\x1b[200~Fixed sample data with a clear local-only disclaimer\x1b[201~');press('a-first',b'\t');press('a-first',b'\r');wait('a-first','local revision with no call',lambda:scope(a,task)['base']['revision']==2);assert snap(a)['conversation']['budgets']['conversation']['charged']==3
  authorize_scope(a,'a-first',task,2);assert len(scope(a,task)['decision_history'])==1
  press('a-first',b'reconnect draft');detach('a-first')
  if review_only:
   launch(a,'a-review-resume',True,width=80,height=24);wait('a-review-resume','authorized decision and draft restored',lambda:screen('a-review-resume','reconnect draft') and scope(a,task)['authorized'])
   press('a-review-resume',b'\x01\x7f');press('a-review-resume',b'\r');wait('a-review-resume','authorized scope is read-only',lambda:screen('a-review-resume','View authorized scope') and screen('a-review-resume','[Edit scope]'));assert not screen('a-review-resume','[Authorize scope]')
   press('a-review-resume',b'\x1b');wait('a-review-resume','modal closed',lambda:not screen('a-review-resume','[Edit scope]'))
   with sqlite3.connect('file:'+str(a['state']/'state.sqlite')+'?mode=ro',uri=True) as c:
    body,outcome=c.execute("SELECT body,outcome FROM session_commands WHERE json_extract(body,'$.action')='authorize_scope' LIMIT 1").fetchone();assert c.execute('SELECT count(*) FROM app_runs').fetchone()[0]==0;assert c.execute('SELECT count(*) FROM session_admissions').fetchone()[0]==0
   assert request(a,'command',json.loads(body))==json.loads(outcome);assert len(scope(a,task)['decision_history'])==1
   receipts=list(a['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'));turns=snap(a)['conversation']['turns'];assert len(receipts)==len(turns)==3
   assert all(json.loads(p.read_text())['status']=='completed' for p in receipts)
   assert snap(a)['conversation']['budgets']['conversation']['charged']==3
   results={'sequential_four_step_review':True,'no_early_authority':True,'local_edit_no_call':True,'exact_revision':2,'scope_decisions':1,'decision_replay_no_duplicate':True,'calls':3,'receipts':3,'workers':0,'consultations':0,'admissions':0,'installed_widths':[120,80],'detach_resume_authority_draft':True,'terminal_restoration':True}
   exit_session('a-review-resume');results['exit_terminal_restoration']=True
   return
  # Project B operates while A is detached/authorized, under its own project service.
  launch(b,'b',width=80,height=24);profile_allowance(b,'b');message(b,'b','Independent project discussion.',1);assert not snap(b)['tasks'];detach('b')
  launch(a,'a-second',True);wait('a-second','draft/context restoration',lambda:screen('a-second','reconnect draft') and scope(a,task)['authorized']);press('a-second',b'\x01\x7f')
  message(a,'a-second','Revise the constraints with an explicit sample-data notice.',4);wait('a-second','material revision invalidates authority',lambda:scope(a,task)['base']['revision']==3 and not scope(a,task)['authorized'])
  message(a,'a-second','Looks good.',5);assert not scope(a,task)['authorized'];assert len(scope(a,task)['decision_history'])==1
  wait('a-second','updated review action',lambda:screen('a-second','[Review scope]'));press('a-second',b'\r');authorize_scope(a,'a-second',task,3)
  message(a,'a-second','Add another material constraint to the proposal.',6);assert scope(a,task)['base']['revision']==4;assert not scope(a,task)['authorized'];assert snap(a)['conversation']['chat']['status']=='exhausted'
  wait('a-second','exhausted scope review remains available',lambda:screen('a-second','[Review scope]'));press('a-second',b'\r');authorize_scope(a,'a-second',task,4);assert snap(a)['conversation']['budgets']['conversation']['charged']==6
  # Durable command replay and database busy refusal do not create decisions/calls.
  db=a['state']/'state.sqlite'
  with sqlite3.connect('file:'+str(db)+'?mode=ro',uri=True) as c:body,outcome=c.execute("SELECT body,outcome FROM session_commands WHERE json_extract(body,'$.action')='authorize_scope' LIMIT 1").fetchone()
  before=len(scope(a,task)['decision_history']);assert request(a,'command',json.loads(body))==json.loads(outcome);assert len(scope(a,task)['decision_history'])==before
  current=scope(a,task);command={'command_id':'busy-before-acceptance','action':'save_proposal_revision','payload':{'base':current['base'],'decision_revision':current['decision_revision'],'authorization_command_id':current['authorization_command_id'],'proposal':current['record']['proposal'],'rationale':'Busy fixture'}}
  with sqlite3.connect(db,timeout=.1) as locked:
   locked.execute('BEGIN IMMEDIATE');begin=time.monotonic();busy=request(a,'command',command);latency=time.monotonic()-begin;assert busy['code']=='busy',busy;assert latency<.5;locked.rollback()
  assert request(a,'command_get',{'command_id':command['command_id']})['status']=='not_found'
  latencies=[]
  for _ in range(4):begin=time.monotonic();snap(a);latencies.append(time.monotonic()-begin)
  assert max(latencies)<1
  packets=list(a['state'].glob('session/coordinator/*/call-v1/packet.json'));receipts=list(a['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'));assert len(packets)==len(receipts)==6;assert all(json.loads(p.read_text())['status']=='completed' for p in receipts)
  assert all(b['key'] not in p.read_text() for p in packets);assert snap(b)['conversation']['budgets']['conversation']['charged']==1
  with sqlite3.connect('file:'+str(db)+'?mode=ro',uri=True) as c:assert c.execute('SELECT count(*) FROM app_runs').fetchone()[0]==0;assert c.execute('SELECT count(*) FROM session_admissions').fetchone()[0]==0
  results={'calls_a':6,'matching_receipts':6,'calls_b':1,'worker_calls':0,'consultation_calls':0,'execution_admissions':0,'visible_scope_answer_edit_authorize':True,'local_action_at_exhaustion':True,'material_invalidation':True,'detached_resume_draft':True,'command_replay':True,'sqlite_busy_seconds':latency,'status_seconds_max':max(latencies),'terminal_restoration':True,'two_project_isolation':True}
  detach('a-second')
  # Resume immediately after each exit: readiness reads may race old shutdown,
  # but no new command/call is created for an uncertain mutation.
  for cycle in range(3):
   name=f'a-exit-{cycle}';launch(a,name,True);wait(name,'scope decision retained after exit',lambda:scope(a,task)['authorized']);assert snap(a)['conversation']['budgets']['conversation']['charged']==6
   assert len(scope(a,task)['decision_history'])==3;exit_session(name)
  checkpoints=list((a['state']/'session/checkpoints').glob('*.json'));assert any(task in p.read_text() and 'scope_checkpoint' in p.read_text() for p in checkpoints)
  assert len(list(a['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json')))==6
  results['exit_resume_no_new_calls']=True;results['scope_checkpoint_refs']=True
 finally:
  # Preserve raw evidence even when cleanup itself encounters an ownership issue.
  for name,data in logs.items():(root/(name+'.pty')).write_bytes(data);(root/(name+'-screen.txt')).write_bytes(screens[name].text())
  if pid:
   for item in resources:
    claim=item['state']/'session/ui-current.json'
    if not claim.exists():continue
    identity=json.loads(claim.read_text())['process'];proc=Path(f'/proc/{identity["pid"]}')
    if proc.exists() and proc.joinpath('exe').resolve()==release/'bin/waywr':
     stat=proc.joinpath('stat').read_text().rsplit(')',1)[1].split()
     if stat[19]==str(identity['start']) and int(stat[1])==pid:os.kill(identity['pid'],15)
   os.waitpid(pid,0);os.close(fd)
  cleanup=[]
  for item in resources:
   subprocess.run([str(launcher),'session','stop'],cwd=item['project'],capture_output=True,timeout=20)
   unit=f'waywr-{item["key"]}.service';load=subprocess.run(['systemctl','--user','show',unit,'--property=LoadState','--value'],capture_output=True,text=True).stdout.strip()
   if load=='loaded':subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15)
   for intent in item['state'].glob('session/coordinator/*/call-v1/attempts/*/intent.json'):
    v=json.loads(intent.read_text());unit=v['unit'];props=subprocess.run(['systemctl','--user','show',unit,'--property=ControlGroup,MainPID,ExecStart'],capture_output=True,text=True).stdout;group=next((s.split('=',1)[1] for s in props.splitlines() if s.startswith('ControlGroup=')),'');procs=Path('/sys/fs/cgroup'+group)/'cgroup.procs' if group else None
    if procs and procs.exists() and procs.read_text().strip():cleanup.append({'unit':unit,'retained':'active cgroup'});continue
    subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15);cleanup.append({'unit':unit,'stopped_empty':True})
   cleanup.append({'project_key':item['key'],'service_stopped':True})
  for name,data in logs.items():(root/(name+'.pty')).write_bytes(data);(root/(name+'-screen.txt')).write_bytes(screens[name].text())
  (root/'summary.json').write_text(json.dumps(results,indent=2)+'\n');(root/'cleanup.json').write_text(json.dumps(cleanup,indent=2)+'\n')
 print(json.dumps(results))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--launcher',type=Path,required=True);p.add_argument('--deny-rust-source',action='store_true');p.add_argument('--scope-review-only',action='store_true');a=p.parse_args()
 if a.deny_rust_source:
  with unavailable_rust_source(a.root.resolve().with_name(a.root.name+'-source-denial.json')):main(a.root.resolve(),a.launcher.resolve(),a.scope_review_only)
 else:main(a.root.resolve(),a.launcher.resolve(),a.scope_review_only)
