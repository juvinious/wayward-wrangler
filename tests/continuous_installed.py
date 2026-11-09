"""Installed schema-7 production TUI, offline mock transport, no operator projects."""
import signal,argparse,fcntl,hashlib,json,os,pty,select,socket,sqlite3,struct,subprocess,termios,time,uuid
from pathlib import Path
from claude_rendering_installed import Screen

def main(root,launcher):
 assert not root.exists();root.mkdir(parents=True);config=json.loads((launcher.parent/'evaluator/launcher.json').read_text());release=Path(config['release']);runtime=Path(config['runtime']);resources=[];logs={};screens={};pid=fd=None;results={}
 def prep(name):
  p=root/name;p.mkdir();r=subprocess.run([str(launcher),'init','.'],cwd=p,capture_output=True,text=True,timeout=20);assert r.returncode==0,r.stderr
  d=p/'.wayward-wrangler';key=json.loads((d/'state/project-identity.json').read_text())['project_key'];item={'project':p,'key':key,'endpoint':runtime/'waywr'/f'{key}.sock','state':d/'state'};resources.append(item)
  assert not item['endpoint'].exists();assert json.loads((d/'coordinator.json').read_text())['mode']=='mock-cli';assert json.loads((d/'state/state-contract.json').read_text())['state_schema']==7
  before={str(v):v.read_bytes() for v in d.glob('*') if v.is_file()};r=subprocess.run([str(launcher),'init','.'],cwd=p,capture_output=True,text=True,timeout=20);assert r.returncode==0,r.stderr;assert all(Path(v).read_bytes()==data for v,data in before.items())
  return item
 def request(item,kind,body):
  with socket.socket(socket.AF_UNIX) as peer:
   peer.settimeout(2);peer.connect(str(item['endpoint']));stream=peer.makefile('rb')
   def call(k,b):peer.sendall((json.dumps({'v':1,'kind':k,'request_id':uuid.uuid4().hex,'body':b})+'\n').encode());return json.loads(stream.readline())['body']
   assert call('hello',{'protocol':{'major':1,'minor':5},'project_key':item['key'],'release_id':release.name,'channel':'command'})['accepted'];return call(kind,body)
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
  wait(name,'normal startup',lambda:screen(name,'MOCK CLI STREAM') and snap(item)['conversation']['continuous'])
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
  press(name,b'/exit\r');wait(name,'visible exit action',lambda:screen(name,'Exit this project session?'));press(name,b'\t');wait(name,'exit action selected',lambda:screen(name,'> [Exit session]'));press(name,b'\r');end=time.monotonic()+15
  while time.monotonic()<end:
   drain(name);found,status=os.waitpid(pid,os.WNOHANG)
   if found:
    assert os.waitstatus_to_exitcode(status)==0;assert termios.tcgetattr(fd)[3]&termios.ICANON;assert b'\x1b[?1049l' in logs[name];os.close(fd);pid=fd=None;return
  (root/(name+'-exit-failure-screen.txt')).write_bytes(screens[name].text());raise AssertionError('exit/terminal restoration')
 def profile_allowance(item,name):
  press(name,b'\x1bOR');wait(name,'visible settings action',lambda:screen(name,'Authorize six'));press(name,b'\t');press(name,b'\r');wait(name,'saved mock allowance',lambda:snap(item)['conversation']['chat']['authorized']);time.sleep(.8);drain(name)
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
  a=prep('project-a');launch(a,'a-first');profile_allowance(a,'a-first')
  press('a-first',b'Discuss fixed-sample weather.\r');wait('a-first','first running',lambda:snap(a)['conversation']['turns'][0]['status']=='reserved')
  first=snap(a)['conversation']['turns'][0]['turn_id'];packet=a['state']/f'session/conversation/{first}/packet.json';before=packet.read_bytes()
  wait('a-first','discoverable interruption',lambda:screen('a-first','Interrupt coordinator'))
  press('a-first',b'Later question while the call runs.\r')
  wait('a-first','both completed',lambda:len(snap(a)['conversation']['turns'])==2 and all(t['status']=='completed' for t in snap(a)['conversation']['turns']))
  assert packet.read_bytes()==before
  second=snap(a)['conversation']['turns'][0]['turn_id'];context=json.loads((a['state']/f'session/conversation/{second}/packet.json').read_text())['selected_context'];assert any(m['role']=='assistant' and 'Discuss fixed-sample weather' in m['text'] for m in context['messages'])
  assert not snap(a)['tasks'];assert snap(a)['conversation']['questions'][0]['status']=='open'
  message(a,'a-first','Celsius',3);assert snap(a)['conversation']['questions'][0]['status']=='open'
  wait('a-first','formal confirmation action',lambda:screen('a-first','Record suggested formal answer'));press('a-first',b'\r');wait('a-first','confirmation panel',lambda:screen('a-first','Confirm formal question decision'));assert snap(a)['conversation']['questions'][0]['status']=='open'
  press('a-first',b'\t\t\r');wait('a-first','formal saved answer',lambda:snap(a)['conversation']['questions'][0]['status']=='answered');assert snap(a)['conversation']['budgets']['conversation']['charged']==3
  message(a,'a-first','Please propose the scope.',4);task=snap(a)['tasks'][0]['task_id'];wait('a-first','review discoverable',lambda:screen('a-first','[Review scope]'));press('a-first',b'\r');wait('a-first','optional purpose step',lambda:screen('a-first','Step 1 of 4'))
  # Local edit through the accepted textarea, no inference call.
  press('a-first',b'\t\r');wait('a-first','local editor',lambda:screen('a-first','Local scope edit'));press('a-first',b'\x1b[200~Local sample-data disclaimer.\x1b[201~');press('a-first',b'\t\r');wait('a-first','saved local edit',lambda:scope(a,task)['base']['revision']==2);assert snap(a)['conversation']['budgets']['conversation']['charged']==4
  authorize_scope(a,'a-first',task,2)
  message(a,'a-first','Looks good. Ordinary discussion only.',5);assert scope(a,task)['authorized'];assert len(scope(a,task)['decision_history'])==1
  press('a-first',b'reconnect draft');detach('a-first');launch(a,'a-resume',True,width=80,height=30);wait('a-resume','restored draft and permission',lambda:screen('a-resume','reconnect draft') and scope(a,task)['authorized']);press('a-resume',b'\x01\x7f')
  message(a,'a-resume','Revise the constraints.',6);assert scope(a,task)['base']['revision']==3 and not scope(a,task)['authorized'];assert snap(a)['conversation']['chat']['status']=='exhausted'
  wait('a-resume','local scope review survives exhaustion',lambda:screen('a-resume','[Review scope]'));press('a-resume',b'\r');authorize_scope(a,'a-resume',task,3)
  # Replay an exact recorded command without another decision or inference.
  with sqlite3.connect('file:'+str(a['state']/'state.sqlite')+'?mode=ro',uri=True) as db:body,outcome=db.execute("SELECT body,outcome FROM session_commands WHERE json_extract(body,'$.action')='authorize_scope' LIMIT 1").fetchone()
  assert request(a,'command',json.loads(body))==json.loads(outcome);assert len(scope(a,task)['decision_history'])==2
  exit_session('a-resume');results['ordinary']={'calls':6,'causal_queued_reply':True,'frozen_packet':True,'formal_confirmation_only':True,'local_edit_no_call':True,'discussion_preserves_authorization':True,'material_revision_requires_permission':True,'detach_resume_draft_authority':True,'exhausted_local_action':True,'command_replay_once':True,'terminal_restoration':True,'widths':[120,80]}
  b=prep('project-b');launch(b,'b');profile_allowance(b,'b');message(b,'b','fixture-proposal',1);task_b=snap(b)['tasks'][0]['task_id'];base=scope(b,task_b)['base']
  press('b',b'fixture-format\r');wait('b','format call running',lambda:snap(b)['conversation']['turns'][0]['status']=='reserved');bad=snap(b)['conversation']['turns'][0]['turn_id'];press('b',b'Older submitted constraint\r')
  wait('b','typed format failure and held queue',lambda:snap(b)['conversation']['chat']['status']=='recoverable' and any(t['status']=='held' for t in snap(b)['conversation']['turns']));assert scope(b,task_b)['base']==base
  old=next(t['turn_id'] for t in snap(b)['conversation']['turns'] if t['status']=='held');wait('b','failure explicitly no save',lambda:screen('b','No scope change saved'));press('b',b'fixture-long\r');wait('b','new turn runs without releasing held',lambda:snap(b)['conversation']['turns'][0]['status']=='reserved');assert next(t for t in snap(b)['conversation']['turns'] if t['turn_id']==old)['status']=='held'
  # A withdrawal while the current call is frozen spends no call and preserves its message.
  press('b',b'Withdraw this queued input\r');wait('b','queued withdrawal target',lambda:any(t['status']=='queued' for t in snap(b)['conversation']['turns']));withdraw=next(t['turn_id'] for t in snap(b)['conversation']['turns'] if t['status']=='queued')
  press('b',b'\x1b[18~');wait('b','queue while running with accepted input visible',lambda:screen('b','Conversation queue') and screen('b','Queued · Withdraw this queued input'));press('b',b'\x1b[C');press('b',b'\x1b[C');wait('b','queued row selected',lambda:screen('b','› Queued · Withdraw this queued input'));press('b',b'\t');wait('b','withdraw action selected',lambda:screen('b','› [Withdraw message]'));press('b',b'\r');wait('b','durable withdrawal without call',lambda:next(t for t in snap(b)['conversation']['turns'] if t['turn_id']==withdraw)['status']=='withdrawn');assert snap(b)['conversation']['budgets']['conversation']['charged']==3;press('b',b'\x1b')
  begin_status=time.monotonic();snap(b);status_elapsed=time.monotonic()-begin_status;assert status_elapsed<1
  press('b',b'new unsent draft');wait('b','draft visible',lambda:screen('b','Draft · not sent'));press('b',b'\t')
  # Composer Tab focuses Send first, then Activity; empty/nonempty routing is tested separately.
  for _ in range(6):
   if screen('b','› [Interrupt coordinator]'):break
   press('b',b'\t')
  assert screen('b','› [Interrupt coordinator]')
  press('b',b'\r');wait('b','explicit interrupt panel',lambda:screen('b','Interrupt coordinator call?'));assert json.loads((b['state']/'session/draft.json').read_text())['draft']['text']=='new unsent draft';begin=time.monotonic();press('b',b'\t\r')
  wait('b','interruption command durably accepted',lambda:request(b,'events_read',{'after_seq':0,'limit':64}).get('events') and any(e['type']=='conversation.cancel_requested' for e in request(b,'events_read',{'after_seq':0,'limit':64})['events']),limit=1);control_elapsed=time.monotonic()-begin;assert control_elapsed<1
  wait('b','settled interruption',lambda:snap(b)['conversation']['chat']['status']=='recoverable');elapsed=time.monotonic()-begin;assert elapsed<2
  assert next(t for t in snap(b)['conversation']['turns'] if t['turn_id']==old)['status']=='held';wait('b','visible settled interruption',lambda:screen('b','• Interrupted') and screen('b','new unsent draft'));press('b',b'\x1b[18~');wait('b','held queue panel',lambda:screen('b','Conversation queue') and screen('b','Continue selected held message') and screen('b','Recovery generation 2'))
  press('b',b'\t\t\r');wait('b','selected held message explicitly released',lambda:next(t for t in snap(b)['conversation']['turns'] if t['turn_id']==old)['status']=='completed');assert snap(b)['conversation']['budgets']['conversation']['charged']==4
  press('b',b'\x1b');wait('b','queue closed',lambda:not screen('b','Conversation queue'));press('b',b'\x1b');press('b',b'\x01\x7f');message(b,'b','Another ordinary discussion.',5)
  assert a['key'] not in json.dumps(json.loads(next(b['state'].glob('session/conversation/*/packet.json')).read_text()))
  exit_session('b');results['recovery']={'format_no_save':True,'new_turn_retains_old_held':True,'interrupt_fenced_and_settled':True,'interrupt_settled_seconds':elapsed,'interrupt_control_seconds':control_elapsed,'held_release_ui':True,'composer_preserved':True,'withdraw_ui_no_charge':True,'status_seconds':status_elapsed,'calls':5,'two_project_isolation':True}
  for item in resources:
   receipts=list(item['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'));assert len(receipts)==results['ordinary' if item==a else 'recovery']['calls'];assert all(json.loads(p.read_text())['schema_version']==3 for p in receipts)
   with sqlite3.connect('file:'+str(item['state']/'state.sqlite')+'?mode=ro',uri=True) as db:assert db.execute('SELECT count(*) FROM app_runs').fetchone()[0]==0;assert db.execute('SELECT count(*) FROM session_admissions').fetchone()[0]==0
  results['workers']=results['consultations']=results['execution_admissions']=0
 finally:
  if pid:
   # Exact fixture frontend child before strace; a stopped tracer may otherwise wait forever.
   children=[]
   for proc in Path('/proc').iterdir():
    if not proc.name.isdigit():continue
    try:
     stat=proc.joinpath('stat').read_text().split(') ')[1].split();cmd=proc.joinpath('cmdline').read_bytes().replace(b'\0',b' ').decode()
     if int(stat[1])==pid and str(release/'bin/waywr') in cmd:children.append((int(proc.name),stat[19]))
    except (OSError,ValueError):pass
   for child,start in children:
    proc=Path('/proc')/str(child)
    if proc.exists() and proc.joinpath('stat').read_text().split(') ')[1].split()[19]==start:os.kill(child,signal.SIGTERM)
   os.waitpid(pid,0);os.close(fd)
  cleanup=[]
  for item in resources:
   r=subprocess.run([str(launcher),'session','stop'],cwd=item['project'],capture_output=True,timeout=20);cleanup.append({'project_key':item['key'],'stop_returncode':r.returncode})
   unit=f'waywr-{item["key"]}.service';props=subprocess.run(['systemctl','--user','show',unit,'--property=ExecStart,ControlGroup,MainPID,LoadState'],capture_output=True,text=True).stdout
   if 'LoadState=loaded' in props:
    assert str(release/'bin/waywr-service') in props;subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15)
   for intent in item['state'].glob('session/coordinator/*/call-v1/attempts/*/intent.json'):
    unit=json.loads(intent.read_text())['unit'];props=subprocess.run(['systemctl','--user','show',unit,'--property=ControlGroup,MainPID,ExecStart'],capture_output=True,text=True).stdout;group=next((s.partition('=')[2] for s in props.splitlines() if s.startswith('ControlGroup=')),'');procs=Path('/sys/fs/cgroup'+group)/'cgroup.procs' if group else None
    if procs and procs.exists() and procs.read_text().strip():cleanup.append({'unit':unit,'retained_active':True});continue
    assert str(release/'bin/waywr-runner') in props or 'MainPID=0' in props;subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15);cleanup.append({'unit':unit,'stopped_empty':True})
  for name,data in logs.items():(root/(name+'.pty')).write_bytes(data);(root/(name+'-screen.txt')).write_bytes(screens[name].text())
  (root/'summary.json').write_text(json.dumps(results,indent=2)+'\n');(root/'cleanup.json').write_text(json.dumps(cleanup,indent=2)+'\n')
 print(json.dumps(results))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--launcher',type=Path,required=True);a=p.parse_args();main(a.root.resolve(),a.launcher.resolve())
