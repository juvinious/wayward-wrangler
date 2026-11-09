#!/usr/bin/python3
"""Explicit offline scripted CLI transport, never a natural-language coordinator."""
import json,sys,time
p=json.load(sys.stdin);ctx=p['selected_context'];scope=ctx['scope'];model=sys.argv[sys.argv.index('--model')+1];n=ctx['budgets']['conversation']['charged'];message=p['current_message']
def emit(v): print(json.dumps(v),flush=True)
proposal={'title':'Fixed-sample weather page','objective':'Display weather from fixed sample data','deliverables':['Static index.html description'],'in_scope':['City selection'],'out_of_scope':['API and execution'],'constraints':['No network'],'acceptance_criteria':['Shows fixed samples'],'unresolved_decisions':[]}
if scope['proposal']:
 proposal=dict(scope['proposal']['record']['proposal']);proposal['constraints']=proposal['constraints']+[f'Mock scope revision {n}']
questions=scope['questions'];action=None;refs=[]
if n==2: action={'kind':'ask_question','payload':{'prompt':'Should the page show Celsius or Fahrenheit?','rationale':'An optional scope choice','options':['Celsius','Fahrenheit']}}
elif n==3 and questions:
 q=questions[0];source=ctx['current_message'];action={'kind':'suggest_question_disposition','payload':{'question_id':q['question_id'],'question_revision':q['revision'],'disposition':'answer','answer':message,'source_message_id':source['message_id'],'source_sha256':source['text_sha256'],'quote_start':0,'quote_end':len(message.encode())}}
elif n in (4,6):
 refs=['question:'+q['question_id'] for q in questions if q['status'] in ('answered','deferred')]
 proposal['unresolved_decisions']=[q['prompt'] for q in questions if q['status']=='deferred']
 action={'kind':'upsert_proposal','payload':{'base':scope['base'],'proposal':proposal}}
r={'schema_version':3,'assistant_text':'Mock transport: response to '+message+'. This is scripted test data; only Waywr reports saves.','evidence_refs':refs,'action':action}
if message=='fixture-proposal':r['action']={'kind':'upsert_proposal','payload':{'base':scope['base'],'proposal':proposal}}
if message=='fixture-format':r['forbidden']=True
emit({'type':'system','subtype':'init','tools':['StructuredOutput'],'skills':[],'mcp_servers':[],'permissionMode':'dontAsk','model':model,'claude_code_version':'2.1.296','slash_commands':[],'plugins':[{'name':name,'path':'builtin','source':name+'@builtin'} for name in ['cc-plugin-agents-md','cc-plugin-plugin-authoring']]})
emit({'type':'stream_event','event':{'type':'content_block_start','index':0,'content_block':{'type':'tool_use','name':'StructuredOutput','input':{}}}})
encoded=json.dumps(r)
for part in (encoded[:70],encoded[70:]):
 time.sleep(6 if message=='fixture-long' else .5);emit({'type':'stream_event','event':{'index':0,'delta':{'type':'input_json_delta','partial_json':part}}})
emit({'type':'stream_event','event':{'type':'content_block_stop','index':0}})
emit({'type':'stream_event','event':{'type':'message_stop'}})
emit({'type':'result','subtype':'success','is_error':False,'stop_reason':'tool_use','structured_output':r,'result':'','usage':{'input_tokens':20,'output_tokens':30},'modelUsage':{model:{'inputTokens':20,'outputTokens':30}},'total_cost_usd':0})
