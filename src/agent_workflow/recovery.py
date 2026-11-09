"""Explicit adoption of one authorized saved output. Never launches a native CLI."""
import getpass
import json
from pathlib import Path
from . import records, artifacts, context
from .storage import LOCAL, ROOT, OperationalError, atomic, canonical, digest, read, alive, now

RUN='context-01'
STAGE='draft'
CORRECTION='recommendation-prose-v1: recommendation is bounded advice, not exact option-label equality'
FAILURE='Question needs 2–3 distinct options and a matching recommendation'
# Pins identify the exact output whose recovery the operator authorized.
AUTHORIZED={
 'inspection':'8f59761fdaf2839ab0f4168d126ce259116fd8ab69c699d3bc14e3f0be566fc0',
 'native':'15fe5745b7da261e7d5d4eb253e5063f6499725d38f37c84071e21e7a016fde4',
 'output':'70d12044908269c903396742c19a3aa90cd78889b83fa667ab56dd89e5a80a79',
}
FILES=('operation.json','packet.json','prompt.txt','output-schema.json','execution.json',
       'receipt.json','claim.json','permit.json','dispatch.json','child.json',
       'binding.json','invocations.json','native-events.jsonl','runtime-result.json')


def require(condition, message):
    if not condition: raise OperationalError('Saved-output adoption refused: '+message)


def schema_check(value, schema):
    """Validate the small type/enum/object/array subset emitted by our frozen schema."""
    types={'object':dict,'array':list,'string':str,'integer':int,'boolean':bool}
    require(schema.get('type') in types and type(value) is types[schema['type']], 'frozen schema type mismatch')
    if 'enum' in schema: require(value in schema['enum'],'frozen schema enum mismatch')
    if schema['type']=='object':
        require(set(value)==set(schema['required'])==set(schema['properties']), 'frozen schema fields mismatch')
        require(schema.get('additionalProperties') is False,'unexpected permissive schema')
        for key,item in value.items():schema_check(item,schema['properties'][key])
    elif schema['type']=='array':
        for item in value:schema_check(item,schema['items'])


def verify(directory):
    from .adapters import real
    from .herdr_backend import validate_result
    directory=Path(directory)
    require(directory.resolve()==(records.run_dir(RUN)/'operations'/STAGE).resolve(),'only context-01/draft is authorized')
    env=records.get_run(RUN)['envelope'];context.check(env)
    require(read(records.run_dir(RUN)/'inputs/envelope.json')==env,'frozen envelope differs from canonical run')
    inspection=records.state_dir()/'evidence/context-rejected-output.json'
    require(digest(inspection.read_bytes())==AUTHORIZED['inspection'],'inspection evidence differs from authorized snapshot')
    evidence=read(inspection)
    hashes={name:digest((directory/name).read_bytes()) for name in FILES}
    require(hashes['native-events.jsonl']==AUTHORIZED['native']==evidence['native_events_sha256'],'native output integrity mismatch')
    op=read(directory/'operation.json');packet=read(directory/'packet.json');failed=read(directory/'receipt.json')
    require(op['run_id']==RUN and op['stage']==STAGE and op['operation_id']==RUN+'-'+STAGE and op['provider']=='claude','original operation identity mismatch')
    require(packet['operation_id']==op['operation_id'] and packet['run_id']==RUN and packet['stage']==STAGE and packet['schema_version']==2,'frozen packet identity mismatch')
    require(digest(canonical(packet))==op['packet_sha256'],'packet hash mismatch')
    reservations=records.operations(RUN)
    require(any(x['stage']==STAGE and x['operation_id']==op['operation_id'] and x['packet_hash']==op['packet_sha256'] for x in reservations),'missing matching stage reservation')
    require(failed==evidence['original_receipt'],'original failure was changed')
    require(failed['status']=='failed' and failed['exit_code']==0 and failed['error']==FAILURE,'failure is not the authorized validation defect')
    require(all(failed[k]==op[k] for k in ('operation_id','packet_sha256','launch_token')),'receipt operation mismatch')
    claim=read(directory/'claim.json');permit=read(directory/'permit.json');dispatch=read(directory/'dispatch.json');binding=read(directory/'binding.json')
    require(failed['runner']==claim['runner']==permit['runner']==dispatch['runner'],'runner identities disagree')
    require(permit['launch_token']==op['launch_token'] and permit['binding']==binding,'dispatch permit mismatch')
    require(all(binding[k]==v for k,v in claim['herdr'].items()),'Herdr binding mismatch')
    count=read(directory/'invocations.json')
    require(count['count']==1 and count['operation_id']==op['operation_id'],'original invocation count mismatch')
    require(not alive(read(directory/'child.json')['identity']),'native process still alive')
    require(not any((directory/f).exists() for f in ('uncertain.json','cancel.json')),'uncertain/cancelled execution cannot be adopted')
    execution=read(directory/'execution.json');schema=read(directory/'output-schema.json')
    require(execution['provider']=='claude' and execution['packet_sha256']==op['packet_sha256'],'execution identity mismatch')
    require(execution['prompt_sha256']==hashes['prompt.txt'],'frozen prompt hash mismatch')
    require((directory/'prompt.txt').read_text()==real.prompt(packet),'prompt does not encode exact frozen packet')
    require(schema==real.schema(packet),'schema does not encode exact frozen identity')
    argv=execution['argv'];require('--json-schema' in argv,'missing original structured schema flag')
    require(json.loads(argv[argv.index('--json-schema')+1])==schema,'executed schema differs from saved schema')
    events=[json.loads(line) for line in (directory/'native-events.jsonl').read_text().splitlines() if line.strip()]
    for event in events:real.inspect_event('claude',event)
    finals=[e for e in events if e.get('type')=='result']
    require(len(finals)==1 and finals[0].get('subtype')=='success' and not finals[0].get('is_error'),'native completion missing or unsuccessful')
    final=finals[0];output=final.get('structured_output')
    require(output==evidence['structured_output'] and digest(canonical(output))==AUTHORIZED['output'],'saved structured output differs')
    require(json.loads(final['result'])==output,'native final representations disagree')
    schema_check(output,schema);validate_result(output,packet)
    require(len(output['questions'])==1,'authorized recovery must reach the original single question')
    return output,hashes,failed


def effective_receipt(directory, failed):
    """Completion evidence overlays the immutable failure; does not rewrite it."""
    directory=Path(directory);path=directory/'recovery'/'adoption.json'
    if not path.exists():return failed
    output,hashes,original=verify(directory);adoption=read(path)
    require(adoption['operation_id']==original['operation_id'] and adoption['evidence_hashes']==hashes and
            adoption['output_sha256']==AUTHORIZED['output'] and adoption['correction']==CORRECTION and
            adoption['result']=='adopted','recovery record no longer matches evidence')
    require(read(directory/'result.json')==output and digest((directory/'result.json').read_bytes())==adoption['output_sha256'],'published adopted output changed')
    return {**failed,'status':'completed','result_sha256':adoption['output_sha256'],
            'original_status':'failed','recovery_record':str(path),'recovered':True}


def adopt(run_id):
    """Caller holds controller.lock. Only first adoption may drive to the choice gate."""
    from . import workflow
    require(run_id==RUN,'no other run is authorized')
    other=records.active_run();require(other in (None,RUN),'another active run exists')
    directory=records.run_dir(RUN)/'operations'/STAGE
    output,hashes,failed=verify(directory)
    path=directory/'recovery'/'adoption.json'
    if path.exists():
        effective_receipt(directory,failed)
        # Explicitly do not consume any saved human answer on repeated adoption.
        if (directory/'recovery'/'published.json').exists():return read(path)
    else:
        snapshot=workflow.state(RUN)
        require(records.get_run(RUN)['status']=='failed' and snapshot.values['next_stage']=='draft' and
                not snapshot.values['completed'] and snapshot.next==('worker',),'run is not at original failed draft boundary')
        expected=artifacts.packet(records.get_run(RUN)['envelope'],snapshot.values,STAGE)
        require(expected==read(directory/'packet.json'),'replayed graph input differs from frozen packet')
        atomic(directory/'result.json',output,immutable=True)
        atomic(path,{'schema_version':1,'operation_id':failed['operation_id'],'output_sha256':AUTHORIZED['output'],
                     'evidence_hashes':hashes,'original_failure':failed,'correction':CORRECTION,
                     'validator_sha256':digest(Path(context.__file__).read_bytes()),
                     'result':'adopted','recorded_by':getpass.getuser(),'time':now(),
                     'authorization':'Operator explicitly authorized exact saved context-01 draft adoption; no new native call'},immutable=True)
    snapshot=workflow.state(RUN)
    if not snapshot.interrupts:
        require(snapshot.values['next_stage']=='draft' and not snapshot.values['completed'],'unexpected continuation boundary')
        records.set_status(RUN,'paused','Explicit saved-output adoption; original failed receipt preserved')
        workflow.drive(RUN)  # LangGraph retries the node, receipt reuse prevents native dispatch.
        snapshot=workflow.state(RUN)
    require(snapshot.interrupts and snapshot.values.get('pending',{}).get('kind')=='choice','adoption did not reach durable human question')
    require(snapshot.values['completed']==['draft'],'unexpected work beyond adoption')
    require(records.response(RUN,snapshot.values['pending']['request_id']) is None,'unexpected human response')
    atomic(directory/'recovery'/'published.json',{'artifact':snapshot.values['current'],
           'request':snapshot.values['pending'],'adoption_sha256':digest(path.read_bytes()),'result':'published-and-paused'},immutable=True)
    return read(path)
