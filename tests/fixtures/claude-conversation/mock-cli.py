#!/usr/bin/python3
"""Credential-free CLI transport fixture. Never invokes a provider."""
import json,sys,time
packet=json.load(sys.stdin)
def emit(v): print(json.dumps(v),flush=True)
model=sys.argv[sys.argv.index('--model')+1]
emit({'type':'system','subtype':'init','tools':[],'skills':[],'mcp_servers':[],'permissionMode':'dontAsk','model':model})
text='Mock transport received your exact message: '+packet['current_message']
emit({'type':'stream_event','event':{'delta':{'type':'thinking_delta','thinking':'PRIVATE_REASONING_CANARY'}}})
for part in [text[:24],text[24:]]:
 time.sleep(.7)
 emit({'type':'stream_event','event':{'delta':{'type':'text_delta','text':part}}})
if packet['current_message']=='fixture-incomplete': sys.exit(0)
emit({'type':'result','subtype':'success','is_error':False,'stop_reason':'end_turn','result':text,'usage':{'input_tokens':10,'output_tokens':20},'modelUsage':{model:{'inputTokens':10,'outputTokens':20}},'total_cost_usd':0})
