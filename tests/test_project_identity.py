"""Offline checks for the public CLI rename and retained controller compatibility."""
import subprocess
import tomllib
import unittest
from unittest.mock import Mock
from agent_workflow import routing
from agent_workflow.storage import ROOT


class ProjectIdentity(unittest.TestCase):
    def test_installed_primary_and_compatibility_entry_points(self):
        project=tomllib.loads((ROOT/'pyproject.toml').read_text())['project']
        self.assertEqual(project['name'],'wayward-wrangler')
        self.assertEqual(project['description'],'Persistent workflows for coding agents.')
        for command in ('wayward-wrangler','awp'):
            self.assertEqual(project['scripts'][command],'agent_workflow.cli:main')
            result=subprocess.run([str(ROOT/'.venv/bin'/command),'--help'],capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertIn('Wayward Wrangler',result.stdout)

    def test_controller_detection_supports_both_cli_names(self):
        for executable in ('/fixture/awp','/fixture/wayward-wrangler','awp','wayward-wrangler'):
            h=Mock()
            def call(method,params=None):
                if method=='tab.list':return {'tabs':[{'tab_id':'t1'}]}
                return {'layout':{'root':{'type':'pane','pane_id':'p1',
                    'command':[executable,'demo','--run','fixture']}}}
            h.call.side_effect=call
            self.assertEqual(routing.controller_panes(h,'fixture'),['p1'])
            self.assertEqual(routing.controller_panes(h,'unrelated'),[])
