"""Explicit test-only subprocess entry point. Never defaults to production state.
Fault hooks live here, not in shipped worker/workflow code. No native providers.
"""
import argparse
import functools
import json
from pathlib import Path
import sys
import time


def barrier(root, point):
    from agent_workflow.storage import atomic, identity
    directory=root/'faults';directory.mkdir(exist_ok=True)
    atomic(directory/(point+'.reached'), {'process':identity(),'point':point}, immutable=True)
    deadline=time.monotonic()+45
    while not (directory/(point+'.release')).exists():
        if time.monotonic()>deadline:raise RuntimeError('Test barrier timeout: '+point)
        time.sleep(.02)


def main():
    p=argparse.ArgumentParser()
    p.add_argument('--state',type=Path,required=True)
    p.add_argument('--marker')
    p.add_argument('action',choices=['prepare','probe','resume','answer','approve','status','runner'])
    p.add_argument('--run');p.add_argument('--socket');p.add_argument('--workspace');p.add_argument('--fault')
    p.add_argument('--seconds',type=float,default=.3)
    args,rest=p.parse_known_args()
    root=args.state.resolve()
    if not (root/'m3-test-state.json').is_file():raise RuntimeError('Explicit isolated state marker required')
    from agent_workflow import storage
    if root==storage.LOCAL.resolve():raise RuntimeError('Production state forbidden')
    storage.LOCAL=root
    from agent_workflow import herdr_backend as backend, records, workflow, artifacts
    from agent_workflow.storage import atomic, read, canonical, digest
    if args.action=='runner':
        from agent_workflow import runner
        fault=read(root/'runner-fault.json')['point'] if (root/'runner-fault.json').exists() else None
        original=runner.atomic
        def runner_atomic(path,value,**kw):
            point=Path(path).stem
            if fault==point+'-before':barrier(root,fault)
            result=original(path,value,**kw)
            if fault==point+'-after':barrier(root,fault)
            return result
        runner.atomic=runner_atomic
        sys.argv=['test-runner',*(rest[1:] if rest[:1]==['--'] else rest)]
        return runner.main()
    real_create=backend.Herdr.create
    def create(self,workspace,label,command,cwd):
        assert command[1:3]==['-m','agent_workflow.runner']
        command=[sys.executable,str(Path(__file__).resolve()),'--state',str(root),
                 '--marker','agent_workflow.runner','runner','--',*command[3:]]
        result=real_create(self,workspace,label,command,cwd)
        if args.fault=='create-after':barrier(root,args.fault)
        return result
    backend.Herdr.create=create
    original_atomic=backend.atomic
    def controller_atomic(path,value,**kw):
        point=Path(path).stem
        if args.fault==point+'-before':barrier(root,args.fault)
        result=original_atomic(path,value,**kw)
        if args.fault==point+'-after':barrier(root,args.fault)
        return result
    backend.atomic=controller_atomic
    original_publish=artifacts.publish
    def publish(*a,**kw):
        value=original_publish(*a,**kw)
        if args.fault=='artifact-after':barrier(root,args.fault)
        return value
    artifacts.publish=publish
    for name,point in [('choice_gate','choice-after'),('final_gate','approval-after')]:
        original=getattr(workflow,name)
        def wrap(function,point):
            @functools.wraps(function)
            def gate(state):
                value=function(state)
                if args.fault==point:barrier(root,point)
                return value
            return gate
        setattr(workflow,name,wrap(original,point))
    if args.action=='prepare':
        target={'socket':args.socket,'workspace':args.workspace}
        config={'roles':{r:{'provider':'fake','model':'deterministic-m3'} for r in ('author','critic')},
                'fake':{'seconds':args.seconds},'review_limit':2}
        env={'schema_version':1,'workflow_version':1,'run_id':args.run,'thread_id':args.run,
             'brief':'Simulated recovery fixture; responses are automated test data, not human approval.',
             'instructions':[],'skills':dict.fromkeys(['draft','critique','revise'],'deterministic test'),
             'rules_sha256':digest('[]'),'config_sha256':digest(canonical(config)),
             'target':target,'config':config}
        records.create_run(env);atomic(records.run_dir(args.run)/'inputs/envelope.json',env,immutable=True)
    elif args.action=='probe':
        directory=root/'probes'/args.run
        packet={'schema_version':1,'operation_id':args.run,'run_id':args.run,'stage':'draft',
                'role':'author','provider':'fake','model':'deterministic-m3','seconds':args.seconds}
        backend.prepare(directory,packet,{'socket':args.socket,'workspace':args.workspace})
        print(json.dumps(backend.wait(directory)))
    elif args.action=='resume':
        with storage.lock(root/'controller.lock'):workflow.drive(args.run)
    elif args.action in ('answer','approve'):
        s=workflow.state(args.run).values;request=s['pending']
        with storage.lock(root/'controller.lock'):
            workflow.answer(args.run,request['request_id'],
                            'answered' if args.action=='answer' else 'approved',
                            '7 days' if args.action=='answer' else 'Automated milestone-3 fixture approval',
                            None if args.action=='answer' else request['artifact_sha256'])
    elif args.action=='status':
        from agent_workflow.cli import summary
        print(json.dumps(summary(args.run)))

if __name__=='__main__':
    try:
        result=main()
    except Exception as e:
        print(type(e).__name__+': '+str(e),file=sys.stderr,flush=True)
        raise SystemExit(3)
    raise SystemExit(result or 0)
