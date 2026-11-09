"""Native adapter contract tests; no authentication, model calls or real Herdr."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from agent_workflow.adapters import real
from agent_workflow import artifacts, records, workflow
from agent_workflow.storage import OperationalError, atomic, canonical, digest

class Native(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.path=Path(self.tmp.name)
        self.packet={"provider":"claude","model":"opus","operation_id":"trial-draft","stage":"draft", "role":"author",
                     "execution":{"provider":"claude","binary":"/fixture/claude","model":"opus","reasoning":"high"},
                     "current_draft":"fixture", "decisions":[{"answer":"Offline only"}], "open_findings":[], "delta":"change"}
    def tearDown(self):self.tmp.cleanup()

    def test_claude_stdin_and_tool_hook_restrictions(self):
        with patch.object(real,"verify_role",return_value="fixture"), patch.dict(os.environ,{"HOME":"/original/home","HERDR_PANE_ID":"not-inherited"}):
            command=real.build(self.path,self.packet)
        args=command["argv"]
        for flag in ("--safe-mode","--strict-mcp-config","--include-partial-messages","--json-schema","--no-session-persistence"):self.assertIn(flag,args)
        self.assertEqual(args[args.index("--tools")+1],"")
        self.assertEqual(args[args.index("--permission-mode")+1],"dontAsk")
        self.assertNotIn("HERDR_PANE_ID",command["env"])
        self.assertEqual(command["env"]["HOME"],"/original/home")
        text=command["stdin"].read_text()
        for value in ("Offline only","change",digest(canonical(self.packet))):self.assertIn(value,text)
        self.assertEqual(command["record"]["prompt_sha256"],digest(text))

    def test_codex_stdin_schema_model_and_readonly_no_hooks(self):
        self.packet.update(provider="codex",model="gpt-6.1-sol")
        self.packet["execution"].update(provider="codex",model="gpt-6.1-sol",binary="/fixture/codex")
        with patch.object(real,"verify_role",return_value="fixture"):command=real.build(self.path,self.packet)
        args=command["argv"]
        self.assertEqual(args[-1],"-")
        self.assertEqual(args[args.index("--sandbox")+1],"read-only")
        self.assertEqual(args[args.index("--ask-for-approval")+1],"never")
        self.assertEqual(args[args.index("--model")+1],"gpt-6.1-sol")
        self.assertIn('model_reasoning_effort="high"',args)
        self.assertIn("hooks",args);self.assertIn("--ignore-user-config",args)
        self.assertNotIn("--dangerously-bypass-approvals-and-sandbox",args)
        schema=json.loads((self.path/"output-schema.json").read_text())
        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(schema["properties"]["packet_sha256"]["enum"],[digest(canonical(self.packet))])

    def test_unexpected_tools_and_bad_output_stop(self):
        for provider,event in [("claude",{"type":"system","subtype":"init","tools":["Bash"]}),
             ("claude",{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read"}]}}),
             ("codex",{"type":"item.started","item":{"type":"command_execution"}})]:
            with self.assertRaises(OperationalError):real.inspect_event(provider,event)
        with self.assertRaises(OperationalError):real.finish("claude",[{"type":"result","subtype":"error"}],self.path)
        with self.assertRaises(OperationalError):real.finish("codex",[{"type":"turn.failed"}],self.path)

    def test_unexpected_codex_item_error_is_not_success(self):
        events=[{"type":"item.completed","item":{"type":"error","message":"unexpected provider failure"}}, {"type":"turn.completed"}]
        with self.assertRaisesRegex(OperationalError,"unexpected item error"):
            real.finish("codex",events,self.path)

    def test_normalizers_preserve_structured_result(self):
        result={"content":"artifact"}
        events=[{"type":"system","subtype":"init","model":"resolved-model","tools":[],"mcp_servers":[],"permissionMode":"dontAsk"},
                {"type":"stream_event"},{"type":"result","subtype":"success","structured_output":result}]
        self.assertEqual(real.finish("claude",events,self.path),result)
        self.assertEqual(json.loads((self.path/"runtime-result.json").read_text())["partial_events"],1)
        other=self.path/"codex";other.mkdir();atomic(other/"native-final.json",result)
        self.assertEqual(real.finish("codex",[{"type":"thread.started","thread_id":"fixture"},{"type":"turn.completed"}],other),result)

    def test_first_critique_schedules_human_choice_and_revision(self):
        config={"trial_id":"m2-cli-01","decision":{"id":"policy","question":"Choose?","options":["a","b"],"recommendation":"a","consequences":"tradeoff"}}
        state={"run_id":"trial","next_stage":"critique-1","completed":["draft"],"decisions":[],"findings":{},"current":{"id":"draft"}}
        result={"content":"critique","findings":[],"dispositions":[],"questions":[],"needs_revision":False}
        with patch.object(workflow.records,"get_run",return_value={"envelope":{"config":config}}), \
             patch.object(workflow.artifacts,"packet",return_value={"operation_id":"op","role":"critic","provider":"codex","model":"sol"}), \
             patch.object(workflow.records,"reserve"), patch.object(workflow.records,"run_dir",return_value=self.path), \
             patch.object(workflow.records,"effective_target",return_value={}),patch.object(workflow,"prepare"), \
             patch.object(workflow,"wait",return_value={"status":"completed"}),patch.object(workflow,"read",return_value=result), \
             patch.object(workflow.artifacts,"publish",return_value={"id":"critique-1","path":"file","sha256":"hash"}):
            change=workflow.worker(state)
        self.assertEqual(change["next_stage"],"revision-1")
        self.assertEqual(change["questions"],[config["decision"]])
        self.assertEqual(workflow.route(change),"prepare_choice")
