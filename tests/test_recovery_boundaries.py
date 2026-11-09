"""Offline recovery permutations. No Herdr or model process is launched."""
import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from agent_workflow import herdr_backend as backend, storage
from agent_workflow.storage import atomic, read, identity, Uncertain, digest, canonical


class RecoveryBoundaries(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.root=Path(self.tmp.name);self.opdir=self.root/'operation'
        self.patches=[patch.object(storage,'LOCAL',self.root),patch.object(backend,'LOCAL',self.root)]
        for p in self.patches:p.start()
        self.packet={'schema_version':1,'operation_id':'test-draft','run_id':'test',
                     'stage':'draft','role':'author','model':'fake','provider':'fake'}
        self.op=backend.prepare(self.opdir,self.packet,{'socket':'/fixture','workspace':'w1'})
        self.me=identity();self.binding={'pane_id':'w1:p1','tab_id':'w1:t1','workspace_id':'w1'}
        self.h=Mock();self.h.server_identity.return_value=self.me
        self.h.find.side_effect=lambda token: [dict(self.binding)]
        self.h.call.return_value={'pane':{'terminal_id':'terminal'}}
        self.p=patch.object(backend,'Herdr',return_value=self.h);self.p.start()

    def tearDown(self):
        self.p.stop()
        for p in reversed(self.patches):p.stop()
        self.tmp.cleanup()

    def launch(self):atomic(self.opdir/'launch.json',{'server':self.me},immutable=True)
    def claim(self):atomic(self.opdir/'claim.json',{'runner':self.me,'herdr':self.binding},immutable=True)
    def completed(self):
        self.claim()
        result={**{k:self.packet[k] for k in ('schema_version','operation_id')},
                'packet_sha256':digest(canonical(self.packet)),'content':'result','findings':[],
                'dispositions':[],'questions':[],'needs_revision':False}
        atomic(self.opdir/'result.json',result,immutable=True)
        done={k:self.op[k] for k in ('operation_id','packet_sha256','launch_token')}
        done.update(status='completed',runner=self.me,result_sha256=digest(canonical(result)))
        atomic(self.opdir/'receipt.json',done,immutable=True)
        return done

    def test_claim_without_launch_refuses_new_pane(self):
        self.claim()
        with self.assertRaisesRegex(Uncertain,'without launch'):backend.ensure(self.opdir)
        self.h.create.assert_not_called()

    def test_persisted_intent_without_claim_never_recreates(self):
        self.launch()
        with patch.object(backend.time,'monotonic',side_effect=[0,9]):
            with self.assertRaisesRegex(Uncertain,'no runner claim'):backend.ensure(self.opdir)
        self.h.create.assert_not_called()
        self.assertFalse((self.opdir/'permit.json').exists())

    def test_claim_reconciliation_and_permit_are_idempotent(self):
        self.launch();self.claim()
        backend.ensure(self.opdir);before=(self.opdir/'permit.json').read_bytes()
        backend.ensure(self.opdir)
        self.assertEqual(before,(self.opdir/'permit.json').read_bytes())
        self.h.create.assert_not_called()

    def test_server_identity_changes_before_or_after_claim_verification(self):
        self.launch();self.claim()
        for key in ('pid','start','boot'):
            changed={**self.me,key:'different'}
            for identities in ([changed],[self.me,changed]):
                with self.subTest(key=key,identities=len(identities)):
                    self.h.server_identity.side_effect=identities
                    with self.assertRaisesRegex(Uncertain,'server changed'):backend.ensure(self.opdir)
                    self.assertFalse((self.opdir/'permit.json').exists())
        self.h.create.assert_not_called()

    def test_missing_server_refuses_and_status_explains(self):
        self.launch();self.claim();self.h.server_identity.side_effect=FileNotFoundError()
        with self.assertRaisesRegex(Uncertain,'server unavailable'):backend.ensure(self.opdir)
        report=backend.inspect_operation(self.opdir)
        self.assertEqual(report['status'],'uncertain');self.assertIn('Do not delete',report['operator_action'])
        self.h.create.assert_not_called()

    def test_lost_creation_response_is_never_retried(self):
        self.h.create.side_effect=TimeoutError('response lost')
        with self.assertRaises(TimeoutError):backend.ensure(self.opdir)
        self.assertTrue((self.opdir/'launch.json').exists())
        with patch.object(backend.time,'monotonic',side_effect=[0,9]):
            with self.assertRaises(Uncertain):backend.ensure(self.opdir)
        self.assertEqual(self.h.create.call_count,1)

    def test_dead_runner_live_child_is_uncertain_without_cleanup(self):
        self.launch();self.claim();atomic(self.opdir/'child.json',{'identity':{'pid':1}})
        with patch.object(backend,'alive',side_effect=[False,True]):
            report=backend.inspect_operation(self.opdir)
        self.assertTrue(report['child_alive']);self.assertEqual(report['status'],'uncertain')
        self.assertIn('still alive',report['detail']);self.h.create.assert_not_called()
        with patch.object(backend,'alive',return_value=False):
            with self.assertRaises(Uncertain):backend.ensure(self.opdir)
            with self.assertRaises(Uncertain):backend.cancel(self.opdir)
        self.assertFalse((self.opdir/'cancel.json').exists())

    def test_receipt_wins_over_cancel_and_unavailable_or_replaced_server(self):
        done=self.completed();before=(self.opdir/'receipt.json').read_bytes()
        self.h.server_identity.side_effect=FileNotFoundError()
        for _ in range(2):
            self.assertEqual(backend.ensure(self.opdir),done)
            self.assertEqual(backend.cancel(self.opdir),done)
        self.assertEqual((self.opdir/'receipt.json').read_bytes(),before)
        self.assertFalse((self.opdir/'cancel.json').exists())
        self.h.server_identity.assert_not_called();self.h.create.assert_not_called()

    def test_corrupt_completion_never_dispatches(self):
        self.completed();(self.opdir/'result.json').write_text('{}')
        with self.assertRaisesRegex(Uncertain,'hash mismatch'):backend.ensure(self.opdir)
        self.assertEqual(backend.inspect_operation(self.opdir)['status'],'uncertain')
        self.h.create.assert_not_called()

    def test_duplicate_or_changed_pane_prevents_permission(self):
        self.launch();self.claim()
        for bindings in ([self.binding,self.binding],[{**self.binding,'pane_id':'other'}]):
            self.h.find.side_effect=None
            self.h.find.return_value=copy.deepcopy(bindings)
            with self.assertRaises(Uncertain):backend.ensure(self.opdir)
            self.assertFalse((self.opdir/'permit.json').exists())
        self.h.create.assert_not_called()

    def test_cancelled_or_failed_receipt_is_terminal_even_with_late_cancel(self):
        done=self.completed()
        for status in ('cancelled','failed'):
            terminal={**done,'status':status,'result_sha256':None}
            atomic(self.opdir/'receipt.json',terminal)
            before=(self.opdir/'receipt.json').read_bytes()
            self.assertEqual(backend.ensure(self.opdir),terminal)
            self.assertEqual(backend.cancel(self.opdir),terminal)
            self.assertEqual((self.opdir/'receipt.json').read_bytes(),before)
            self.assertFalse((self.opdir/'cancel.json').exists())
        self.h.create.assert_not_called()

    def test_explicit_state_root_rejects_escape(self):
        with self.assertRaises(storage.OperationalError):
            storage.operation_path(self.root/'../outside',local=self.root)
        self.assertEqual(storage.operation_path(self.opdir,local=self.root),self.opdir)

    def test_permission_without_launch_is_uncertain_in_status(self):
        atomic(self.opdir/'permit.json',{'runner':self.me})
        report=backend.inspect_operation(self.opdir)
        self.assertEqual(report['status'],'uncertain')
        self.assertIn('without launch',report['detail'])
