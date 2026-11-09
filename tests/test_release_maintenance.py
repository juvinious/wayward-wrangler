import json
import tempfile
import unittest
from pathlib import Path

from agent_workflow.release_maintenance import manage
from agent_workflow.storage import OperationalError, atomic, digest


class ReleaseMaintenanceTest(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.root=Path(self.temp.name)
        self.catalog=self.root/'releases'
        for release in ('old','new'):
            root=self.catalog/release
            (root/'bin').mkdir(parents=True)
            (root/'venv/bin').mkdir(parents=True)
            (root/'bin/waywr').write_text('fixture')
            (root/'venv/bin/python').write_text('fixture')
            atomic(root/'manifest.json',{'schema_version':1,'release_id':release,'protocol_major':1,
                                         'state_schema_min':1,'state_schema_max':1})
        self.a=self.project('A','a'*32)
        self.b=self.project('B','b'*32)

    def tearDown(self):self.temp.cleanup()

    def project(self,name,key):
        path=self.root/name
        (path/'.wayward-wrangler/state').mkdir(parents=True)
        atomic(path/'.wayward-wrangler/project.toml',f'schema_version = 1\nproject_key = "{key}"\n')
        return path

    def run_action(self,action,project,release=None):
        return manage(action,project,None,self.root/'registry',self.catalog,release)

    def test_project_bindings_upgrade_independently(self):
        self.assertEqual(self.run_action('bind',self.a,'old')['status'],'bound')
        self.assertEqual(self.run_action('bind',self.b,'old')['status'],'bound')
        self.assertEqual(self.run_action('upgrade',self.a,'new')['release_binding']['release_id'],'new')
        self.assertEqual(self.run_action('status',self.b)['release_binding']['release_id'],'old')
        self.assertEqual(self.run_action('rollback',self.a,'old')['release_binding']['release_id'],'old')
        self.assertEqual(self.run_action('status',self.b)['release_binding']['release_id'],'old')
        self.assertTrue((self.a/'.wayward-wrangler/state/session/release-bindings/3-complete.json').exists())

    def test_unavailable_unrelated_project_does_not_block_upgrade(self):
        self.run_action('bind',self.a,'old')
        self.b.rename(self.root/'B-offline')
        self.assertEqual(self.run_action('upgrade',self.a,'new')['status'],'bound')

    def test_busy_selected_project_refuses_only_its_upgrade(self):
        import sqlite3
        self.run_action('bind',self.a,'old')
        self.run_action('bind',self.b,'old')
        db=sqlite3.connect(self.a/'.wayward-wrangler/state/state.sqlite')
        db.execute('CREATE TABLE session_tasks(task_id TEXT,status TEXT)')
        db.execute("INSERT INTO session_tasks VALUES ('task','awaiting-input')")
        db.commit();db.close()
        with self.assertRaisesRegex(OperationalError,'unresolved'):
            self.run_action('upgrade',self.a,'new')
        self.assertEqual(self.run_action('upgrade',self.b,'new')['status'],'bound')

    def test_incomplete_binding_is_visible_and_reconciled_by_exact_action(self):
        old=self.run_action('bind',self.a,'old')['release_binding']
        manifest=self.catalog/'new/manifest.json'
        from agent_workflow.storage import digest
        intended={'schema_version':1,'project_key':'a'*32,'generation':2,
                  'action':'upgrade','old':old,'target_release_id':'new',
                  'target_manifest_sha256':digest(manifest.read_bytes())}
        root=self.a/'.wayward-wrangler/state/session/release-bindings'
        atomic(root/'2-intent.json',intended,immutable=True)
        status=self.run_action('status',self.a)
        self.assertEqual(status['binding_recovery'],intended)
        with self.assertRaisesRegex(OperationalError,'exact target action'):
            self.run_action('rollback',self.a,'new')
        result=self.run_action('upgrade',self.a,'new')
        self.assertEqual(result['release_binding']['binding_generation'],2)
        self.assertTrue((root/'2-complete.json').exists())
        self.assertIsNone(self.run_action('status',self.a)['binding_recovery'])


if __name__=='__main__':unittest.main()
