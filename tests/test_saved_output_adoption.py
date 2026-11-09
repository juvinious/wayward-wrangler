"""Offline adoption fixtures only. No real-run mutation, Herdr launch or model call."""
import copy
import json
from pathlib import Path
from unittest.mock import patch
from test_context_continuity import ContextContinuity, question
from agent_workflow import recovery, workflow, records, storage, context
from agent_workflow import herdr_backend as backend
from agent_workflow.adapters import real
from agent_workflow.storage import atomic,canonical,digest,read,OperationalError


class SavedOutputAdoption(ContextContinuity):
    # Reuse fixture setup/helpers, not the parent's tests (see load_tests below).
    def setUp(self):
        super().setUp()
        local=self.root/'.local'
        self.extra=[patch.object(recovery,'RUN','fixture'),patch.object(recovery,'ROOT',self.root),
                    patch.object(recovery,'LOCAL',local),patch.object(storage,'LOCAL',local),
                    patch.object(recovery,'alive',return_value=False),patch.object(recovery,'AUTHORIZED',{})]
        for p in self.extra:p.start()
        (self.root/'src/agent_workflow').mkdir(parents=True)
        (self.root/'src/agent_workflow/context.py').write_text('fixture validator version')
        records.create_run(self.env)
        atomic(records.run_dir('fixture')/'inputs/envelope.json',self.env,immutable=True)
        self.directory=records.run_dir('fixture')/'operations/draft'
        def prepare(directory,packet,target):
            op=backend.prepare(directory,packet,target)
            runner={'pid':99999999,'start':'fixture','boot':'fixture','state':'R'}
            binding={'pane_id':'fixture:p1','tab_id':'fixture:t1','workspace_id':'fixture','terminal_id':'fixture-terminal'}
            failed={**{k:op[k] for k in ('operation_id','packet_sha256','launch_token')},'schema_version':1,
                    'runner':runner,'status':'failed','exit_code':0,'error':recovery.FAILURE,'completed':1,'result_sha256':None}
            self.output=self.result(packet,[question()]);self.output['questions'][0]['recommendation']='Keep both to protect existing backups.'
            schema=real.schema(packet);prompt=real.prompt(packet)
            data={'claim.json':{'runner':runner,'herdr':{k:v for k,v in binding.items() if k!='terminal_id'}},
                  'permit.json':{'runner':runner,'launch_token':op['launch_token'],'binding':binding},
                  'dispatch.json':{'runner':runner},'binding.json':binding,'child.json':{'identity':runner},
                  'invocations.json':{'count':1,'operation_id':op['operation_id']},'receipt.json':failed,
                  'output-schema.json':schema,'runtime-result.json':{'fixture':True},
                  'execution.json':{'provider':'claude','packet_sha256':op['packet_sha256'],'prompt_sha256':digest(prompt),
                    'argv':['claude','--json-schema',canonical(schema)]}}
            for file,value in data.items():atomic(directory/file,value,immutable=True)
            atomic(directory/'prompt.txt',prompt,immutable=True)
            final={'type':'result','subtype':'success','is_error':False,'structured_output':self.output,'result':json.dumps(self.output)}
            atomic(directory/'native-events.jsonl',json.dumps(final)+'\n',immutable=True)
            inspection={'native_events_sha256':digest((directory/'native-events.jsonl').read_bytes()),'original_receipt':failed,'structured_output':self.output}
            evidence=local/'evidence/context-rejected-output.json';atomic(evidence,inspection,immutable=True)
            recovery.AUTHORIZED.update(inspection=digest(evidence.read_bytes()),native=inspection['native_events_sha256'],output=digest(canonical(self.output)))
        with patch.object(workflow,'prepare',side_effect=prepare),patch.object(workflow,'wait',return_value={'status':'failed'}):
            with self.assertRaises(workflow.StageStopped):workflow.drive('fixture')
        self.original={p:p.read_bytes() for p in self.directory.iterdir() if p.is_file()}

    def tearDown(self):
        for p in reversed(self.extra):p.stop()
        super().tearDown()

    def test_adoption_publishes_gate_without_dispatch_and_repeat_is_noop(self):
        with patch.object(backend.Herdr,'create',side_effect=AssertionError('No dispatch permitted')) as launch:
            first=recovery.adopt('fixture')
            s=workflow.state('fixture').values
            self.assertEqual(s['completed'],['draft']);self.assertEqual(s['pending']['question'],self.output['questions'][0])
            self.assertEqual(read(self.directory/'result.json'),self.output)
            self.assertEqual(records.get_run('fixture')['status'],'awaiting-input')
            self.assertEqual((records.run_dir('fixture')/s['current']['path']).read_text(),self.output['content'])
            effective=backend.receipt(self.directory)
            self.assertTrue(effective['recovered']);self.assertEqual(effective['original_status'],'failed')
            saved={p:p.read_bytes() for p in (self.directory/'recovery').iterdir()}
            with patch.object(workflow,'drive',side_effect=AssertionError('Repeated adoption must not drive')):
                self.assertEqual(first,recovery.adopt('fixture'))
            self.assertEqual({p:p.read_bytes() for p in saved},saved)
            self.assertEqual({p:p.read_bytes() for p in self.original},self.original)
            launch.assert_not_called()
            self.assertIsNone(records.response('fixture',s['pending']['request_id']))

    def test_evidence_mismatch_refused_before_publication(self):
        for name in ('packet.json','prompt.txt','output-schema.json','native-events.jsonl','receipt.json','permit.json'):
            p=self.directory/name;original=p.read_bytes()
            if p.suffix=='.json':
                value=read(p);value['tampered']=True
                if name=='permit.json':value['launch_token']='wrong'
                p.write_text(canonical(value))
            else:p.write_bytes(original+b'changed')
            with self.assertRaises((OperationalError,ValueError,KeyError)):
                recovery.adopt('fixture')
            self.assertFalse((self.directory/'recovery/adoption.json').exists())
            self.assertFalse((self.directory/'result.json').exists())
            p.write_bytes(original)

    def test_original_identity_and_live_native_refusal(self):
        with patch.object(recovery,'alive',return_value=True):
            with self.assertRaisesRegex(OperationalError,'still alive'):recovery.adopt('fixture')
        with self.assertRaisesRegex(OperationalError,'no other run'):recovery.adopt('unrelated')
        op=read(self.directory/'operation.json');op['operation_id']='wrong';(self.directory/'operation.json').write_text(canonical(op))
        with self.assertRaisesRegex(OperationalError,'identity'):recovery.adopt('fixture')

    def test_recovery_tampering_and_saved_answer_never_consumed_on_repeat(self):
        recovery.adopt('fixture');s=workflow.state('fixture').values
        workflow.answer('fixture',s['pending']['request_id'],'answered','Keep both')
        with patch.object(workflow,'drive',side_effect=AssertionError('Do not consume saved answer')):
            recovery.adopt('fixture')
        self.assertEqual(workflow.state('fixture').values['decisions'],[])
        output=read(self.directory/'result.json');output['content']='altered';(self.directory/'result.json').write_text(canonical(output))
        with self.assertRaisesRegex(OperationalError,'output changed'):backend.receipt(self.directory)


def load_tests(loader, tests, pattern):
    import unittest
    names=('test_adoption_publishes_gate_without_dispatch_and_repeat_is_noop',
           'test_evidence_mismatch_refused_before_publication','test_original_identity_and_live_native_refusal',
           'test_recovery_tampering_and_saved_answer_never_consumed_on_repeat')
    return unittest.TestSuite(SavedOutputAdoption(name) for name in names)
