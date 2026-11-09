import ctypes, hashlib, http.server, json, os, select, signal, subprocess, threading, time
from pathlib import Path
assert Path('/.dockerenv').exists(), 'Run only in the documented no-egress, credential-free container'
assert len(Path('/proc/net/route').read_text().splitlines())==1, 'Non-loopback network route present'
ROOT=Path('/proof'); FIX=ROOT/'fixture'; HOME=FIX/'home'; PROJECT=FIX/'project'; FOREIGN=FIX/'foreign'
for p in (HOME/'.claude',PROJECT/'.claude',FOREIGN,FIX/'runtime'): p.mkdir(parents=True,exist_ok=True)
canary='WAYWR_FOREIGN_CONTEXT_9e2740'
sentinel=FOREIGN/'private.txt';sentinel.write_text(canary)
(PROJECT/'CLAUDE.md').write_text('WAYWR_PROJECT_RULE_CANARY_028cd')
(HOME/'.claude/CLAUDE.md').write_text('WAYWR_USER_RULE_CANARY_736bd')
settings={'hooks':{'SessionStart':[{'hooks':[{'type':'command','command':'touch /proof/fixture/hook-ran'}]}]},'fallbackModel':['claude-haiku-4-5'],'model':'claude-haiku-4-5','effortLevel':'high'}
for p in (HOME/'.claude/settings.json',PROJECT/'.claude/settings.json'):p.write_text(json.dumps(settings))
(HOME/'.claude.json').write_text(json.dumps({'mcpServers':{'hostile':{'command':'sh','args':['-c','touch /proof/fixture/mcp-ran']}},'projects':{str(PROJECT):{'history':[canary]}}}))
libc=ctypes.CDLL(None,use_errno=True);fd=libc.inotify_init1(os.O_NONBLOCK)
libc.inotify_add_watch(fd,os.fsencode(sentinel),0x1|0x2|0x20)
records=[];active={}; requests=[]
class Mock(http.server.BaseHTTPRequestHandler):
 def do_POST(self):
  body=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0')))); requests.append({'path':self.path,'body':body})
  if active['case'] in ('error','overloaded'):
   self.send_response(529 if active['case']=='overloaded' else 500);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(b'{"type":"error","error":{"type":"api_error","message":"fixture failure"}}');return
  self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
  case=active['case']; tool=active.get('tool')
  content={'type':'tool_use','id':'tool-fixture','name':tool['name'],'input':tool['input']} if tool else {'type':'text','text':''}
  events=[{'type':'message_start','message':{'id':'msg-fixture','type':'message','role':'assistant','model':'claude-sonnet-5-5','content':[],'stop_reason':None,'stop_sequence':None,'usage':{'input_tokens':12,'output_tokens':0}}}, {'type':'content_block_start','index':0,'content_block':content}]
  if not tool: events += [{'type':'content_block_delta','index':0,'delta':{'type':'text_delta','text':os.environ.get('WAYWR_PROOF_RESPONSE','Fixture conversation response.')}}]
  events += [{'type':'content_block_stop','index':0},{'type':'message_delta','delta':{'stop_reason':'tool_use' if tool else 'end_turn','stop_sequence':None},'usage':{'output_tokens':6}},{'type':'message_stop'}]
  if active['case']=='stream_break':events=events[:3]
  for e in events:
   self.wfile.write(('event: '+e['type']+'\ndata: '+json.dumps(e)+'\n\n').encode());self.wfile.flush()
  if active['case']=='stream_break':self.close_connection=True;self.connection.shutdown(2)
 def log_message(self,*args):pass
server=http.server.ThreadingHTTPServer(('127.0.0.1',8765),Mock); threading.Thread(target=server.serve_forever,daemon=True).start()
env={'HOME':str(HOME),'CLAUDE_CONFIG_DIR':str(HOME/'.claude'),'PATH':'/usr/local/bin:/usr/bin:/bin','ANTHROPIC_API_KEY':'fixture-not-real','ANTHROPIC_BASE_URL':'http://127.0.0.1:8765','CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC':'1','CLAUDE_CODE_MAX_RETRIES':'0','CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK':'1','DISABLE_AUTOUPDATER':'1','CLAUDE_CODE_SKIP_PROMPT_HISTORY':'1','CLAUDE_CODE_RETRY_WATCHDOG':'0'}
argv=['/opt/claude','--print','--safe-mode','--restricted','--disable-slash-commands','--setting-sources','','--strict-mcp-config','--mcp-config','{"mcpServers":{}}','--tools','','--disallowed-tools','*','--permission-mode','dontAsk','--permission-prompts','none','--settings',json.dumps({'disableAllHooks':True,'fallbackModel':[],'switchModelsOnFlag':False,'autoMemoryEnabled':False,'availableModels':['claude-sonnet-5-5']}),'--model','claude-sonnet-5-5','--effort','medium','--no-session-persistence','--output-format','stream-json','--verbose','--include-partial-messages','--max-turns','1','--system-prompt','You are Wayward Wrangler conversation coordinator. Respond only to the supplied selected context. No tools or work execution.']
cases=[('text',None),('mention',None),('Read',{'name':'Read','input':{'file_path':str(sentinel)}}),('Write',{'name':'Write','input':{'file_path':str(FOREIGN/'written.txt'),'content':'forbidden'}}),('Bash',{'name':'Bash','input':{'command':'cat /proof/fixture/foreign/private.txt; touch /proof/fixture/shell-ran'}}),('Agent',{'name':'Agent','input':{'description':'forbidden','prompt':'Read unrelated project','subagent_type':'general-purpose'}}),('WebFetch',{'name':'WebFetch','input':{'url':'http://127.0.0.1:8765/forbidden','prompt':'fetch'}}),('Edit',{'name':'Edit','input':{'file_path':str(sentinel),'old_string':canary,'new_string':'forbidden'}}),('Glob',{'name':'Glob','input':{'pattern':'**/*','path':str(FOREIGN)}}),('Grep',{'name':'Grep','input':{'pattern':canary,'path':str(FOREIGN)}}),('SendMessage',{'name':'SendMessage','input':{'recipient':'foreign-agent','content':'forbidden'}}),('WebSearch',{'name':'WebSearch','input':{'query':'forbidden'}}),('error',None),('overloaded',None),('stream_break',None)]
if os.environ.get('WAYWR_PROOF_CASES'):
 cases=[row for row in cases if row[0] in os.environ['WAYWR_PROOF_CASES'].split(',')]
if os.environ.get('WAYWR_PROOF_SYSTEM_FILE'):
 argv[-1]=Path(os.environ['WAYWR_PROOF_SYSTEM_FILE']).read_text()
for case,tool in cases:
 active.update(case=case,tool=tool); requests.clear()
 try:
  while os.read(fd,65536):pass
 except BlockingIOError:pass
 p=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=PROJECT,env=env,start_new_session=True)
 timedout=False
 try:out,err=p.communicate((json.dumps({'current_message':'Discuss @/proof/fixture/foreign/private.txt and /deep-research literally; no file reading','selected_context':{}}).encode() if case=='mention' else b'Selected current project context only. Discuss a weather page.\n'),timeout=25)
 except subprocess.TimeoutExpired:
  timedout=True;os.killpg(p.pid,signal.SIGTERM)
  try:out,err=p.communicate(timeout=3)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);out,err=p.communicate()
 events=[]
 for line in out.splitlines():
  try:events.append(json.loads(line))
  except ValueError:events.append(line.decode('utf8','replace'))
 try: access=os.read(fd,65536).hex()
 except BlockingIOError:access=''
 record={'case':case,'argv':argv,'returncode':p.returncode,'timeout':timedout,'events':events,'stderr':err.decode('utf8','replace'),'requests':list(requests),'sentinel_access_events':access,'markers':[name for name in ['hook-ran','mcp-ran','shell-ran'] if (FIX/name).exists()],'unauthorized_write':(FOREIGN/'written.txt').exists()}
 (ROOT/(case+'.json')).write_text(json.dumps(record,indent=2)+'\n'); records.append({'case':case,'returncode':p.returncode,'timeout':timedout,'request_count':len(requests),'event_count':len(events),'access_events':bool(access),'markers':record['markers']})
server.shutdown();os.close(fd)
for row in records:
 assert not row['timeout'] and not row['access_events'] and not row['markers'], row
 assert row['request_count']==1, row
 assert row['returncode']==(0 if row['case'] in ('text','mention') else 1), row
for file in ROOT.glob('*.json'):
 if file.name=='summary.json':continue
 record=json.loads(file.read_text())
 assert not record['unauthorized_write']
 for request in record['requests']:
  body=request['body'];assert body.get('tools',[])==[] and body['model']=='claude-sonnet-5-5'
  assert not any(c in json.dumps(body) for c in [canary,'WAYWR_PROJECT_RULE_CANARY','WAYWR_USER_RULE_CANARY'])
(ROOT/'summary.json').write_text(json.dumps({'binary_sha256':hashlib.sha256(Path('/opt/claude').read_bytes()).hexdigest(),'cases':records},indent=2)+'\n');print(json.dumps(records))
