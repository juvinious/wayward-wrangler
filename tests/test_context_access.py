"""Explicit foreign reads are source read-only and scoped to one selected task."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from agent_workflow import artifacts, context_access, records
from agent_workflow import workflow
from agent_workflow.runtime import RuntimePaths, bind, unbind
from agent_workflow.session_service import Service
from agent_workflow.storage import OperationalError, atomic, canonical, digest


class ContextAccessTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        root=Path(self.tmp.name)
        self.a=RuntimePaths(root/'A',root/'A/.wayward-wrangler/state',root/'registry','a'*32)
        self.b=RuntimePaths(root/'B',root/'B/.wayward-wrangler/state',root/'registry','b'*32)
        for paths in (self.a,self.b):
            paths.project.mkdir()
            atomic(paths.project/'.wayward-wrangler/project.toml',
                   f'schema_version = 1\nproject_key = "{paths.key}"\n')
            atomic(paths.state/'project-identity.json',{'schema_version':1,'project_key':paths.key})
        atomic(self.a.project/'.wayward-wrangler/workflow.toml',
               'review_limit = 2\n[roles.author]\nprovider = "fake"\nmodel = "author"\n'
               '[roles.critic]\nprovider = "fake"\nmodel = "critic"\n[fake]\nseconds = 1.0\n')
        atomic(self.a.state/'session/ui-current.json',{'pane':{'workspace_id':'w-test'}})
        token=bind(self.b)
        try:
            records.create_run({'run_id':'shared','project_key':self.b.key})
            ref=artifacts.publish('shared','draft',{'operation_id':'shared-draft',
                'content':'B PRIVATE SENTINEL','findings':[]},'packet',[])
            records.request('shared',{'request_id':'A1','kind':'final-review','artifact':ref,
                                      'artifact_sha256':ref['sha256'],'decision_revision':0,
                                      'decisions':[],'findings':{}})
            records.save_response('shared',{'request_id':'A1','outcome':'approved','answer':'approved fixture',
                                            'artifact_sha256':ref['sha256'],'decision_revision':0})
            records.set_status('shared','approved')
        finally:unbind(token)
        self.service=Service(self.a,self.a.state/'session/ipc.sock','/tmp/unused-herdr.sock','proof-v1','/bin/false')
        submitted=self.service.store.command({'command_id':'submit','action':'submit_message',
                                               'expected_state_revision':0,'payload':{'text':'A task'}})
        self.task_id=submitted['task_id']
        task=self.service.store.task(self.task_id)
        call=self.service.store.reserve_coordinator(task,'packet')
        self.service.store.publish_proposal(self.task_id,call['call_id'],{'proposal':{
            'task_id':self.task_id,'task_version':1,'message_sha256':digest(task['body']),
            'brief':task['body']}},'result')

    def tearDown(self):self.tmp.cleanup()

    def grant(self, command_id='grant', mode='pinned'):
        return {'command_id':command_id,'action':'grant_context_access',
                'expected_task_version':1,'expected_state_revision':2,
                'payload':{'task_id':self.task_id,'source_project':str(self.b.project),
                           'run':'shared','kind':'artifact','id':'draft',
                           'purpose':'Use the exact selected artifact for this task',
                           'dependency_mode':mode}}

    def test_default_zero_foreign_context_and_explicit_import(self):
        self.assertEqual(context_access.active_for_task(self.a,self.task_id),[])
        with patch('agent_workflow.herdr_backend.Herdr.call',return_value={}):
            self.assertNotIn('cross_imports',self.service.plan({'task_id':self.task_id}))
        body=self.grant()
        accepted=self.service.grant_context(body)
        self.assertEqual(accepted['status'],'accepted')
        self.assertEqual(self.service.grant_context(body),accepted)
        grants=context_access.active_for_task(self.a,self.task_id)
        self.assertEqual(len(grants),1)
        self.assertEqual(grants[0]['item']['content'],'B PRIVATE SENTINEL')
        self.assertEqual(grants[0]['source_project_key'],self.b.key)
        with patch('agent_workflow.herdr_backend.Herdr.call',return_value={}):
            plan=self.service.plan({'task_id':self.task_id})
        self.assertEqual([item['item']['qualified_id'] for item in plan['cross_imports']],
                         [grants[0]['item']['qualified_id']])
        self.assertNotIn('B PRIVATE SENTINEL',canonical(self.service.store.snapshot()))
        frozen=[{'grant_id':grants[0]['grant_id'],'item_sha256':digest(canonical(grants[0]['item'])),
                 'purpose':grants[0]['purpose'],'item':grants[0]['item']}]
        context_access.check_frozen(self.a,frozen)
        revoked=self.service.revoke_context({'command_id':'revoke','action':'revoke_context_access',
                                              'payload':{'grant_id':accepted['grant_id']}})
        self.assertEqual(revoked['status'],'accepted')
        self.assertEqual(context_access.active_for_task(self.a,self.task_id),[])
        with self.assertRaisesRegex(OperationalError,'revoked'):
            context_access.check_frozen(self.a,frozen)

    def test_pinned_snapshot_ignores_unrelated_source_changes(self):
        accepted=self.service.grant_context(self.grant(mode='pinned'))
        self.assertEqual(accepted['status'],'accepted')
        grant=context_access.active_for_task(self.a,self.task_id)[0]
        frozen=[{'grant_id':grant['grant_id'],'item_sha256':digest(canonical(grant['item'])),
                 'purpose':grant['purpose'],'item':grant['item']}]
        token=bind(self.b)
        try:
            records.create_run({'run_id':'unrelated','project_key':self.b.key})
            records.set_status('unrelated','approved')
        finally:unbind(token)
        context_access.check_frozen(self.a,frozen)
        self.assertEqual(context_access.select(context_access.source_binding(self.b.project),
            {'run':'shared','kind':'artifact','id':'draft'})['version_sha256'],grant['item']['version_sha256'])

    def test_selected_source_change_stops_only_explicit_dependency(self):
        accepted=self.service.grant_context(self.grant(mode='must_match_source'))
        self.assertEqual(accepted['status'],'accepted')
        grant=context_access.active_for_task(self.a,self.task_id)[0]
        frozen=[{'grant_id':grant['grant_id'],'item_sha256':digest(canonical(grant['item'])),
                 'purpose':grant['purpose'],'item':grant['item']}]
        path=self.b.state/'runs/shared/artifacts/draft.md'
        path.write_text('Changed selected source')
        with self.assertRaisesRegex(OperationalError,'changed'):
            context_access.check_frozen(self.a,frozen)

    def test_revoked_import_refuses_future_worker_dispatch(self):
        accepted=self.service.grant_context(self.grant())
        grant=context_access.active_for_task(self.a,self.task_id)[0]
        frozen=[{'grant_id':grant['grant_id'],'item_sha256':digest(canonical(grant['item'])),
                 'purpose':grant['purpose'],'item':grant['item']}]
        self.service.revoke_context({'command_id':'revoke','action':'revoke_context_access',
                                     'payload':{'grant_id':accepted['grant_id']}})
        token=bind(self.a)
        try:
            with patch.object(workflow,'check_stop'), \
                 patch.object(records,'get_run',return_value={'envelope':{'cross_imports':frozen}}), \
                 patch('agent_workflow.context.check'), \
                 patch.object(records,'reserve') as dispatch:
                with self.assertRaisesRegex(OperationalError,'revoked'):
                    workflow.worker({'run_id':'future','next_stage':'draft','completed':[]})
                dispatch.assert_not_called()
        finally:unbind(token)


if __name__=='__main__':unittest.main()
