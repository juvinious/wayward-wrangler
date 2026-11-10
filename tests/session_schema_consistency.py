"""Check the published envelope against the current service's minor-1 through minor-10 surface.

This is a source contract check, not an assertion that every request succeeds for
every project state. It deliberately does not execute a service or agent.
"""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
schema = json.loads((ROOT / "protocol/session-v1.schema.json").read_text())
service = (ROOT / "backend/src/session_service.rs").read_text()


def check() -> None:
    kinds = set(schema["properties"]["kind"]["enum"])
    minor = schema["x-waywr-minor-1"]
    expected = {
        "host_operations_v1": ["operation.list", "operation.list_result", "operation.inspect", "operation.inspect_result"],
        "output_pages_v1": ["operation.output.read", "operation.output.page"],
        "artifact_pages_v1": ["artifact.read", "artifact.page"],
        "herdr_views_v1": ["view.prepare", "view.prepare_result", "view.sent", "view.sent_result", "view.bind", "view.bind_result"],
    }
    assert minor == expected
    assert schema["x-waywr-minor-2"] == {
        "conversation_v1": ["conversation.read", "conversation.result"],
        "file_candidates_v1": ["artifact.read", "artifact.page"],
    }
    assert all(set(group) <= kinds for group in minor.values())
    assert set(schema["x-waywr-minor-2"]["conversation_v1"]) <= kinds
    assert {"hello", "hello_result", "command", "command_result", "command_get", "snapshot",
            "events_read", "events_result", "task.plan", "task.plan_result", "task.get", "task.result",
            "session.status", "session.status_result", "ui.attach_direct", "ui.attach_direct_result",
            "error"} <= kinds
    dispatch = service.split("    fn dispatch(&self,request:", 1)[1].split("    fn dispatch_view(", 1)[0]
    for request, response in (
        ("operation.list", "operation.list_result"),
        ("operation.inspect", "operation.inspect_result"),
        ("operation.output.read", "operation.output.page"),
        ("artifact.read", "artifact.page"),
        ("view.prepare", "view.prepare_result"),
        ("view.sent", "view.sent_result"),
        ("view.bind", "view.bind_result"),
        ("session.status", "session.status_result"),
    ):
        assert f'"{request}"=>Ok(("{response}"' in dispatch
    assert '"conversation.read"=>if self.conversation' in dispatch
    assert '("conversation.result",self.session.conversation_snapshot()?)' in dispatch
    hello = service.split('let allowed=hello["kind"]', 1)[1].split("if !allowed{return Ok(());}", 1)[0]
    assert 'body["protocol"]["major"]==1' in hello
    assert '"minor":if crate::provider_selection::enabled(&self.session){10}else if self.session.development(){6}else if self.session.continuous(){5}else if self.session.scope_enabled(){4}else if self.session.conversation_chat(){3}else if self.conversation{2}else if self.host{1}else{0}' in hello
    for capability in expected:
        assert f'"{capability}"' in hello
    assert '"capabilities":if crate::provider_selection::enabled(&self.session)' in hello
    assert 'else if self.session.continuous(){json!([' in hello
    assert 'else if self.session.scope_enabled(){json!([' in hello
    assert '"conversation_v1"' in hello
    assert '"file_candidates_v1"' in hello
    assert schema["x-waywr-minor-3"]=={"claude_chat_v1":["conversation.read","conversation.result"]}
    assert schema["x-waywr-minor-4"]=={"task_scope_v1":["task.get","task.result","conversation.read","conversation.result"]}
    assert '"task_scope_v1"' in hello
    assert schema["x-waywr-minor-5"]=={"continuous_scope_v1":["conversation.read","conversation.result","task.get","task.result"]}
    assert '"continuous_scope_v1"' in hello
    assert schema["x-waywr-minor-6"] == {
        "development_v1": ["snapshot.conversation.development", "task.get.scope.operating_policy",
            "authorize_scope.operating_policy_sha256", "development.steer", "development.resume",
            "operation.list", "operation.output.read", "development.cancel"],
        "request_history_v1": ["stable request identity", "history log/summary/projection paths",
            "timestamped attributed public events"],
    }
    for capability in schema["x-waywr-minor-6"]:
        assert f'"{capability}"' in hello
    assert 'body["protocol"]["minor"].as_u64().unwrap_or(0)>=if crate::provider_selection::enabled(&self.session){10}' in hello
    assert "provider_selection_v1" in schema["x-waywr-minor-7"]
    assert '"provider_selection_v1"' in hello
    provider_source=(ROOT / "backend/src/provider_selection.rs").read_text()
    for action in schema["x-waywr-minor-7"]["provider_selection_v1"][1:]:
        assert f'"{action}"' in provider_source
    assert '"conversational_coordination_v1"' in hello
    coordination=(ROOT / "backend/src/coordination.rs").read_text()
    for action in schema["x-waywr-minor-8"]["conversational_coordination_v1"][1:]:
        assert f'"{action}"' in coordination
    assert '"coordinator_recovery_v1"' in hello
    assert "coordinator_recovery_v1" in schema["x-waywr-minor-9"]
    assert '"ordinary_project_workflow_v1"' in hello
    assert "ordinary_project_workflow_v1" in schema["x-waywr-minor-10"]
    assert '"state_schema":if crate::provider_selection::enabled(&self.session){9}' in hello  # IPC minor is not storage schema
    for action in schema["x-waywr-task-scope-actions"]:
        assert f'"{action}"' in service
    for action in schema["x-waywr-claude-chat-actions"]:
        assert f'"{action}"' in service
    live = (ROOT / "backend/src/live_input.rs").read_text()
    for action in schema["x-waywr-live-input-v1"]["actions"]:
        assert f'"{action}"' in live
    assert '"live_input_contract"' in (ROOT / "backend/src/continuous.rs").read_text()
    assert 'body["channel"]=="view" && self.host' in service
    assert '"operation.retry_start"=>' in service  # durable command action, not frame kind
    assert '"view.herdr.open"' not in kinds  # composed CLI action, not an IPC frame


if __name__ == "__main__":
    check()
    print("session schema and minor-1/2/3/4/5/6/7/8/9/10 service surface: OK")
