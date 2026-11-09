#!/usr/bin/python3
"""Scripted transport scenario, not natural-language interpretation; no network/agents."""
import json,sys,time
p=json.load(sys.stdin);ctx=p['selected_context'];scope=ctx['scope'];model=sys.argv[sys.argv.index('--model')+1]
n=ctx['budgets']['conversation']['charged']
proposal={'title':'Fixed-sample local weather page','objective':'Display weather from fixed sample data without external requests','deliverables':['One static HTML page and sample data description'],'in_scope':['City selection','Celsius/Fahrenheit toggle'],'out_of_scope':['Live weather API','Backend server'],'constraints':['No network or API keys'],'acceptance_criteria':['Selecting a sample city shows its fixed temperature'],'unresolved_decisions':['Final accent color']}
if scope['proposal']:
 proposal=dict(scope['proposal']['record']['proposal']);proposal['constraints']=proposal['constraints']+[f'Mock refinement from turn {n}']
action=None
if n==2:action={'kind':'ask_question','payload':{'prompt':'Should the sample page include a Celsius/Fahrenheit toggle?','rationale':'This affects the saved scope.','options':['Celsius only','Include the toggle']}}
elif n in (3,4,6):action={'kind':'upsert_proposal','payload':{'base':scope['base'],'proposal':proposal}}
text={1:'Mock transport: discussion only. Send a second message for the scripted clarification.',2:'Mock transport: a scope choice needs your answer.',3:'Mock transport: the proposal is saved for review. No work can execute.',4:'Mock transport: here is a revision to review.',5:'Mock transport: that message grants no authority; use the explicit scope action.',6:'Mock transport: another material change requires fresh authorization.'}.get(n,'Mock transport response.')
r={'schema_version':2,'assistant_text':text,'evidence_refs':[],'action':action}
def emit(v):print(json.dumps(v),flush=True)
emit({'type':'system','subtype':'init','tools':[],'skills':[],'mcp_servers':[],'permissionMode':'dontAsk','model':model})
emit({'type':'stream_event','event':{'delta':{'type':'thinking_delta','thinking':'PRIVATE_SCOPE_REASONING'}}})
encoded=json.dumps(r)
if p["current_message"]=="fixture-malformed":encoded=json.dumps({**r,"authorization":True})
for part in (encoded[:32],encoded[32:70],encoded[70:]):
 time.sleep(.45);emit({'type':'stream_event','event':{'delta':{'type':'text_delta','text':part}}})
emit({'type':'result','subtype':'success','is_error':False,'stop_reason':'end_turn','result':encoded,'usage':{'input_tokens':20,'output_tokens':30},'modelUsage':{model:{'inputTokens':20,'outputTokens':30}},'total_cost_usd':0})
