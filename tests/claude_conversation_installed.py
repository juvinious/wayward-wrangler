"""Normal installed TUI path, mock CLI only; never touches operator projects."""
import argparse,fcntl,json,os,pty,select,signal,socket,sqlite3,struct,subprocess,tempfile,termios,time,uuid
from pathlib import Path

def main(root,release):
 assert not root.exists();root.mkdir(parents=True)
 private=Path(tempfile.mkdtemp(prefix='ww-chat-'))
 env={**os.environ,'HOME':str(private/'home'),'XDG_RUNTIME_DIR':str(private/'runtime'),'XDG_STATE_HOME':str(root/'index'),'TERM':'xterm-256color','PATH':'/usr/bin:/bin','HERDR_ENV':'1','HERDR_SOCKET_PATH':str(private/'absent.sock')}
 for n in ('home','runtime'): (private/n).mkdir()
 exe=release/'bin/waywr';fixture=(Path(__file__).parent/'fixtures/claude-conversation/mock-cli.py').resolve()
 resources=[];logs={};pid=None;fd=None
 def prepare(name):
  p=root/name;p.mkdir();r=subprocess.run([str(exe),'init','.'],cwd=p,env=env,capture_output=True,text=True,timeout=20);assert r.returncode==0,r.stderr
  d=p/'.wayward-wrangler'
  # Empty disposable fixture only, before service/SQLite/messages: explicitly select legacy schema 5.
  assert not (d/'state/state.sqlite').exists()
  config=(d/'project.toml').read_text().replace('[conversation]\nschema_version = 3','[conversation]\nschema_version = 2').replace('[conversation]\nschema_version = 4','[conversation]\nschema_version = 2');(d/'project.toml').write_text(config)
  state=json.loads((d/'state/state-contract.json').read_text());state['state_schema']=5;(d/'state/state-contract.json').write_text(json.dumps(state,indent=2,sort_keys=True)+'\n')
  profile=json.loads((d/'coordinator.json').read_text());profile.update(profile='claude-chat-v1',call_limit=3,mode='mock-cli',binary=str(fixture),auth_home=str(private/'home'),auth_config_dir=str(private/'home/.claude'))
  (d/'coordinator.json').write_text(json.dumps(profile)+'\n')
  key=json.loads((d/'state/project-identity.json').read_text())['project_key'];resources.append((p,key))
  return p,key,private/'runtime/waywr'/f'{key}.sock'
 def request(endpoint,key,kind,body):
  with socket.socket(socket.AF_UNIX) as peer:
   peer.settimeout(3);peer.connect(str(endpoint));stream=peer.makefile('rb')
   def call(k,b):peer.sendall((json.dumps({'v':1,'kind':k,'request_id':uuid.uuid4().hex,'body':b})+'\n').encode());return json.loads(stream.readline())['body']
   assert call('hello',{'protocol':{'major':1,'minor':3},'project_key':key,'release_id':release.name,'channel':'command'})['accepted']
   return call(kind,body)
 def launch(p,name,resume=False):
  nonlocal pid,fd
  pid,fd=pty.fork()
  if pid==0:
   fcntl.ioctl(0,termios.TIOCSWINSZ,struct.pack('HHHH',36,140,0,0));os.chdir(p);os.execve(str(exe),[str(exe)]+(['resume'] if resume else []),env)
  logs[name]=bytearray()
 def drain(name):
  if select.select([fd],[],[],.025)[0]:
   try:logs[name].extend(os.read(fd,65536))
   except OSError:pass
 def wait(name,label,probe,seconds=30):
  end=time.monotonic()+seconds
  while time.monotonic()<end:
   drain(name)
   try:
    value=probe()
    if value:return value
   except (FileNotFoundError,ConnectionRefusedError,socket.timeout):pass
   time.sleep(.03)
  raise AssertionError(label)
 def press(name,value):os.write(fd,value);time.sleep(.1);drain(name)
 def detach(name):
  nonlocal pid,fd
  press(name,b'\x11');press(name,b'\t');press(name,b'\r');end=time.monotonic()+10
  while time.monotonic()<end:
   drain(name);found,status=os.waitpid(pid,os.WNOHANG)
   if found:
    assert os.waitstatus_to_exitcode(status)==0;os.close(fd);pid=fd=None;return
  raise AssertionError('detach failed')
 def authorize(p,key,endpoint,name):
  launch(p,name);wait(name,'startup',lambda: b'MOCK CLI STREAM' in logs[name] and request(endpoint,key,'snapshot',{})['conversation']['chat']['status']=='awaiting-authorization')
  press(name,b'\x1bOR');press(name,b'\t');press(name,b'\r') # F3, explicit settings action
  wait(name,'authorization',lambda:request(endpoint,key,'snapshot',{})['conversation']['chat']['authorized'])
  # Ratatui emits sparse cursor updates, not complete strings; allow the existing
  # bounded event/snapshot refresh to display the committed authorization.
  time.sleep(.7);drain(name)
 def snapshot(e,k):return request(e,k,'snapshot',{})
 def child_path(p):
  paths=list((p/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/attempts/*/child.json'))
  return max(paths,key=lambda x:x.stat().st_mtime) if paths else None
 results={};latencies=[]
 try:
  p,key,e=prepare('project');authorize(p,key,e,'first')
  message='Hello, I would like to discuss a fixed-data local page.'
  press('first',message.encode()+b'\r')
  wait('first','provisional output',lambda:snapshot(e,key)['conversation'].get('provisional'))
  press('first',b'draft preserved across detach')
  start=time.monotonic();s=snapshot(e,key);latencies.append(time.monotonic()-start);assert latencies[-1]<1
  assert not s['tasks'];detach('first')
  end=time.monotonic()+20
  while time.monotonic()<end:
   s=snapshot(e,key)
   if s['conversation']['turns'][0]['status']=='completed':break
   time.sleep(.1)
  else:raise AssertionError('background completion')
  launch(p,'second',True);wait('second','reopen history/draft',lambda:message.encode() in logs['second'] and b'draft preserved across detach' in logs['second'])
  press('second',b'\x01\x7f');press('second',b'What tradeoffs does a units toggle have?\r')
  path=wait('second','second child',lambda:child_path(p) if snapshot(e,key)['conversation']['budgets']['conversation']['charged']==2 and len(list((p/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/attempts/*/child.json')))==2 else None)
  subprocess.run(['systemctl','--user','restart',f'waywr-{key}.service'],check=True,timeout=15)
  wait('second','restart adoption',lambda:snapshot(e,key)['conversation']['turns'][0]['status']=='completed')
  assert snapshot(e,key)['conversation']['budgets']['conversation']['charged']==2
  # Event and command sockets reconnect independently (two-second socket deadline).
  time.sleep(3);drain('second')
  press('second',b'Tell me more about those tradeoffs.\r')
  wait('second','third completion',lambda:snapshot(e,key)['conversation']['budgets']['conversation']['charged']==3 and snapshot(e,key)['conversation']['turns'][0]['status']=='completed')
  press('second',b'Fourth message must stay a draft\r');assert snapshot(e,key)['conversation']['budgets']['conversation']['charged']==3
  assert snapshot(e,key)['conversation']['chat']['status']=='exhausted'
  db=p/'.wayward-wrangler/state/state.sqlite'
  with sqlite3.connect('file:'+str(db)+'?mode=ro',uri=True) as c:
   body,outcome=c.execute("SELECT body,outcome FROM session_commands WHERE json_extract(body,'$.action')='submit_message' LIMIT 1").fetchone()
  assert request(e,key,'command',json.loads(body))==json.loads(outcome)
  assert snapshot(e,key)['conversation']['budgets']['conversation']['charged']==3
  assert b'PRIVATE_REASONING_CANARY' not in logs['first']+logs['second']
  packets=list((p/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/packet.json'))
  assert len(packets)==3
  receipts=list((p/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/attempts/*/receipt.json'));assert len(receipts)==3
  assert all(json.loads(r.read_text())['status']=='completed' for r in receipts)
  results['normal']={'calls':3,'receipts':3,'task_count':0,'status_latency_seconds':max(latencies),'detach_resume_draft':True,'restart_adopted_once':True,'exhaustion':True,'replay':True,'thinking_hidden':True}
  detach('second')
  q,k,ep=prepare('interrupt-project');authorize(q,k,ep,'interrupt');press('interrupt',b'Interrupt this credential-free fixture\r')
  child=wait('interrupt','active child',lambda:child_path(q));owned=json.loads(child.read_text())['identity'];cp=owned['pid']
  stat=Path(f'/proc/{cp}/stat').read_text().rsplit(')',1)[1].split();assert stat[19]==owned['start']
  os.kill(cp,signal.SIGSTOP) # test-owned mock process only, no model call
  time.sleep(.7);drain('interrupt');press('interrupt',b'/cancel');press('interrupt',b'\r');press('interrupt',b'\t');press('interrupt',b'\r')
  wait('interrupt','durable interruption request',lambda:any(v['type']=='conversation.cancel_requested' for v in snapshot(ep,k)['recent_events']))
  time.sleep(.2)
  if Path(f'/proc/{cp}/stat').exists():os.kill(cp,signal.SIGCONT)
  wait('interrupt','cancel receipt',lambda:snapshot(ep,k)['conversation']['turns'][0]['status']=='cancelled')
  assert not snapshot(ep,k)['tasks'];results['cancel']={'status':'cancelled','calls':1};detach('interrupt')
  q,k,ep=prepare('uncertain-project');authorize(q,k,ep,'uncertain');press('uncertain',b'Preserve partial evidence if runner disappears\r')
  path=wait('uncertain','uncertain child',lambda:child_path(q));claim=json.loads((path.parent/'claim.json').read_text());runner=claim['runner'];rp=runner['pid']
  # Verify the saved start identity immediately before fault injection.
  ticks=int(Path(f'/proc/{rp}/stat').read_text().rsplit(')',1)[1].split()[19]);assert str(ticks)==str(runner['start'])
  os.kill(rp,signal.SIGKILL)
  wait('uncertain','uncertain stopped state',lambda:snapshot(ep,k)['conversation']['turns'][0]['status']=='uncertain')
  assert snapshot(ep,k)['conversation']['budgets']['conversation']['charged']==1
  time.sleep(2);assert len(list((q/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/attempts/*/spawn-committed.json')))==1
  results['uncertain']={'status':'uncertain','calls':1,'spawn_count':1};detach('uncertain')
 finally:
  for name,data in logs.items():(root/(name+'.pty')).write_bytes(data)
  if pid:
   os.kill(pid,signal.SIGTERM);os.waitpid(pid,0);os.close(fd)
  cleanup=[]
  for project,key in resources:
   for child in (project/'.wayward-wrangler/state/session/coordinator').glob('*/call-v1/attempts/*/child.json'):
    identity=json.loads(child.read_text())['identity'];end=time.monotonic()+6
    while time.monotonic()<end:
     proc=Path(f'/proc/{identity["pid"]}/stat')
     if not proc.exists() or proc.read_text().rsplit(')',1)[1].split()[0]=='Z':break
     time.sleep(.1)
    else:raise AssertionError('Owned child still active; preserve service')
   unit=f'waywr-{key}.service';subprocess.run([str(exe),'session','stop'],cwd=project,env=env,capture_output=True,timeout=15)
   loaded=subprocess.run(['systemctl','--user','show',unit,'--property=LoadState','--value'],capture_output=True,text=True).stdout.strip()
   if loaded=='loaded':subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15)
   props=subprocess.run(['systemctl','--user','show',unit,'--property=MainPID,ActiveState,ExecStart'],capture_output=True,text=True).stdout;cleanup.append({'project_key':key,'unit':unit,'properties':props})
  for name,data in logs.items():(root/(name+'.pty')).write_bytes(data)
  (root/'cleanup.json').write_text(json.dumps(cleanup,indent=2)+'\n')
  (root/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
 print(json.dumps(results))
if __name__=='__main__':
 a=argparse.ArgumentParser();a.add_argument('--root',type=Path,required=True);a.add_argument('--release',type=Path,required=True);o=a.parse_args();main(o.root.resolve(),o.release.resolve())
