import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from agent_workflow.coordinator_recovery import verify
from agent_workflow.release_maintenance import _quiescent
from agent_workflow.runtime import RuntimePaths
from agent_workflow.session_service import Service, Store
from agent_workflow.storage import OperationalError, atomic, canonical, digest, identity, read


class PrepermitRecovery(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        root=Path(self.temp.name)
        self.paths=RuntimePaths(root,root/'state',root/'registry','a'*32)
        self.store=Store(self.paths)
        accepted=self.store.command({'command_id':'task','action':'submit_message',
            'expected_state_revision':0,'payload':{'text':'Draft a decision log'}})
        self.task_id=accepted['task_id']
        task=self.store.task(self.task_id)
        self.directory=self.paths.state/'session/coordinator'/self.task_id/'call-v1'
        packet={'schema_version':1,'operation_kind':'coordinator','operation_id':'coordinator-'+self.task_id,
            'run_id':self.task_id,'stage':'coordinator-1','role':'coordinator','provider':'fake',
            'model':'deterministic-coordinator-v1','task_id':self.task_id,'task_version':1,
            'project_key':self.paths.key,'message':task['body'],'message_sha256':digest(task['body']),'seconds':1.0}
        target={'socket':'/tmp/isolated.sock','workspace':'w1'}
        op={'operation_id':packet['operation_id'],'packet_sha256':digest(canonical(packet)),
            'project_key':self.paths.key,'launch_token':'original-token','target':target,'provider':'fake'}
        dead={**identity(),'pid':999999999,'start':'0'}
        atomic(self.directory/'packet.json',packet,immutable=True)
        atomic(self.directory/'target.json',target,immutable=True)
        atomic(self.directory/'operation.json',op,immutable=True)
        atomic(self.directory/'launch.json',{'status':'may-have-been-sent','target':target,'server':identity()},immutable=True)
        atomic(self.directory/'claim.json',{'runner':dead,'herdr':{'workspace_id':'w1','pane_id':'w1:p2','tab_id':'w1:t2'}},immutable=True)
        atomic(self.paths.state/'session/ui-current.json',{'project_key':self.paths.key,
            'socket':target['socket'],'pane':{'workspace_id':'w1'},'server':identity()},immutable=True)
        call=self.store.reserve_coordinator(task,op['packet_sha256'])
        self.store.stop_coordinator(self.task_id,call['call_id'],'uncertain','runner exited')
        self.service=Service(self.paths,self.paths.state/'ipc.sock',target['socket'],'test','/bin/false')

    def tearDown(self):self.temp.cleanup()

    def command(self):
        return {'command_id':'recover-one','action':'recover_coordinator',
            'expected_state_revision':2,'expected_task_version':1,'payload':{'task_id':self.task_id}}

    def test_verified_prepermit_recovery_is_append_only_and_idempotent(self):
        with patch.object(self.service,'start_coordinator',return_value=True), patch('agent_workflow.herdr_backend.Herdr') as herdr:
            herdr.return_value.server_identity.return_value=identity()
            herdr.return_value.call.return_value={'workspace':{'workspace_id':'w1'}}
            first=self.service.recover_coordinator(self.command())
            second=self.service.recover_coordinator(self.command())
        self.assertEqual(first,second)
        self.assertEqual(first['status'],'accepted')
        self.assertEqual(self.store.task(self.task_id)['status'],'queued')
        self.assertEqual(read(self.directory/'recovery.json')['original_launch_token'],'original-token')
        self.assertFalse((self.directory/'receipt.json').exists())
        self.assertEqual(self.store.pending_coordinator_call()['task_id'],self.task_id)
        self.assertEqual(len(self.store.events(0,64)['events']),3)

    def test_permit_or_mismatched_packet_refuses_without_reopening(self):
        atomic(self.directory/'permit.json',{'test':'execution may have started'},immutable=True)
        self.assertEqual(self.service.recover_coordinator(self.command())['code'],'recovery_refused')
        self.assertEqual(self.store.task(self.task_id)['status'],'blocked')
        (self.directory/'permit.json').unlink()
        altered=read(self.directory/'packet.json');altered['message']='changed'
        atomic(self.directory/'packet.json',altered)
        self.assertEqual(self.service.recover_coordinator(self.command())['code'],'recovery_refused')
        self.assertFalse((self.directory/'recovery.json').exists())

    def test_restart_after_recovery_intent_reuses_original_command(self):
        body=self.command()
        atomic(self.directory/'recovery.json',{'schema_version':1,'command_id':body['command_id'],
            'replacement':'call-v1-r1','original_launch_token':'original-token'},immutable=True)
        replacement=Service(self.paths,self.paths.state/'ipc.sock','/tmp/isolated.sock','test','/bin/false')
        with patch.object(replacement,'start_coordinator',return_value=True):
            self.assertEqual(replacement.recover_coordinator(body)['status'],'accepted')
            self.assertEqual(replacement.recover_coordinator(body)['status'],'accepted')
        self.assertEqual(self.store.task(self.task_id)['status'],'queued')

    def test_changed_current_herdr_binding_refuses_replacement(self):
        with patch('agent_workflow.herdr_backend.Herdr') as herdr:
            herdr.return_value.server_identity.return_value={**identity(),'pid':1}
            self.assertEqual(self.service.recover_coordinator(self.command())['code'],'recovery_refused')
        self.assertEqual(self.store.task(self.task_id)['status'],'blocked')
        self.assertFalse((self.directory/'recovery.json').exists())

    def test_upgrade_quiescence_accepts_only_proven_prepermit_task(self):
        _quiescent(self.paths)
        atomic(self.directory/'dispatch.json',{'test':'ambiguous'},immutable=True)
        with self.assertRaisesRegex(OperationalError,'unresolved'):_quiescent(self.paths)


if __name__=='__main__':unittest.main()
