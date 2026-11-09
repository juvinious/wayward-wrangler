"""Opt-in isolated Herdr fault matrix; never uses the active server or native CLIs.
Usage: .venv/bin/python tests/milestone3.py --evidence-root NEW_DIRECTORY
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid
from agent_workflow.herdr_backend import Herdr, receipt, inspect_operation
from agent_workflow.storage import atomic, read, identity, alive, digest

ACTOR=Path(__file__).with_name('m3_actor.py').resolve()


def eventually(check, timeout=15):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        value=check()
        if value:return value
        time.sleep(.03)
    raise AssertionError('Timed out waiting for test-owned evidence')


def signal_owned(expected, sig):
    """Pin the process with pidfd, then verify boot/start before signaling it."""
    if not alive(expected):return
    fd=os.pidfd_open(expected['pid'])
    try:
        actual=identity(expected['pid'])
        assert all(actual[k]==expected[k] for k in ('pid','boot','start'))
        signal.pidfd_send_signal(fd,sig)
    finally:os.close(fd)


class Suite:
    def __init__(self,root):
        self.root=root;root.mkdir(parents=True,exist_ok=False)
        self.base=Path(tempfile.mkdtemp(prefix='ww-m3-'))
        for d in ('home','config/herdr','runtime'):(self.base/d).mkdir(parents=True,exist_ok=True)
        (self.base/'config/herdr/config.toml').write_text('onboarding = false\nshell = "/bin/sh"\n')
        self.socket=str(self.base/'runtime/test.sock')
        self.env={'PATH':os.environ['PATH'],'HOME':str(self.base/'home'),
                  'XDG_CONFIG_HOME':str(self.base/'config'),'XDG_RUNTIME_DIR':str(self.base/'runtime'),
                  'HERDR_SOCKET_PATH':self.socket,'SHELL':'/bin/sh','TERM':'xterm-256color','LANG':'C.UTF-8'}
        self.children=[];self.checks=[];self.server=None;self.counter=0
        atomic(root/'isolation.json',{'base':str(self.base),'socket':self.socket,'harness':identity(),
                                    'fixture_responses':'Automated test data only; not operator approval'},immutable=True)

    def start_server(self):
        self.counter+=1
        log=(self.root/f'server-{self.counter}.log').open('wb')
        self.server=subprocess.Popen(['herdr','server'],env=self.env,stdout=log,stderr=subprocess.STDOUT)
        log.close();self.server_id=identity(self.server.pid);self.h=Herdr(self.socket)
        def ready():
            assert self.server.poll() is None,'Owned server exited'
            try:return self.h.server_identity()
            except OSError:return False
        peer=eventually(ready);assert peer['pid']==self.server.pid
        atomic(self.root/f'server-{self.counter}.json',peer,immutable=True)
        self.workspace=self.h.call('workspace.create',{'label':'Wayward Wrangler M3 isolated',
                          'cwd':str(self.root),'focus':False})['workspace']['workspace_id']

    def stop_server(self):
        if self.server and self.server.poll() is None:
            peer=self.h.server_identity()
            assert all(peer[k]==self.server_id[k] for k in ('pid','boot','start'))
            self.h.call('server.stop');self.server.wait(timeout=10)
        self.server=None

    def case(self,name):
        root=self.root/name;root.mkdir();atomic(root/'m3-test-state.json',{'case':name},immutable=True)
        return root,'m3-'+name+'-'+uuid.uuid4().hex[:8]

    def actor(self,root,run,action,*,fault=None,wait=True,seconds=.3):
        cmd=[sys.executable,str(ACTOR),'--state',str(root),action,'--run',run,
             '--socket',self.socket,'--workspace',self.workspace,'--seconds',str(seconds)]
        if fault:cmd+=['--fault',fault]
        n=len(self.children);log=root/f'controller-{n}.log'
        stream=log.open('wb');p=subprocess.Popen(cmd,stdout=stream,stderr=subprocess.STDOUT)
        stream.close();entry=(p,identity(p.pid),log);self.children.append(entry)
        if wait:
            code=p.wait(timeout=25)
            assert code==0,log.read_text()
            return log.read_text()
        return entry

    def reached(self,root,point):return eventually(lambda:(root/'faults'/f'{point}.reached').exists())
    def release(self,root,point):atomic(root/'faults'/f'{point}.release',{'release':True},immutable=True)
    def kill_controller(self,entry):
        p,who,_=entry;signal_owned(who,signal.SIGKILL);assert p.wait(timeout=5)==-signal.SIGKILL
    def done(self,op):return eventually(lambda: receipt(op))
    def snapshot(self,root,op):
        report=inspect_operation(op)
        atomic(root/'diagnostic.json',report,immutable=True)
        return report
    def count(self,op):return read(op/'invocations.json')['count'] if (op/'invocations.json').exists() else 0
    def pass_case(self,name,**values):
        self.checks.append({'case':name,'passed':True,**values})
        atomic(self.root/'progress.json',self.checks)
        print('PASS',name,flush=True)

    def dispatch(self):
        for fault in ('launch-before','launch-after','create-after','permit-before','permit-after'):
            root,run=self.case(fault);op=root/'probes'/run
            if fault=='create-after':atomic(root/'runner-fault.json',{'point':'claim-before'},immutable=True)
            entry=self.actor(root,run,'probe',fault=fault,wait=False)
            self.reached(root,fault)
            if fault=='create-after':self.reached(root,'claim-before')
            before_claim=(op/'claim.json').exists();before_count=self.count(op)
            if fault in ('create-after','permit-before'):assert before_count==0
            token=read(op/'operation.json')['launch_token']
            panes=self.h.find(token)
            self.kill_controller(entry)
            if fault=='create-after':self.release(root,'claim-before')
            if fault=='launch-after':
                retry=self.actor(root,run,'probe',wait=False)
                assert retry[0].wait(timeout=15)==3,retry[2].read_text()
                assert self.h.find(token)==[] and self.count(op)==0
                diagnostic=self.snapshot(root,op);assert diagnostic['status']=='uncertain'
                self.pass_case(fault,count=0,diagnostic=diagnostic)
                continue
            self.actor(root,run,'probe')
            assert self.done(op)['status']=='completed' and self.count(op)==1
            after=self.h.find(token);assert len(after)==1
            if panes:assert after==panes
            before=(op/'receipt.json').read_bytes()
            self.actor(root,run,'probe')
            assert self.count(op)==1 and (op/'receipt.json').read_bytes()==before
            self.snapshot(root,op)
            self.pass_case(fault,count=1,claim_before_kill=before_claim,panes=after)

    def publication_and_gates(self):
        root,run=self.case('checkpoint');self.actor(root,run,'prepare')
        entry=self.actor(root,run,'resume',fault='artifact-after',wait=False)
        self.reached(root,'artifact-after');op=root/'runs'/run/'operations/draft'
        assert self.count(op)==1 and self.done(op)['status']=='completed'
        artifact=root/'runs'/run/'artifacts/draft.json';published=artifact.read_bytes()
        self.kill_controller(entry)
        before=json.loads(self.actor(root,run,'status'));assert 'draft' not in before['completed']
        atomic(root/'before-replay.json',before,immutable=True)
        self.actor(root,run,'resume')
        after=json.loads(self.actor(root,run,'status'))
        assert after['completed']==['draft','critique-1'] and after['status']=='awaiting-input'
        assert artifact.read_bytes()==published and self.count(op)==1
        self.actor(root,run,'answer')  # explicitly labeled automated fixture, record-only
        response=root/'runs'/run/'artifacts/decision-D-retention.json';answer=response.read_bytes()
        entry=self.actor(root,run,'resume',fault='choice-after',wait=False)
        self.reached(root,'choice-after');self.kill_controller(entry)
        self.actor(root,run,'resume');final=json.loads(self.actor(root,run,'status'))
        assert final['status']=='awaiting-input' and final['pending']['kind']=='final-review'
        assert len(final['pending']['decisions'])==1 and final['pending']['decision_revision']==1
        assert response.read_bytes()==answer and len(final['completed'])==5
        for stage in final['completed']:assert self.count(root/'runs'/run/'operations'/stage)==1
        self.actor(root,run,'approve')
        approval=root/'runs'/run/'artifacts/decision-A1.json';approved=approval.read_bytes()
        entry=self.actor(root,run,'resume',fault='approval-after',wait=False)
        self.reached(root,'approval-after');self.kill_controller(entry)
        self.actor(root,run,'resume');self.actor(root,run,'resume')
        end=json.loads(self.actor(root,run,'status'))
        assert end['status']=='approved' and approval.read_bytes()==approved and response.read_bytes()==answer
        for stage in end['completed']:assert self.count(root/'runs'/run/'operations'/stage)==1
        atomic(root/'final-status.json',end,immutable=True)
        self.pass_case('publication-choice-approval-replay',stages=5,invocations=5,decision_revision=1,
                       artifact_sha256=end['current']['sha256'],fixture_approval=True)

    def orphan(self):
        root,run=self.case('orphan');op=root/'probes'/run
        entry=self.actor(root,run,'probe',wait=False,seconds=30)
        eventually(lambda:(op/'child.json').exists() and self.count(op)==1)
        child=read(op/'child.json')['identity'];runner=read(op/'claim.json')['runner']
        assert alive(child);signal_owned(runner,signal.SIGKILL)
        eventually(lambda:not alive(runner));assert alive(child)
        assert entry[0].wait(timeout=10)==3,entry[2].read_text()
        report=self.snapshot(root,op);assert report['status']=='uncertain' and report['child_alive']
        retry=self.actor(root,run,'probe',wait=False,seconds=30)
        assert retry[0].wait(timeout=10)==3 and self.count(op)==1 and not (op/'receipt.json').exists()
        # No application auto-collection: harness owns this child and has its saved identity.
        signal_owned(child,signal.SIGTERM);eventually(lambda:not alive(child))
        self.pass_case('runner-death-surviving-child',diagnostic=report,child=child,
                       cleanup='pidfd verified test child only',count=1)

    def request_cancel(self,root,op):
        code='from pathlib import Path; from agent_workflow import storage; storage.LOCAL=Path(__import__("sys").argv[1]); from agent_workflow.herdr_backend import cancel; print(cancel(Path(__import__("sys").argv[2])))'
        result=subprocess.run([sys.executable,'-c',code,str(root),str(op)],capture_output=True,text=True,timeout=10)
        assert result.returncode==0,result.stderr
        return result.stdout

    def cancellation(self):
        sentinel_root,sentinel_run=self.case('cancel-sentinel')
        self.actor(sentinel_root,sentinel_run,'probe')
        sentinel=read(sentinel_root/'probes'/sentinel_run/'claim.json')['runner']
        root,run=self.case('cancel-before-permit');op=root/'probes'/run
        entry=self.actor(root,run,'probe',fault='permit-before',wait=False)
        self.reached(root,'permit-before');assert self.count(op)==0
        self.request_cancel(root,op)
        assert self.done(op)['status']=='cancelled'
        self.kill_controller(entry);self.actor(root,run,'probe')
        assert self.count(op)==0 and alive(sentinel)
        self.snapshot(root,op)
        self.pass_case('cancel-before-permit',count=0,unrelated_test_runner_alive=True)
        for point in ('child-after','result-before','receipt-after'):
            root,run=self.case('cancel-'+point);op=root/'probes'/run
            atomic(root/'runner-fault.json',{'point':point},immutable=True)
            entry=self.actor(root,run,'probe',wait=False,seconds=2)
            self.reached(root,point)
            before=receipt(op)
            if before:
                raw=(op/'receipt.json').read_bytes()
            self.request_cancel(root,op)
            self.release(root,point)
            done=self.done(op);assert done['status']==('cancelled' if point=='child-after' else 'completed')
            if before:assert (op/'receipt.json').read_bytes()==raw
            assert self.count(op)==1 and entry[0].wait(timeout=10)==0
            self.snapshot(root,op)
            assert alive(sentinel)
            pane=self.h.find(read(op/'operation.json')['launch_token'])[0]
            output=self.h.call('pane.read',{'pane_id':pane['pane_id'],'source':'recent','lines':80})
            atomic(root/'pane-output.json',output,immutable=True)
            self.pass_case('cancel-'+point,outcome=done['status'],count=1,unrelated_test_runner_alive=True)

    def server_change(self):
        root,run=self.case('server-change');op=root/'probes'/run
        entry=self.actor(root,run,'probe',fault='permit-before',wait=False)
        self.reached(root,'permit-before');self.kill_controller(entry)
        assert self.count(op)==0
        original=read(op/'launch.json')['server'];self.stop_server()
        missing=self.actor(root,run,'probe',wait=False)
        assert missing[0].wait(timeout=10)==3 and self.count(op)==0
        self.start_server()
        changed=self.actor(root,run,'probe',wait=False)
        assert changed[0].wait(timeout=10)==3 and self.count(op)==0
        assert 'server changed' in changed[2].read_text()
        assert self.h.find(read(op/'operation.json')['launch_token'])==[]
        assert not (op/'permit.json').exists()
        # Completed evidence is usable without contacting the replacement server.
        completed=self.root/'permit-after'
        prior=next((completed/'probes').iterdir())
        before=(prior/'receipt.json').read_bytes()
        self.actor(completed,prior.name,'probe')
        assert (prior/'receipt.json').read_bytes()==before and self.count(prior)==1
        self.pass_case('missing-and-replaced-server',original=original,replacement=self.server_id,
                       missing_error=missing[2].read_text(),changed_error=changed[2].read_text(),count=0)

    def cleanup(self):
        # Child identities come only from newly created case directories under this suite.
        for path in self.root.glob('*/probes/*/child.json'):
            child=read(path)['identity']
            if alive(child):signal_owned(child,signal.SIGTERM)
        for p,who,_ in self.children:
            if p.poll() is None:signal_owned(who,signal.SIGKILL);p.wait(timeout=5)
        self.stop_server()
        remaining=[]
        for pattern in ('*/probes/*/claim.json','*/probes/*/child.json','*/runs/*/operations/*/claim.json','*/runs/*/operations/*/child.json'):
            for path in self.root.glob(pattern):
                who=read(path).get('runner') or read(path).get('identity')
                if alive(who):remaining.append({'path':str(path),'identity':who})
        atomic(self.root/'cleanup.json',{'remaining':remaining},immutable=True)
        assert not remaining,remaining


def main():
    p=argparse.ArgumentParser();p.add_argument('--evidence-root',type=Path,required=True)
    p.add_argument('--cases',choices=['all','cancellation'],default='all');args=p.parse_args()
    suite=Suite(args.evidence_root.resolve())
    passed=False
    try:
        suite.start_server()
        if args.cases=='all':
            suite.dispatch();suite.publication_and_gates();suite.orphan()
        suite.cancellation()
        if args.cases=='all':suite.server_change()
        passed=True
    finally:
        suite.cleanup()
        atomic(suite.root/'result.json',{'passed':passed,'checks':suite.checks,'retained_processes':False},immutable=True)
    print('MILESTONE 3 INTEGRATION PASSED:',suite.root,flush=True)

if __name__=='__main__':main()
