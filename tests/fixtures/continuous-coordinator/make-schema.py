import json
from pathlib import Path
string={'type':'string'}
proposal={'type':'object','properties':{k: string if k in ('title','objective') else {'type':'array','items':string} for k in ('title','objective','deliverables','in_scope','out_of_scope','constraints','acceptance_criteria','unresolved_decisions')},'required':['title','objective','deliverables','in_scope','out_of_scope','constraints','acceptance_criteria','unresolved_decisions'],'additionalProperties':False}
def obj(properties):return {'type':'object','properties':properties,'required':list(properties),'additionalProperties':False}
base=obj({'task_id':string,'revision':{'type':'integer'},'sha256':string})
def action(kind,payload):return obj({'kind':{'type':'string','const':kind},'payload':payload})
schema=obj({'schema_version':{'type':'integer','const':3},'assistant_text':string,'evidence_refs':{'type':'array','items':string},'action':{'anyOf':[{'type':'null'},action('ask_question',obj({'prompt':string,'rationale':string,'options':{'type':'array','items':string}})),action('upsert_proposal',obj({'base':{'anyOf':[{'type':'null'},base]},'proposal':proposal})),action('suggest_question_disposition',obj({'question_id':string,'question_revision':{'type':'integer'},'disposition':{'enum':['answer','defer']},'answer':string,'source_message_id':string,'source_sha256':string,'quote_start':{'type':'integer'},'quote_end':{'type':'integer'}}))]}})
Path(__file__).with_name('result-v3.schema.json').write_text(json.dumps(schema,indent=2)+'\n')
