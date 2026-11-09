"""Offline UI and plugin boundary tests. No live Herdr calls or worker dispatch."""
import io
import os
from pathlib import Path
import re
import tempfile
import tomllib
import unittest
from contextlib import redirect_stdout
from types import SimpleNamespace
from unittest.mock import Mock, patch
from agent_workflow import cli, records, review, terminal, workflow
from agent_workflow.storage import OperationalError, ROOT, atomic, digest

class TTY(io.StringIO):
    def isatty(self): return True

class Presentation(unittest.TestCase):
    def test_auto_color_and_opt_out(self):
        with patch.object(terminal, "MODE", "auto"), patch.dict(os.environ, {"TERM":"xterm", "NO_COLOR":""}):
            self.assertIn("\033[", terminal.paint("awaiting-input", "warning", TTY()))
            self.assertEqual(terminal.paint("plain", stream=io.StringIO()), "plain")
            with patch.dict(os.environ, {"NO_COLOR":"1"}):
                self.assertEqual(terminal.paint("plain", stream=TTY()), "plain")
            with patch.dict(os.environ, {"TERM":"dumb"}):
                self.assertEqual(terminal.paint("plain", stream=TTY()), "plain")

    def test_link_osc8_and_plain_fallback(self):
        url=review.url("demo", {"id":"draft", "sha256":"a"*64})
        with patch("sys.stdout", TTY()), patch.dict(os.environ, {"TERM":"xterm"}):
            self.assertIn("\033]8;;"+url, terminal.link("Review",url))
        with patch("sys.stdout", io.StringIO()):
            self.assertEqual(terminal.link("Review",url), "Review: "+url)

    def test_json_stays_plain_even_with_forced_color(self):
        data={"run_id":"demo","status":"approved"}
        out=TTY()
        with patch.object(cli,"summary",return_value=data), patch.object(terminal,"MODE","always"), redirect_stdout(out):
            cli.print_summary("demo",True)
        self.assertNotIn("\033",out.getvalue())
        self.assertIn('"approved"',out.getvalue())

    def test_open_input_cannot_become_a_decision(self):
        with patch.object(review,"open_review",side_effect=OperationalError("offline")), \
             patch.object(workflow,"answer") as answer, redirect_stdout(io.StringIO()):
            self.assertTrue(cli.review_input("demo","open"))
            self.assertFalse(cli.review_input("demo","30 days"))
            answer.assert_not_called()

class Review(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.local=Path(self.tmp.name)
        self.patches=[patch.object(records,"LOCAL",self.local),patch.object(records,"DB",self.local/"state.sqlite"),
            patch.object(review,"LOCAL",self.local),patch.object(review,"Herdr"),
            patch.dict(os.environ,{"HERDR_ENV":"1","HERDR_SOCKET_PATH":"/fixture.sock","HERDR_WORKSPACE_ID":"elsewhere"}),
            patch.object(review.shutil,"which",return_value="/usr/bin/vim")]
        mocks=[p.start() for p in self.patches]
        self.h=mocks[3].return_value
        records.create_run({"run_id":"demo","target":{"socket":"/fixture.sock","workspace":"run-space"}})
        self.file=records.run_dir("demo")/"artifacts/draft.md"
        atomic(self.file,"# Exact review content")
        self.ref={"id":"draft","path":"artifacts/draft.md","sha256":digest(self.file.read_bytes())}
        atomic(self.file.with_suffix(".json"),{"manifest":self.ref})
        self.layout=None
        def call(method,params=None):
            if method=="tab.list":
                return {"tabs":[] if self.layout is None else [{"tab_id":"run-space:t9","workspace_id":"run-space"}]}
            if method=="layout.export": return {"layout":self.layout}
            return {}
        def create(workspace,label,command,cwd):
            self.layout={"workspace_id":workspace,"tab_id":"run-space:t9","root":{"type":"pane","command":command}}
            return self.layout
        self.h.call.side_effect=call
        self.h.create.side_effect=create

    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.tmp.cleanup()

    def test_link_matches_manifest_and_rejects_arbitrary_targets(self):
        manifest=tomllib.loads((ROOT/"herdr-plugin/herdr-plugin.toml").read_text())
        target=review.url("demo",self.ref)
        self.assertRegex(target,manifest["link_handlers"][0]["pattern"])
        self.assertEqual(review.parse_url(target),("demo","draft",self.ref["sha256"]))
        for bad in (target+"?cmd=rm",target+"\n",target.replace("demo","../demo"),target.replace("awp.invalid","evil.test"),"file:///etc/passwd"):
            with self.assertRaises(OperationalError):review.parse_url(bad)

    def test_open_reuses_editor_targets_run_workspace_and_preserves_state(self):
        before=records.get_run("demo")
        files={p:p.read_bytes() for p in records.run_dir("demo").rglob("*") if p.is_file()}
        review.open_review("demo","draft",self.ref["sha256"])
        review.open_review("demo","draft",self.ref["sha256"])
        self.h.create.assert_called_once()
        argv=self.h.create.call_args.args[2]
        self.assertEqual(self.h.create.call_args.args[0],"run-space")
        self.assertIn("-M",argv)
        self.assertIn("NONE",argv)
        self.assertEqual(argv[-2:],["--",str(self.file)])
        self.assertEqual(records.get_run("demo"),before)
        self.assertEqual({p:p.read_bytes() for p in files},files)
        self.h.call.assert_any_call("tab.focus",{"tab_id":"run-space:t9"})

    def test_stale_link_and_changed_bytes_refused_before_launch(self):
        with self.assertRaisesRegex(OperationalError,"hash"):
            review.open_review("demo","draft","0"*64)
        self.file.write_text("changed")
        with self.assertRaisesRegex(OperationalError,"path/hash"):
            review.open_review("demo","draft",self.ref["sha256"])
        self.h.create.assert_not_called()

    def test_missing_context_wrong_server_and_path_escape_refused(self):
        with patch.dict(os.environ,{"HERDR_ENV":""}):
            with self.assertRaisesRegex(OperationalError,"inside Herdr"):review.open_review("demo","draft")
        with patch.dict(os.environ,{"HERDR_SOCKET_PATH":"/other.sock"}):
            with self.assertRaisesRegex(OperationalError,"another Herdr server"):review.open_review("demo","draft")
        self.ref["path"]="../../../../etc/passwd"
        atomic(self.file.with_suffix(".json"),{"manifest":self.ref})
        with self.assertRaisesRegex(OperationalError,"path/hash"):review.open_review("demo","draft")
        self.h.create.assert_not_called()

    def test_lost_create_response_is_not_automatically_retried(self):
        self.h.create.side_effect=TimeoutError("lost response")
        with self.assertRaises(TimeoutError): review.open_review("demo","draft")
        self.h.create.assert_called_once()

if __name__=="__main__": unittest.main()
