"""Scoped fake CLI/real operation-host recovery gate. No credentials or providers."""
import argparse,json,os,signal,socket,sqlite3,subprocess,tempfile,time,uuid
from pathlib import Path

def main(root,binaries):
 assert not root.exists();root.mkdir(parents=True)
 runtime=Path(tempfile.mkdtemp(prefix='ww-scope-host-'));resources=[];results={}
 fixture=(Path(__file__).parent/'fixtures/claude-scope/mock-cli.py').resolve()
 def prepare(name):
  project=root/name;state=project/'.wayward-wrangler/state';state.mkdir(parents=True);key=uuid.uuid4().hex;endpoint=runtime/(key[:8]+'.sock')
  (state/'state-contract.json').write_text(json.dumps({'schema_version':1,'state_schema':6,'execution_host':'systemd-user-v1','project_key':key})+'\n')
  (project/'.wayward-wrangler/coordinator.json').write_text(json.dumps({'schema_version':1,'profile':'claude-proposal-v1','mode':'mock-cli','provider':'claude','model':'claude-sonnet-5-5','effort':'medium','call_limit':6,'binary':str(fixture),'auth_home':str(runtime),'auth_config_dir':str(runtime)})+'\n')
  item={'project':project,'state':state,'key':key,'endpoint':endpoint,'process':None,'log':None};resources.append(item);start(item);return item
 def start(item):
  item['log']=(item['project']/f'service-{time.time_ns()}.log').open('wb')
  args=[str(binaries/'waywr-service'),'--project',str(item['project']),'--state-dir',str(item['state']),'--project-key',item['key'],'--release-id','scope-host-fixture','--ipc',str(item['endpoint']),'--runner',str(binaries/'waywr-runner'),'--frontend',str(binaries/'waywr')]
  item['process']=subprocess.Popen(args,stdout=item['log'],stderr=subprocess.STDOUT)
  wait(lambda:request(item,'snapshot',{}));return item
 def request(item,kind,body):
  with socket.socket(socket.AF_UNIX) as peer:
   peer.settimeout(2);peer.connect(str(item['endpoint']));f=peer.makefile('rb')
   def call(k,b):peer.sendall((json.dumps({'v':1,'kind':k,'request_id':uuid.uuid4().hex,'body':b})+'\n').encode());return json.loads(f.readline())['body']
   assert call('hello',{'protocol':{'major':1,'minor':4},'project_key':item['key'],'release_id':'scope-host-fixture','channel':'command'})['accepted']
   return call(kind,body)
 def wait(fn,limit=25):
  end=time.monotonic()+limit
  while time.monotonic()<end:
   try:
    v=fn()
    if v:return v
   except (OSError,ValueError,KeyError):pass
   time.sleep(.04)
  raise AssertionError('Timed out waiting for isolated host outcome')
 def snap(item):return request(item,'snapshot',{})
 def cmd(item,action,payload,**expected):return request(item,'command',{'command_id':uuid.uuid4().hex,'action':action,'payload':payload,**expected})
 def authorize(item):
  c=snap(item)['conversation'];assert cmd(item,'select_coordinator',{'authorize':True,'profile_sha256':c['chat']['profile_sha256']},expected_generation=c['binding']['generation'])['status']=='accepted'
 def send(item,text,task=None):
  r=cmd(item,'submit_message',{'text':text,'task_id':task},expected_state_revision=snap(item)['state_revision']);assert r['status']=='accepted',r;return r['turn_id']
 def terminal(item):return wait(lambda:snap(item)['conversation']['turns'][0]['status'] if snap(item)['conversation']['turns'][0]['status'] in ('completed','cancelled','uncertain','failed','output-invalid') else None)
 def children(item):return list(item['state'].glob('session/coordinator/*/call-v1/attempts/*/child.json'))
 def identity_signal(record,sig):
  identity=record['identity'];path=Path(f'/proc/{identity["pid"]}/stat');raw=path.read_text().rsplit(')',1)[1].split();assert raw[19]==str(identity['start']);assert identity['boot']==Path('/proc/sys/kernel/random/boot_id').read_text().strip();os.kill(identity['pid'],sig)
 def stop(item):
  p=item['process']
  if p and p.poll() is None:p.send_signal(signal.SIGTERM);p.wait(timeout=12)
  if item['log']:item['log'].close()
 try:
  a=prepare('a');b=prepare('b');authorize(a);authorize(b)
  send(a,'Natural discussion');assert terminal(a)=='completed';assert not snap(a)['tasks']
  send(a,'Clarify scope');assert terminal(a)=='completed';question=snap(a)['conversation']['questions'][0];assert question['status']=='open'
  # B progresses while A has a durable unanswered question; no A material in B packet.
  send(b,'Independent discussion');assert terminal(b)=='completed'
  bpacket=json.loads(next(b['state'].glob('session/conversation/*/packet.json')).read_text());assert a['key'] not in json.dumps(bpacket)
  answer=cmd(a,'answer_conversation',{'question_id':question['question_id'],'answer':'Include the toggle','decision_revision':question['scope']['decision_revision']});assert answer['status']=='accepted';assert snap(a)['conversation']['budgets']['conversation']['charged']==2
  question=snap(a)['conversation']['questions'][0]
  payload={'text':question['answer'],'task_id':question['scope']['target_task_id'],'answer_question_id':question['question_id']}
  command={'command_id':'continuation-once','action':'submit_message','expected_state_revision':snap(a)['state_revision'],'payload':payload}
  assert request(a,'command',command)['status']=='accepted'
  wait(lambda:len(children(a))==3);stop(a);start(a);assert terminal(a)=='completed';assert len(children(a))==3
  tasks=snap(a)['tasks'];assert len(tasks)==1;task=tasks[0]['task_id'];assert tasks[0]['run_id'] is None
  assert request(a,'command',command)['status']=='accepted';assert len(children(a))==3
  for forged in ('admit_task','authorize_consultation','answer_task'):
   assert cmd(a,forged,{'task_id':task})['code']=='outside_increment'
  assert snap(a)['conversation']['budgets']['task']['limit']==0
  results['normal']={'calls':3,'receipts':len(list(a['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'))),'one_proposal':True,'restart_adoption_once':True,'continuation_replay':True,'two_project_isolation':True}
  bad=prepare('invalid');authorize(bad);send(bad,'fixture-malformed');assert terminal(bad)=='output-invalid';assert not snap(bad)['tasks'];assert snap(bad)['conversation']['chat']['status']=='blocked';results['output_invalid']={'calls':1,'receipts':len(list(bad['state'].glob('session/coordinator/*/call-v1/attempts/*/receipt.json'))),'tasks':0}
  c=prepare('cancel');authorize(c);turn=send(c,'Cancelable mock')
  path=wait(lambda:children(c)[0] if children(c) else None);identity_signal(json.loads(path.read_text()),signal.SIGSTOP)
  begin=time.monotonic();r=cmd(c,'cancel_conversation',{'turn_id':turn});latency=time.monotonic()-begin;assert r['status']=='accepted';assert latency<1
  assert terminal(c)=='cancelled';assert snap(c)['conversation']['budgets']['conversation']['charged']==1;assert len(children(c))==1
  results['cancel']={'status':'cancelled','calls':1,'control_latency_seconds':latency}
  d=prepare('uncertain');authorize(d);send(d,'Uncertain mock');path=wait(lambda:children(d)[0] if children(d) else None)
  claim=json.loads((path.parent/'claim.json').read_text());identity_signal({'identity':claim['runner']},signal.SIGKILL)
  assert terminal(d)=='uncertain';time.sleep(1.7);assert len(children(d))==1;assert snap(d)['conversation']['budgets']['conversation']['charged']==1
  results['uncertain']={'calls':1,'spawn_count':1,'automatic_replay':False}
 finally:
  disposition=[]
  for item in resources:
   try:stop(item)
   except Exception as e:disposition.append({'project_key':item['key'],'cleanup_error':str(e)});continue
   for path in item['state'].glob('session/coordinator/*/call-v1/attempts/*/intent.json'):
    record=json.loads(path.read_text());unit=record['unit'];props=subprocess.run(['systemctl','--user','show',unit,'--property=ControlGroup,MainPID,ExecStart'],capture_output=True,text=True).stdout
    group=next((v.split('=',1)[1] for v in props.splitlines() if v.startswith('ControlGroup=')),'')
    procs=Path('/sys/fs/cgroup'+group)/'cgroup.procs' if group else None
    if procs and procs.exists() and procs.read_text().strip():disposition.append({'unit':unit,'retained':'nonempty cgroup'});continue
    assert str(binaries/'waywr-runner') in props or 'MainPID=0' in props
    subprocess.run(['systemctl','--user','stop',unit],check=True,timeout=15);disposition.append({'unit':unit,'stopped_empty':True})
  (root/'cleanup.json').write_text(json.dumps(disposition,indent=2)+'\n');(root/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
 print(json.dumps(results))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--binaries',type=Path,required=True);a=p.parse_args();main(a.root.resolve(),a.binaries.resolve())
