import ctypes, hashlib, http.server, json, os, select, signal, subprocess, threading, time, struct
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
# Optional focused extension; the original proof remains reproducible unchanged.
builtin_proof=os.environ.get('WAYWR_BUILTIN_PROOF')=='1'
watched={str(sentinel):'foreign'};canaries=[canary,'WAYWR_PROJECT_RULE_CANARY','WAYWR_USER_RULE_CANARY']
if builtin_proof:
 for path,label in [(PROJECT/'AGENTS.md','project_agents'),(PROJECT/'.claude/AGENTS.md','project_dot_agents'),(FIX/'AGENTS.md','ancestor_agents'),(HOME/'AGENTS.md','home_agents'),(HOME/'.claude/AGENTS.md','user_agents')]:
  value='WAYWR_INSTRUCTION_'+label.upper()+'_e68c2f';path.write_text(value);canaries.append(value);watched[str(path)]=label
 plugin=HOME/'.claude/plugins/cache/fixture/waywr-inherited-canary/1.0.0'
 for d in ['.claude-plugin','skills/inherited-canary','hooks']: (plugin/d).mkdir(parents=True,exist_ok=True)
 content={'.claude-plugin/plugin.json':json.dumps({'name':'waywr-inherited-canary','version':'1.0.0','description':'WAYWR_PLUGIN_MANIFEST_79b24a'}),'skills/inherited-canary/SKILL.md':'---\nname: inherited-canary\ndescription: WAYWR_PLUGIN_SKILL_2a98d1\n---\nWAYWR_PLUGIN_BODY_abc743','hooks/hooks.json':json.dumps({'hooks':{'SessionStart':[{'hooks':[{'type':'command','command':'touch /proof/fixture/plugin-hook-ran'}]}]}}),'.mcp.json':json.dumps({'mcpServers':{'inherited':{'command':'sh','args':['-c','touch /proof/fixture/plugin-mcp-ran']}}})}
 for rel,value in content.items():
  path=plugin/rel;path.write_text(value);watched[str(path)]='plugin_'+rel
 canaries+=['WAYWR_PLUGIN_MANIFEST_79b24a','WAYWR_PLUGIN_SKILL_2a98d1','WAYWR_PLUGIN_BODY_abc743']
 registry=HOME/'.claude/plugins/installed_plugins.json';registry.write_text(json.dumps({'version':2,'plugins':{'waywr-inherited-canary@fixture':[{'scope':'user','installPath':str(plugin),'version':'1.0.0','installedAt':'2026-10-15T00:00:00.000Z','lastUpdated':'2026-10-15T00:00:00.000Z'}]}}));watched[str(registry)]='plugin_registry'
 market=HOME/'.claude/plugins/marketplaces/fixture';(market/'.claude-plugin').mkdir(parents=True)
 (market/'.claude-plugin/marketplace.json').write_text(json.dumps({'name':'fixture','owner':{'name':'Waywr fixture'},'plugins':[{'name':'waywr-inherited-canary','source':str(plugin),'version':'1.0.0'}]}))
 (HOME/'.claude/plugins/known_marketplaces.json').write_text(json.dumps({'fixture':{'source':{'source':'directory','path':str(market)},'installLocation':str(market),'lastUpdated':'2026-10-15T00:00:00.000Z','autoUpdate':False}}))
 settings['enabledPlugins']={'waywr-inherited-canary@fixture':True}
 for path in [HOME/'.claude/settings.json',PROJECT/'.claude/settings.json']:path.write_text(json.dumps(settings))
 for path,label in [(PROJECT/'CLAUDE.md','project_claude'),(HOME/'.claude/CLAUDE.md','user_claude')]:watched[str(path)]=label
libc=ctypes.CDLL(None,use_errno=True);fd=libc.inotify_init1(os.O_NONBLOCK);assert fd>=0
watch_ids={}
for path,label in watched.items():
 wd=libc.inotify_add_watch(fd,os.fsencode(path),0x1|0x2|0x20);assert wd>=0,path;watch_ids[wd]=label
def file_events():
 result=[]
 while True:
  try:data=os.read(fd,65536)
  except BlockingIOError:break
  offset=0
  while offset<len(data):
   wd,mask,cookie,size=struct.unpack_from('iIII',data,offset);offset+=16+size
   assert not mask&0x4000,'inotify queue overflow'
   result.append({'file':watch_ids.get(wd,'unknown'),'mask':mask})
 return result
if builtin_proof:
 # Positive instrumentation control: a real child opens/reads every watched file.
 subprocess.run(['/usr/local/bin/python3','-c','import pathlib,sys; [pathlib.Path(p).read_bytes() for p in sys.argv[1:]]',*watched],check=True)
 control=file_events();assert {e['file'] for e in control if e['mask']&1}==set(watched.values())
 (ROOT/'file-watch-control.json').write_text(json.dumps(control,indent=2)+'\n')
records=[];active={}; requests=[]
class Mock(http.server.BaseHTTPRequestHandler):
 def do_POST(self):
  body=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0')))); requests.append({'path':self.path,'body':body})
  if active['case'] in ('error','overloaded','unauthorized','rate'):
   self.send_response({'overloaded':529,'error':500,'unauthorized':401,'rate':429}[active['case']]);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(b'{"type":"error","error":{"type":"api_error","message":"fixture failure"}}');return
  self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
  case=active['case']; tool=active.get('tool')
  content={'type':'tool_use','id':'tool-fixture','name':tool['name'],'input':{}} if tool else {'type':'text','text':''}
  events=[{'type':'message_start','message':{'id':'msg-fixture','type':'message','role':'assistant','model':'claude-sonnet-5-5','content':[],'stop_reason':None,'stop_sequence':None,'usage':{'input_tokens':12,'output_tokens':0}}}, {'type':'content_block_start','index':0,'content_block':content}]
  if tool:
   raw=json.dumps(tool['input'])
   if case=='duplicate':raw=raw.replace('"schema_version": 3','"schema_version": 3, "schema_version": 3')
   events += [{'type':'content_block_delta','index':0,'delta':{'type':'input_json_delta','partial_json':raw[:30]}},{'type':'content_block_delta','index':0,'delta':{'type':'input_json_delta','partial_json':raw[30:]}}]
  if not tool: events += [{'type':'content_block_delta','index':0,'delta':{'type':'text_delta','text':'x'*70000 if case=='oversize' else 'Fixture conversation response.'}}]
  events += [{'type':'content_block_stop','index':0},{'type':'message_delta','delta':{'stop_reason':'tool_use' if tool else 'end_turn','stop_sequence':None},'usage':{'output_tokens':6}},{'type':'message_stop'}]
  if case in ('truncated','cancel'):events=events[:3]
  for e in events:
   self.wfile.write(('event: '+e['type']+'\ndata: '+json.dumps(e)+'\n\n').encode());self.wfile.flush()
   if case=='cancel':time.sleep(5)
 def log_message(self,*args):pass
server=http.server.ThreadingHTTPServer(('127.0.0.1',8765),Mock); threading.Thread(target=server.serve_forever,daemon=True).start()
env={'HOME':str(HOME),'CLAUDE_CONFIG_DIR':str(HOME/'.claude'),'PATH':'/usr/local/bin:/usr/bin:/bin','ANTHROPIC_API_KEY':'fixture-not-real','ANTHROPIC_BASE_URL':'http://127.0.0.1:8765','CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC':'1','CLAUDE_CODE_MAX_RETRIES':'0','CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK':'1','DISABLE_AUTOUPDATER':'1','CLAUDE_CODE_SKIP_PROMPT_HISTORY':'1','CLAUDE_CODE_RETRY_WATCHDOG':'0'}
argv=['/opt/claude','--print','--safe-mode','--restricted','--disable-slash-commands','--setting-sources','','--strict-mcp-config','--mcp-config','{"mcpServers":{}}','--tools','','--disallowed-tools','Read,Write,Edit,Glob,Grep,Bash,Agent,SendMessage,WebFetch,WebSearch,Task,TaskOutput,TaskStop,NotebookEdit,Skill,AskUserQuestion,EnterPlanMode,ExitPlanMode','--permission-mode','dontAsk','--permission-prompts','none','--settings',json.dumps({'disableAllHooks':True,'fallbackModel':[],'switchModelsOnFlag':False,'autoMemoryEnabled':False,'availableModels':['claude-sonnet-5-5']}),'--model','claude-sonnet-5-5','--effort','medium','--no-session-persistence','--output-format','stream-json','--verbose','--include-partial-messages','--max-turns','1','--system-prompt','You are Wayward Wrangler conversation coordinator. Respond only to the supplied selected context. No tools or work execution.']
schema=json.loads((ROOT/'result-v3.schema.json').read_text());argv+=['--json-schema',json.dumps(schema)]
reply={'schema_version':3,'assistant_text':'Fixture **discussion**.','evidence_refs':[],'action':None}
proposal={'title':'Weather','objective':'Fixed samples','deliverables':['index.html'],'in_scope':['Local page'],'out_of_scope':['API'],'constraints':['No network'],'acceptance_criteria':['Shows samples'],'unresolved_decisions':[]}
extra=[('question',{'kind':'ask_question','payload':{'prompt':'Units?','rationale':'Scope','options':['C']}}),('proposal',{'kind':'upsert_proposal','payload':{'base':None,'proposal':proposal}}),('suggestion',{'kind':'suggest_question_disposition','payload':{'question_id':'q-one','question_revision':1,'disposition':'answer','answer':'C','source_message_id':'m-one','source_sha256':'f'*64,'quote_start':0,'quote_end':1}})]
cases=[('valid',{'name':'StructuredOutput','input':reply}),('malformed',{'name':'StructuredOutput','input':{**reply,'forbidden':True}}),('text',None),('Read',{'name':'Read','input':{'file_path':str(sentinel)}}),('Write',{'name':'Write','input':{'file_path':str(FOREIGN/'written.txt'),'content':'forbidden'}}),('Bash',{'name':'Bash','input':{'command':'cat /proof/fixture/foreign/private.txt; touch /proof/fixture/shell-ran'}}),('Agent',{'name':'Agent','input':{'description':'forbidden','prompt':'Read unrelated project','subagent_type':'general-purpose'}}),('WebFetch',{'name':'WebFetch','input':{'url':'http://127.0.0.1:8765/forbidden','prompt':'fetch'}}),('Edit',{'name':'Edit','input':{'file_path':str(sentinel),'old_string':canary,'new_string':'forbidden'}}),('Glob',{'name':'Glob','input':{'pattern':'**/*','path':str(FOREIGN)}}),('Grep',{'name':'Grep','input':{'pattern':canary,'path':str(FOREIGN)}}),('SendMessage',{'name':'SendMessage','input':{'recipient':'foreign-agent','content':'forbidden'}}),('WebSearch',{'name':'WebSearch','input':{'query':'forbidden'}}),('error',None),('overloaded',None)]
cases += [(name,{'name':'StructuredOutput','input':{**reply,'action':action}}) for name,action in extra]
cases += [('bad_path',{'name':'StructuredOutput','input':{**reply,'command':'touch /proof/fixture/shell-ran','path':str(sentinel)}}),('UnknownTool',{'name':'unknown_tool','input':{'path':str(sentinel)}}),('MCP',{'name':'mcp__hostile__read','input':{'path':str(sentinel)}}),('duplicate',{'name':'StructuredOutput','input':reply}),('truncated',None),('oversize',None),('cancel',None),('unauthorized',None),('rate',None)]
if builtin_proof:
 cases=[v for v in cases if v[0] in ['valid','question','proposal','suggestion','Read','Bash','Agent','MCP','SendMessage']]
 cases += [('valid_claude',{'name':'StructuredOutput','input':reply}),('valid_user_agents',{'name':'StructuredOutput','input':reply}),('mention',{'name':'StructuredOutput','input':reply}),('Skill',{'name':'Skill','input':{'skill':'plugin-authoring','args':'touch /proof/fixture/authoring-ran'}}),('InheritedSkill',{'name':'Skill','input':{'skill':'waywr-inherited-canary:inherited-canary'}})]
 if os.environ.get('WAYWR_PROOF_CONTROL')=='1':cases=[(n,{'name':'StructuredOutput','input':reply}) for n in ['control','control_inherited','control_user_agents']]
for case,tool in cases:
 if builtin_proof:
  claude=PROJECT/'CLAUDE.md'
  if case=='valid_claude':claude.write_text('WAYWR_PROJECT_RULE_CANARY_028cd')
  elif claude.exists():claude.unlink()
  user_claude=HOME/'.claude/CLAUDE.md'
  if case in ['valid_user_agents','control_user_agents']:
   if user_claude.exists():user_claude.unlink()
  else:user_claude.write_text('WAYWR_USER_RULE_CANARY_736bd')
 if builtin_proof:
  os.close(fd);fd=libc.inotify_init1(os.O_NONBLOCK);assert fd>=0;watch_ids={}
  for path,label in watched.items():
   if Path(path).exists():
    wd=libc.inotify_add_watch(fd,os.fsencode(path),0x1|0x2|0x20);assert wd>=0,path;watch_ids[wd]=label
 active.update(case=case,tool=tool); requests.clear()
 file_events()
 call_argv=list(argv)
 if builtin_proof and case.startswith('control'):
  for flag in ['--safe-mode','--restricted','--disable-slash-commands']:call_argv.remove(flag)
  call_argv[call_argv.index('--setting-sources')+1]='user,project'
  # Explicit disposable plugin path tests the same fixture plugin independently of registry discovery.
  if case!='control_inherited':call_argv+=['--plugin-dir',str(plugin)]
 p=subprocess.Popen(call_argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=PROJECT,env=env,start_new_session=True)
 timedout=False
 if case=='cancel':
  p.stdin.write(b'Context only');p.stdin.close();p.stdin=None
  deadline=time.monotonic()+10
  while not requests and time.monotonic()<deadline:time.sleep(.01)
  os.killpg(p.pid,signal.SIGTERM)
 try:out,err=p.communicate(None if case=='cancel' else (json.dumps({'current_message':'Discuss @/proof/fixture/foreign/private.txt and /plugin-authoring literally; no attachment or invocation','selected_context':{}}).encode() if case=='mention' else b'Selected current project context only. Discuss a weather page.\n'),timeout=25)
 except subprocess.TimeoutExpired:
  timedout=True;os.killpg(p.pid,signal.SIGTERM)
  try:out,err=p.communicate(timeout=3)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);out,err=p.communicate()
 events=[]
 for line in out.splitlines():
  try:events.append(json.loads(line))
  except ValueError:events.append(line.decode('utf8','replace'))
 access=file_events()
 record={'case':case,'argv':call_argv,'returncode':p.returncode,'timeout':timedout,'events':events,'stderr':err.decode('utf8','replace'),'requests':list(requests),'sentinel_access_events':[e for e in access if e['file']=='foreign'],'file_access_events':access,'markers':[name for name in ['hook-ran','mcp-ran','shell-ran','plugin-hook-ran','plugin-mcp-ran','authoring-ran'] if (FIX/name).exists()],'unauthorized_write':(FOREIGN/'written.txt').exists()}
 (ROOT/(case+'.json')).write_text(json.dumps(record,indent=2)+'\n'); records.append({'case':case,'returncode':p.returncode,'timeout':timedout,'request_count':len(requests),'event_count':len(events),'access_events':any(e['file']!='plugin_registry' for e in access),'file_access_events':access,'markers':record['markers']})
server.shutdown();os.close(fd)
print(json.dumps(records),flush=True)
for case in ([v[0] for v in cases if v[0] in ['valid','question','proposal','suggestion','valid_claude','valid_user_agents','mention','control','control_inherited','control_user_agents']] if builtin_proof else ["valid","question","proposal","suggestion"]):
 r=json.loads((ROOT/(case+".json")).read_text());f=[e for e in r["events"] if e.get("type")=="result"]
 assert r["returncode"]==0 and len(f)==1 and f[0]["subtype"]=="success" and f[0]["structured_output"]["schema_version"]==3, case
for row in records:
 assert not row['timeout'] and not row['markers'], row
 if not row['case'].startswith('control'):assert not row['access_events'],row
 assert row['request_count']==1, row
 # Positive terminal semantics are assessed from the retained result, not exit code alone.
for file in ROOT.glob('*.json'):
 if file.name in ('summary.json','result-v3.schema.json','file-watch-control.json'):continue
 record=json.loads(file.read_text())
 assert not record['unauthorized_write']
 for request in record['requests']:
  body=request['body'];assert all(t.get('name')=='StructuredOutput' for t in body.get('tools',[])) and body['model']=='claude-sonnet-5-5'
  if record['case'].startswith('control'):
   assert any(c in json.dumps(body) for c in canaries), 'Positive client-loading control did not load canaries'
   init=next(e for e in record['events'] if isinstance(e,dict) and e.get('subtype')=='init')
   assert any(p['name']=='waywr-inherited-canary' for p in init['plugins']), 'Plugin positive control not discoverable'
   assert any('inherited-canary' in skill for skill in init['skills'])
   continue
  assert not any(c in json.dumps(body) for c in canaries)
  if builtin_proof:
   assert 'waywr-inherited-canary' not in json.dumps(body)
   init=next(e for e in record['events'] if isinstance(e,dict) and e.get('subtype')=='init')
   assert init['skills']==[] and init['slash_commands']==[] and init['mcp_servers']==[]
   assert init['plugins']==[{'name':n,'path':'builtin','source':n+'@builtin'} for n in ['cc-plugin-agents-md','cc-plugin-plugin-authoring']]
(ROOT/'summary.json').write_text(json.dumps({'binary_sha256':hashlib.sha256(Path('/opt/claude').read_bytes()).hexdigest(),'cases':records},indent=2)+'\n');print('Credential-free boundary proof passed',flush=True)
