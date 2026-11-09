"""Approval input transport only. Never submits anything to the real workflow."""
import json
import os
import pty
import select
import subprocess
import sys
import time
import unittest
from agent_workflow.terminal import normalize_command
from agent_workflow import terminal
from agent_workflow.storage import OperationalError

class Input(unittest.TestCase):
    command = "approve " + "a" * 64

    def test_color_enabled_coordinator_runner_label(self):
        prior = terminal.MODE
        try:
            terminal.MODE = "always"
            self.assertIn("Coordinator", terminal.paint("Coordinator", "coordinator"))
            self.assertIn("Future role", terminal.paint("Future role", "future-role"))
        finally:
            terminal.MODE = prior

    def test_normal_and_framed_pastes_keep_exact_hash(self):
        self.assertEqual(normalize_command(self.command),self.command)
        self.assertEqual(normalize_command("\x1b[200~"+self.command+"\x1b[201~"),self.command)
        self.assertEqual(normalize_command("approve\t"+"a"*64),self.command)
        self.assertEqual(normalize_command("approve\u00a0"+"a"*64),self.command)
        self.assertEqual(normalize_command("approve wrong"),"approve wrong")

    def test_multiple_commands_and_unrecognized_controls_refused(self):
        for text in ["open\n"+self.command,"\x1b[200~open\n"+self.command+"\x1b[201~",
                     "\x1b[201~"+self.command,"\x1b[31m"+self.command,"\u200b"+self.command]:
            with self.assertRaises(OperationalError):normalize_command(text)

    def test_readline_collects_paste_and_rejects_multiline_in_real_pty(self):
        code = ('import json; from agent_workflow.terminal import read_command; '
                'from agent_workflow.storage import OperationalError\n'
                'try: print("RESULT="+json.dumps(read_command("PROMPT> ")))\n'
                'except OperationalError as e: print("REFUSED="+str(e))\n')
        for payload,expected in [(self.command,"RESULT="+json.dumps(self.command)),
                                 ("open\n"+self.command,"REFUSED=Paste one command only")]:
            master,slave=pty.openpty()
            env={**os.environ,"TERM":"xterm-256color"}
            child=subprocess.Popen([sys.executable,"-c",code],stdin=slave,stdout=slave,stderr=slave,env=env)
            os.close(slave)
            output=b"";deadline=time.monotonic()+5
            try:
                while b"PROMPT> " not in output and time.monotonic()<deadline:
                    if select.select([master],[],[],.1)[0]:output+=os.read(master,65536)
                self.assertIn(b"PROMPT> ",output)
                os.write(master,b"\x1b[200~"+payload.encode()+b"\x1b[201~\r")
                while child.poll() is None and time.monotonic()<deadline:
                    if select.select([master],[],[],.1)[0]:
                        try:output+=os.read(master,65536)
                        except OSError:break
                child.wait(timeout=2)
                self.assertIn(expected,output.decode(errors="replace"))
            finally:
                if child.poll() is None:child.kill();child.wait()
                os.close(master)

class NaturalApproval(unittest.TestCase):
    def test_explicit_phrases_bind_displayed_version_only(self):
        from agent_workflow.cli import final_review_input
        shown={"artifact_sha256":"a"*64}
        for phrase in ("approve", "approve this revision", "I approve this revision."):
            self.assertEqual(final_review_input(phrase,shown)[:2],("approved","a"*64))
        self.assertEqual(final_review_input("approve "+"b"*64,shown)[1],"b"*64)
        for phrase in ("yes", "looks good", "do not approve", "maybe", "reject "):
            with self.assertRaises(OperationalError):final_review_input(phrase,shown)

    def test_changed_review_or_decisions_cannot_receive_old_consent(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        from agent_workflow import cli, workflow
        shown={"request_id":"A1","artifact_sha256":"a"*64,"decision_revision":1}
        for changed in ({**shown,"artifact_sha256":"b"*64},
                        {**shown,"decision_revision":2},None):
            with patch.object(workflow,"state",return_value=SimpleNamespace(values={"pending":changed})), \
                 patch.object(workflow,"answer") as answer:
                with self.assertRaisesRegex(OperationalError,"changed"):
                    cli.answer_displayed("fixture",shown,"approved","Approved through controller","a"*64)
                answer.assert_not_called()
        with patch.object(workflow,"state",return_value=SimpleNamespace(values={"pending":shown})), \
             patch.object(workflow,"answer") as answer:
            cli.answer_displayed("fixture",shown,"approved","Approved through controller","a"*64)
            answer.assert_called_once_with("fixture","A1","approved","Approved through controller","a"*64)
