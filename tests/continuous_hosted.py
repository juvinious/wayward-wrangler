"""Schema-7 real host / pinned mock CLI gate. Never authenticated providers."""
import argparse,json,os,socket,sqlite3,subprocess,tempfile,time,uuid,signal,hashlib
from pathlib import Path

def main(root,binaries):
 assert not root.exists();root.mkdir(parents=True);runtime=Path(tempfile.mkdtemp(prefix='ww-dialogue-host-'));resources=[];result={}
 fixture=(Path(__file__).parent/'fixtures/continuous-coordinator/mock-cli.py').resolve()
 def wait(fn,limit=30):
  end=time.monotonic()+limit
  while time.monotonic()<end:
   try:
    value=fn()
    if value:return value
   except (OSError,ValueError,KeyError,IndexError):pass
   time.sleep(.04)
  raise AssertionError('Timeout in isolated host gate')
 def request(item,kind,body):
  with socket.socket(socket.AF_UNIX) as peer:
   peer.settimeout(2);peer.connect(str(item['endpoint']));f=peer.makefile('rb')
   def call(k,b):peer.sendall((json.dumps({'v':1,'kind':k,'request_id':uuid.uuid4().hex,'body':b})+'\n').encode());return json.loads(f.readline())['body']
   assert call('hello',{'protocol':{'major':1,'minor':5},'project_key':item['key'],'release_id':'dialogue-host','channel':'command'})['accepted']
   return call(kind,body)
 def start(item):
  item['log']=(item['project']/f'service-{time.time_ns()}.log').open('wb');item['process']=subprocess.Popen([str(binaries/'waywr-service'),'--project',str(item['project']),'--state-dir',str(item['state']),'--project-key',item['key'],'--release-id','dialogue-host','--ipc',str(item['endpoint']),'--runner',str(binaries/'waywr-runner'),'--frontend',str(binaries/'waywr')],stdout=item['log'],stderr=subprocess.STDOUT)
  wait(lambda:request(item,'snapshot',{}))
 def stop(item):
  if item['process'].poll() is None:item['process'].send_signal(signal.SIGTERM);item['process'].wait(timeout=12)
  item['log'].close()
 def prepare(name):
  project=root/name;state=project/'.wayward-wrangler/state';state.mkdir(parents=True);key=uuid.uuid4().hex
  (state/'state-contract.json').write_text(json.dumps({'schema_version':1,'state_schema':7,'execution_host':'systemd-user-v1','project_key':key})+'\n')
  (project/'.wayward-wrangler/coordinator.json').write_text(json.dumps({'schema_version':1,'profile':'claude-proposal-v2','mode':'mock-cli','provider':'claude','model':'claude-sonnet-5-5','effort':'medium','call_limit':6,'binary':str(fixture),'auth_home':str(runtime),'auth_config_dir':str(runtime)})+'\n')
  item={'project':project,'state':state,'key':key,'endpoint':runtime/(key[:8]+'.sock')};resources.append(item);start(item);c=snapshot(item)['conversation'];assert command(item,'select_coordinator',{'authorize':True,'profile_sha256':c['chat']['profile_sha256']},expected_generation=1)['status']=='accepted';return item
 def snapshot(item):return request(item,'snapshot',{})
 def command(item,action,payload,**extra):return request(item,'command',{'command_id':uuid.uuid4().hex,'action':action,'payload':payload,**extra})
 def send(item,text,ack=None):
  r=command(item,'submit_message',{'text':text,'subject_id':'discussion','acknowledged_outcome':ack});assert r['status']=='accepted',r;return r['turn_id']
 def turn(item,id):return next((t for t in snapshot(item)['conversation']['turns'] if t['turn_id']==id),{})
 def terminal(item,id):return wait(lambda:turn(item,id)['status'] if turn(item,id).get('status') in ('completed','output-invalid','cancelled','uncertain') else None)
 def attempts(item):return list(item['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'))
 try:
  a=prepare('a');b=prepare('b');one=send(a,'fixture-proposal');wait(lambda:turn(a,one).get('status')=='reserved');frozen=a['state']/f'session/conversation/{one}/packet.json';before=frozen.read_bytes();two=send(a,'Discuss another detail');assert terminal(a,one)=='completed';assert terminal(a,two)=='completed';assert frozen.read_bytes()==before
  packet=json.loads((a['state']/f'session/conversation/{two}/packet.json').read_text());assert any(m['role']=='assistant' for m in packet['selected_context']['messages'])
  task=snapshot(a)['tasks'][0]['task_id'];detail=request(a,'task.get',{'task_id':task})['scope'];body={'base':detail['base'],'decision_revision':detail['decision_revision'],'authorization_command_id':detail['authorization_command_id'],'acknowledged_unresolved_sha256':hashlib.sha256((json.dumps(detail['record']['proposal']['unresolved_decisions'],sort_keys=True,indent=2,ensure_ascii=False)+'\n').encode()).hexdigest()};r=command(a,'authorize_scope',body);assert r['status']=='accepted',r
  fail=send(a,'fixture-format');wait(lambda:turn(a,fail).get('status')=='reserved');old=send(a,'Held old message');assert terminal(a,fail)=='output-invalid';assert turn(a,old)['status']=='held';assert request(a,'task.get',{'task_id':task})['scope']['base']==detail['base']
  stop(a);start(a);pause=snapshot(a)['conversation']['continuous']['pause'];assert pause['recoverable'];new=send(a,'fixture-long',pause);wait(lambda:list(a['state'].glob(f'session/coordinator/{new}/call-v1/attempts/*/child.json')));begin=time.monotonic();r=command(a,'cancel_conversation',{'turn_id':new});latency=time.monotonic()-begin;assert r['actions_fenced'] and latency<1;assert terminal(a,new)=='cancelled';assert turn(a,old)['status']=='held'
  pause=snapshot(a)['conversation']['continuous']['pause'];r=command(a,'continue_held',{'turn_ids':[old],'acknowledged_outcome':pause});assert r['released']==1;assert terminal(a,old)=='completed';assert snapshot(a)['conversation']['budgets']['conversation']['charged']==5
  # B remains independent; exact foreign task/turn IDs refuse without a call.
  assert command(b,'withdraw_queued',{'turn_id':old})['status']=='rejected';assert command(b,'submit_message',{'text':'Foreign','task_id':task})['code']=='task_not_found';assert snapshot(b)['conversation']['budgets']['conversation']['charged']==0
  begin=time.monotonic();request(b,'snapshot',{});status_latency=time.monotonic()-begin;assert status_latency<1
  send(b,'Other project discussion');wait(lambda:snapshot(b)['conversation']['turns'][0]['status']=='completed');bp=json.loads(next(b['state'].glob('session/conversation/*/packet.json')).read_text());assert a['key'] not in json.dumps(bp)
  # Lost acceptance reply: same durable command is reconciled, never a fresh turn.
  stable={'command_id':'lost-reply','action':'submit_message','payload':{'text':'Retain this once','subject_id':'discussion'}}
  with socket.socket(socket.AF_UNIX) as peer:
   peer.connect(str(b['endpoint']));f=peer.makefile('rb');peer.sendall((json.dumps({'v':1,'kind':'hello','request_id':'hello-lost','body':{'protocol':{'major':1,'minor':5},'project_key':b['key'],'release_id':'dialogue-host','channel':'command'}})+'\n').encode());assert json.loads(f.readline())['body']['accepted'];peer.sendall((json.dumps({'v':1,'kind':'command','request_id':'lost','body':stable})+'\n').encode())
  accepted=wait(lambda:(lambda r:r if r.get('status')=='accepted' else None)(request(b,'command_get',{'command_id':'lost-reply'})))
  assert request(b,'command',stable)==accepted;assert terminal(b,accepted['turn_id'])=='completed';assert len(attempts(b))==2
  # Supervisor restarts while the exact mock host/child continues.
  c=prepare('restart');running=send(c,'fixture-long');wait(lambda:list(c['state'].glob('session/coordinator/*/call-v1/attempts/*/child.json')));stop(c);start(c);begin=time.monotonic();request(c,'snapshot',{});assert time.monotonic()-begin<1;assert terminal(c,running)=='completed';assert len(attempts(c))==1;assert snapshot(c)['conversation']['budgets']['conversation']['charged']==1
  for item in resources:
   for receipt in attempts(item):
    rec=json.loads(receipt.read_text());proof=json.loads(receipt.with_name('completion.json').read_text());assert rec['schema_version']==3 and proof['packet_sha256']==rec['packet_sha256'];assert hashlib.sha256((json.dumps(proof,sort_keys=True,indent=2,ensure_ascii=False)+'\n').encode()).hexdigest()==rec['completion_sha256']
   with sqlite3.connect(item['state']/'state.sqlite') as db:
    assert db.execute('select count(*) from session_admissions').fetchone()[0]==0;assert db.execute('select count(*) from app_runs').fetchone()[0]==0
  result={'causal_followup':True,'frozen_packet':True,'invalid_output_preserves_scope':True,'recovery_restart':True,'held_release_explicit':True,'interrupt_latency':latency,'status_latency':status_latency,'two_project_isolation':True,'a_calls':5,'a_receipts':len(attempts(a)),'b_calls':2,'b_receipts':len(attempts(b)),'restart_call_receipts':len(attempts(c)),'lost_reply_once':True,'workers':0}
 finally:
  cleanup=[]
  for item in resources:
   stop(item)
   for intent in item['state'].glob('session/coordinator/*/call-v1/attempts/*/intent.json'):
    unit=json.loads(intent.read_text())['unit'];show=subprocess.run(['systemctl','--user','show',unit,'--property=ControlGroup,MainPID,ExecStart'],capture_output=True,text=True).stdout;group=next((l.partition('=')[2] for l in show.splitlines() if l.startswith('ControlGroup=')),'');procs=Path('/sys/fs/cgroup'+group)/'cgroup.procs' if group else None
    if procs and procs.exists() and procs.read_text().strip():cleanup.append({'unit':unit,'retained_nonempty':True});continue
    assert str(binaries/'waywr-runner') in show or 'MainPID=0' in show;subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15);cleanup.append({'unit':unit,'stopped_empty':True})
  (root/'cleanup.json').write_text(json.dumps(cleanup,indent=2)+'\n');(root/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps(result))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--binaries',type=Path,required=True);a=p.parse_args();main(a.root.resolve(),a.binaries.resolve())
