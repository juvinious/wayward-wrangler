"""One authorized trial: visible controller exits, records answer, then resumes.

Only controls process lifetime. Never supplies or synthesizes human input.
"""
import os
import sys
import subprocess
import time
from . import records, workflow
from .storage import ROOT, atomic, identity, read, now, OperationalError


def snapshot(run):
    state=workflow.state(run).values
    pending=state.get('pending')
    operations={p.name:{key:read(p/(key+'.json')) for key in ('invocations','binding','receipt') if (p/(key+'.json')).exists()}
                for p in (records.run_dir(run)/'operations').glob('*') if p.is_dir()}
    return {'time':now(),'state':state,'operations':operations,
            'answer':records.response(run,pending['request_id']) if pending else None}


def main():
    run='context-01'
    if not os.environ.get('HERDR_PANE_ID'): raise OperationalError('Run inside Herdr')
    target=records.effective_target(run)
    recovered='--after-adoption' in sys.argv[1:]
    if recovered:
        from .recovery import effective_receipt
        d=records.run_dir(run)/'operations/draft'
        effective_receipt(d,read(d/'receipt.json'))
        if not (d/'recovery/published.json').exists(): raise OperationalError('Adoption must first reach its durable question')
    directory=records.run_dir(run)/('recovery-gate-restart-evidence' if recovered else 'gate-restart-evidence')
    base=[str(ROOT/'.venv/bin/wayward-wrangler'),'demo','--run',run,'--workspace',target['workspace']]
    for phase,extra in [('1-unanswered-exit',['--exit-at-choice']),
                        ('2-record-answer',['--record-choice-and-exit']),('3-consume-answer',[])]:
        if (directory/(phase+'-start.json')).exists():
            raise OperationalError('Demonstration already started; inspect saved phase, do not blindly relaunch')
        print(f'\nController phase {phase}. No answer is supplied automatically.',flush=True)
        before=snapshot(run)
        child=subprocess.Popen(base+extra)
        atomic(directory/(phase+'-start.json'),{'controller':identity(child.pid),'before':before},immutable=True)
        code=child.wait()
        after=snapshot(run)
        atomic(directory/(phase+'-exit.json'),{'exit_code':code,'after':after},immutable=True)
        if phase=='1-unanswered-exit':
            pending=after['state'].get('pending')
            if code!=2 or not pending or pending['kind']!='choice' or after['answer'] is not None:
                print('No unanswered agent choice reached. Stopped without retry.',flush=True);break
            print('First controller exited at the unanswered gate. Starting a fresh controller for your input.',flush=True)
        elif phase=='2-record-answer':
            if code!=2 or not after['answer']:
                print('No answer recorded. Stopped; use the documented resume command when ready.',flush=True);break
            if before['operations']!=after['operations']:
                raise OperationalError('Unexpected operation change while recording answer')
            print('Answer persisted and controller exited. Starting a fresh controller to consume it.',flush=True)
        else: break
    print('Controller demonstration output retained; no automatic retry.',flush=True)
    while True: time.sleep(2)


if __name__=='__main__':
    try: main()
    except KeyboardInterrupt: print('Controller demonstration interrupted; inspect state before resuming.')
