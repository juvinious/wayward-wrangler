"""Isolated records/checkpoints and deterministic result fixtures; never native CLI calls."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from agent_workflow import context, records, artifacts, workflow
from agent_workflow.adapters import real
from agent_workflow.herdr_backend import validate_result
from agent_workflow.storage import OperationalError, atomic, canonical, digest


def question():
    return {'id':'collision','question':'How should existing files be handled?',
            'options':['Keep both','Replace with confirmation'],'recommendation':'Keep both',
            'consequences':'Preservation versus stable filenames','topic_key':'export.collision',
            'rationale':'The brief leaves this substantive behavior unspecified',
            'evidence_refs':['brief'],'related_decision_refs':[],'reconsideration_reason':''}


class ContextContinuity(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.root=Path(self.tmp.name); local=self.root/'.local'
        self.patches=[patch.object(context,'ROOT',self.root),patch.object(records,'LOCAL',local),
                      patch.object(records,'DB',local/'state.sqlite')]
        for p in self.patches:p.start()
        self.source('source')
        (self.root/'AGENTS.md').write_text('Canonical rules')
        (self.root/'CLAUDE.md').write_text('Read [AGENTS.md](AGENTS.md).')
        self.selection=self.root/'context.toml'
        self.selection.write_text('''schema_version=1
project_id="notes"
scope="release"
[[entries]]
kind="decision"
run="source"
id="D-policy"
topic_key="release.backup-policy"
scope="release"
reason="release policy"
[[entries]]
kind="artifact"
run="source"
id="draft"
scope="release"
reason="approved scope"
[[entries]]
kind="findings"
run="source"
ids=["open"]
scope="release"
reason="unresolved work"
[[entries]]
kind="document"
path="CLAUDE.md"
scope="release"
reason="instructions"
''')
        self.snapshot=context.resolve(self.selection)
        self.env={'run_id':'fixture','workflow_version':2,'context_snapshot':self.snapshot,
                  'context_digest':digest(canonical(self.snapshot)), 'brief':'Choose export behavior',
                  'instructions':[],'skills':dict.fromkeys(['draft','critique','revise'],'fixture'),
                  'rules_sha256':'rules','config_sha256':'config', 'target':{},
                  'config':{'roles':{'author':{'provider':'claude','model':'opus'},'critic':{'provider':'codex','model':'sol'}}}}

    def tearDown(self):
        for p in reversed(self.patches):p.stop()
        self.tmp.cleanup()

    def source(self,run):
        records.create_run({'run_id':run})
        result={'operation_id':run+'-draft','content':'Approved product scope','findings':[], 'dispositions':[]}
        ref=artifacts.publish(run,'draft',result,'packet',[])
        q={'id':'policy','question':'Backup policy?','options':['Offline','Cloud'],'recommendation':'Offline','consequences':'local or network'}
        req={'request_id':'D-policy','kind':'choice','question':q,'artifact_sha256':ref['sha256'],'decision_revision':0}
        records.request(run,req)
        answer=records.save_response(run,{'request_id':'D-policy','answer':'Offline','outcome':'answered','artifact_sha256':ref['sha256'],'decision_revision':0})
        records.request(run,{'request_id':'A1','kind':'final-review','artifact':ref,'artifact_sha256':ref['sha256'],
                             'decision_revision':1,'decisions':[{**answer,'question':q}],'findings':{}})
        records.save_response(run,{'request_id':'A1','answer':'fixture approval','outcome':'approved','artifact_sha256':ref['sha256'],'decision_revision':1})
        records.set_status(run,'approved')

    def state(self):return workflow.initial('fixture',self.env)
    def packet(self,state=None,stage='draft'):return artifacts.packet(self.env,state or self.state(),stage)
    def result(self,packet,questions=None):
        return {'schema_version':2,'operation_id':packet['operation_id'],'packet_sha256':digest(canonical(packet)),
                'content':'Provisional export proposal','findings':[],'dispositions':[],
                'questions':questions or [],'needs_revision':False}

    def test_selection_delivery_and_canonical_pointer(self):
        p=self.packet()
        self.assertEqual(p['decisions'][0]['answer'],'Offline')
        self.assertEqual(p['decisions'][0]['qualified_id'],'source/D-policy')
        self.assertEqual(len([e for e in self.snapshot['entries'] if e['kind']=='document']),1)
        self.assertNotIn('planned_decision',p)
        self.assertIn('source/draft',p['evidence_ids'])
        self.assertEqual(real.schema(p)['properties']['schema_version']['enum'],[2])
        self.assertNotIn('leave questions empty',real.prompt(p))
        self.assertEqual(records.response('fixture','D-policy'),None)

    def test_stale_selection_document_artifact_and_export(self):
        targets=[self.selection,self.root/'AGENTS.md',records.run_dir('source')/'artifacts/draft.md',
                 records.run_dir('source')/'artifacts/decision-D-policy.json']
        for path in targets:
            original=path.read_bytes()
            path.write_text('{}' if path.suffix=='.json' else path.read_text()+'\nchanged')
            with self.assertRaisesRegex(OperationalError,'Stale context'):context.check(self.env)
            path.write_bytes(original)
        context.check(self.env)

    def test_reject_unapproved_scope_conflict_and_duplicate_topic(self):
        records.set_status('source','rejected')
        with self.assertRaises(OperationalError):context.resolve(self.selection)
        records.set_status('source','approved')
        text=self.selection.read_text();self.selection.write_text(text.replace('reason="release policy"','reason="release policy"\nscope2="irrelevant"').replace('scope="release"\nreason="release policy"','scope="different"\nreason="release policy"'))
        with self.assertRaises(OperationalError):context.resolve(self.selection)
        self.selection.write_text(text)
        self.source('second')
        self.selection.write_text(text+'''\n[[entries]]
kind="decision"
run="second"
id="D-policy"
topic_key="release.backup-policy"
scope="release"
reason="conflict"
''')
        with self.assertRaisesRegex(OperationalError,'Conflicting'):context.resolve(self.selection)

    def test_explicit_approved_supersession_blocks_old_selection(self):
        records.create_run({'run_id':'later','context_snapshot':{'project_id':'notes'}})
        records.save_response('later',{'request_id':'D-change','answer':'new','outcome':'answered','supersedes':['source/D-policy'],'artifact_sha256':'hash','decision_revision':0})
        records.set_status('later','approved')
        with self.assertRaisesRegex(OperationalError,'supersession'):context.resolve(self.selection)

    def test_open_findings_qualified_across_runs(self):
        self.source('second')
        for run in ('source','second'):
            f={'id':'C1-001','severity':'minor','evidence':'Unresolved wording','requested_change':'Clarify'}
            result={'operation_id':run+'-critique','content':'Finding','findings':[f]}
            artifacts.publish(run,'critique-1',result,'packet',[])
            req=context.request(run,'A1');req['findings']={'C1-001':{**f,'status':'open','origin':'critique-1','reviewed':req['artifact']}}
            with records.connect() as db:db.execute('UPDATE app_requests SET payload=? WHERE run_id=? AND request_id=?',(canonical(req),run,'A1'))
        self.selection.write_text(self.selection.read_text()+'''\n[[entries]]
kind="findings"
run="second"
ids=["open"]
scope="release"
reason="more findings"
''')
        snap=context.resolve(self.selection)
        self.assertEqual({e['qualified_id'] for e in snap['entries'] if e['kind']=='findings'},{'source/C1-001','second/C1-001'})

    def test_application_owned_finding_origin_uses_immutable_ledger(self):
        from agent_workflow.findings import durable_id
        run='source';stage='critique-1';operation=run+'-'+stage
        raw={'schema_version':2,'operation_id':operation,'content':'Issue evidence',
             'new_findings':[{'severity':'minor','evidence':'Missing fallback','requested_change':'Specify fallback'}],
             'assessments':[],'questions':[],'needs_revision':True}
        artifacts.publish(run,stage,raw,'packet',[])
        key=durable_id(operation,0)
        ledger={'schema_version':1,'stage':stage,'result_sha256':digest(canonical(raw)),
                'new_findings':[{**raw['new_findings'][0],'id':key,'source_index':0}],
                'assessments':[],'missing_assessments':[]}
        path=records.run_dir(run)/'quality'/f'{stage}-finding-ledger.json'
        atomic(path,ledger,immutable=True)
        req=context.request(run,'A1')
        req['findings']={key:{**raw['new_findings'][0],'id':key,'status':'open','origin':stage,'reviewed':req['artifact']}}
        with records.connect() as db:
            db.execute('UPDATE app_requests SET payload=? WHERE run_id=? AND request_id=?',(canonical(req),run,'A1'))
        selected=context.resolve(self.selection)
        self.assertIn('source/'+key,{entry['qualified_id'] for entry in selected['entries']})
        ledger['new_findings'][0]['evidence']='changed'
        path.write_text(canonical(ledger))
        with self.assertRaisesRegex(OperationalError,'ledger'):
            context.resolve(self.selection)

    def test_question_contract_invalid_references_and_legacy_refusal(self):
        p=self.packet();r=self.result(p,[question()]);validate_result(r,p)
        for change in ({'evidence_refs':['made-up']},{'options':['only']},{'related_decision_refs':['source/D-policy']}, {'topic_key':''}):
            bad=copy.deepcopy(r);bad['questions'][0].update(change)
            with self.assertRaises(OperationalError):validate_result(bad,p)
        p1={**p,'schema_version':1};r1={**r,'schema_version':1,'packet_sha256':digest(canonical(p1))}
        with self.assertRaisesRegex(OperationalError,'controller-owned'):validate_result(r1,p1)

    def test_prose_recommendation_is_advice_not_an_answer(self):
        p=self.packet();q=question()
        q['recommendation']='Keep both so earlier backups remain recoverable.'
        result=self.result(p,[q])
        validate_result(result,p)
        change=workflow.context_change(self.state(),result,{'id':'draft'},'packet',{})
        self.assertEqual(change['questions'][0]['recommendation'],q['recommendation'])
        self.assertEqual(records.response('fixture','D-collision'),None)

    def test_inherited_repeat_and_reconsideration_routing(self):
        s=self.state();p=self.packet(s);q=question();prior=s['inherited_decisions'][0]
        q.update(topic_key='release.backup-policy',question=prior['question']['question'],options=prior['question']['options'])
        ref={'id':'draft','sha256':'hash'}
        change=workflow.context_change(s,self.result(p,[q]),ref,'packet',{})
        self.assertEqual(change['questions'],[])
        q['question']='Reopen this without evidence?'
        change=workflow.context_change(s,self.result(p,[q]),ref,'packet',{})
        self.assertTrue(change['blocked_reason'])
        q.update(related_decision_refs=['source/D-policy'],reconsideration_reason='New brief evidence changes the scope')
        change=workflow.context_change(s,self.result(p,[q]),ref,'packet',{})
        self.assertFalse(change['blocked_reason']);self.assertEqual(len(change['questions']),1)

    def test_question_limit_retains_completed_stage(self):
        s=self.state();s['questions_asked']=1
        change=workflow.context_change(s,self.result(self.packet(s),[question()]),{'id':'draft'},'packet',{})
        self.assertEqual(change['completed'],['draft']);self.assertTrue(change['blocked_reason'])
        self.assertEqual(workflow.route(change),'blocked_gate')

    def test_two_questions_are_serial_and_answers_do_not_spend_stage_slots(self):
        s=self.state();s['max_questions']=2
        first=question();second={**question(),'id':'storage','topic_key':'storage.retention',
                            'question':'Where should local records remain?'}
        change=workflow.context_change(s,self.result(self.packet(s),[first,second]),
                                       {'id':'draft','sha256':'candidate'},'packet',{})
        self.assertFalse(change['blocked_reason'])
        self.assertEqual(change['completed'],['draft'])
        s.update(change)
        pending=workflow.prepare_choice(s)['pending'];s['pending']=pending
        answer=records.save_response('fixture',{'request_id':pending['request_id'],'answer':first['options'][0],
                                               'outcome':'answered','artifact_sha256':'candidate',
                                               'decision_revision':0,'context_digest':s['context_digest']})
        with patch.object(workflow,'interrupt',return_value=answer):
            first_change=workflow.choice_gate(s)
        self.assertEqual(len(first_change['questions']),1)
        s.update(first_change)
        pending2=workflow.prepare_choice(s)['pending']
        self.assertNotEqual(pending2['request_id'],pending['request_id'])
        self.assertEqual(pending2['decision_revision'],1)
        self.assertEqual(s['completed'],['draft'])

    def test_answer_requires_author_revision_and_late_answer_blocks(self):
        s=self.state();s.update(next_stage='critique-1',revision_required=True)
        p=self.packet(s,'critique-1')
        change=workflow.context_change(s,self.result(p),{'id':'critique-1'},'packet',{})
        self.assertEqual(change['next_stage'],'revision-1')
        s.update(next_stage='final',questions=[question()],pending={'request_id':'D-one'})
        saved={'request_id':'D-one','context_digest':s['context_digest']}
        with patch.object(workflow,'interrupt',return_value=saved),patch.object(records,'response',return_value=saved),patch.object(records,'export_response'):
            change=workflow.choice_gate(s)
        self.assertTrue(change['blocked_reason'])
        with self.assertRaises(OperationalError):workflow.prepare_final({**s,**change})

    def test_duplicate_request_refuses_changed_payload(self):
        req={'request_id':'D-fixture','question':question()}
        records.request('fixture',req);records.request('fixture',req)
        with self.assertRaises(OperationalError):records.request('fixture',{**req,'extra':'changed'})

    def test_real_checkpoints_record_only_answer_then_fresh_graph(self):
        records.create_run(self.env)
        calls=[];packets=[]
        def prepare(directory,packet,target):
            if not (directory/'result.json').exists():
                calls.append(packet['stage']);packets.append(packet)
                result=self.result(packet,[question()] if packet['stage']=='draft' else [])
                validate_result(result,packet);atomic(directory/'result.json',result,immutable=True)
        with patch.object(workflow,'prepare',side_effect=prepare),patch.object(workflow,'wait',return_value={'status':'completed'}):
            first=workflow.drive('fixture'); gate=first.values['pending']
            self.assertEqual(calls,['draft']);self.assertEqual(first.values['completed'],['draft'])
            workflow.drive('fixture');self.assertEqual(calls,['draft'])
            workflow.answer('fixture',gate['request_id'],'answered','Keep both')
            # Persisted before any graph consumption; a new graph/connection reads it.
            self.assertEqual(workflow.state('fixture').values['decisions'],[])
            second=workflow.drive('fixture')
            self.assertEqual(calls,['draft','critique-1','revision-1','critique-2'])
            self.assertEqual(len(second.values['decisions']),1)
            self.assertEqual(packets[1]['decisions'][-1]['answer'],'Keep both')
            self.assertTrue(packets[-1]['previous_reviewed'])
            final=second.values['pending'];self.assertEqual(final['kind'],'final-review')
            with self.assertRaises(OperationalError):workflow.answer('fixture','A1','approved','fixture approval','stale')
            workflow.answer('fixture','A1','approved','fixture approval',final['artifact_sha256'])
            workflow.drive('fixture');workflow.drive('fixture')
            self.assertEqual(records.get_run('fixture')['status'],'approved')
            self.assertEqual(records.response('fixture','A1')['context_digest'],self.env['context_digest'])
            self.assertEqual(len(calls),4)
            self.assertEqual(records.response('source','D-policy')['answer'],'Offline')

    def test_answer_rejects_changed_context_and_binding(self):
        records.create_run(self.env)
        s=self.state();s.update(current={'id':'draft'},pending={'request_id':'A1','context_digest':'stale','artifact_sha256':'hash','decision_revision':0,'kind':'final-review'})
        from types import SimpleNamespace
        original_content=artifacts.content
        with patch.object(workflow,'state',return_value=SimpleNamespace(values=s,interrupts=[1])),patch.object(artifacts,'content',side_effect=lambda run,ref: '' if run=='fixture' else original_content(run,ref)):
            with self.assertRaisesRegex(OperationalError,'context'):workflow.answer('fixture','A1','approved','yes','hash')
        changed={**self.env,'context_digest':'bad'}
        with self.assertRaisesRegex(OperationalError,'Stale context'):context.check(changed)
