"""Native noninteractive CLIs. No SDK, shell dispatch, session reuse or retries."""
import json
import os
from pathlib import Path
import subprocess
from ..storage import OperationalError, atomic, canonical, digest

VERSIONS = {"claude": "2.1.296", "codex": "0.162.1"}
DISABLED = ("hooks", "plugins", "apps", "shell_tool", "shell_snapshot", "multi_agent",
            "multi_agent_v2", "browser_use", "browser_use_external", "computer_use",
            "image_generation", "view_image", "memories", "goals", "tool_suggest",
            "code_mode", "code_mode_host", "unified_exec", "request_permissions_tool")

def obj(properties):
    return {"type":"object", "properties":properties, "required":list(properties), "additionalProperties":False}

def schema(packet):
    string={"type":"string"}
    def array(item): return {"type":"array", "items":item}
    result = obj({"schema_version":{"type":"integer","enum":[packet.get("schema_version",1)]},
        "operation_id":{"type":"string","enum":[packet["operation_id"]]},
        "packet_sha256":{"type":"string","enum":[digest(canonical(packet))]},
        "content":string,
        "findings":array(obj({"id":string,"severity":{"type":"string","enum":["minor","substantive"]},"evidence":string,"requested_change":string})),
        "dispositions":array(obj({"id":string,"status":{"type":"string","enum":["addressed"]},"rationale":string})),
        "questions":array(obj({"id":string,"question":string,"options":array(string),"recommendation":string,"consequences":string})),
        "needs_revision":{"type":"boolean"}})
    if packet.get("schema_version")==2:
        question = obj({"id":string,"question":string,"options":array(string),
            "recommendation":string,"consequences":string,"topic_key":string,"rationale":string,
            "evidence_refs":array(string),"related_decision_refs":array(string),"reconsideration_reason":string})
        result["properties"]["questions"]=array(question)
    if packet.get("finding_contract_version") == 2:
        properties = result["properties"]
        properties.pop("findings")
        properties.pop("dispositions")
        properties["new_findings"] = array(obj({"severity":{"type":"string","enum":["minor","substantive"]},
                                                  "evidence":string,"requested_change":string}))
        properties["assessments"] = array(obj({"finding_ref":string,
                                               "assessment":{"type":"string","enum":["open","proposed-addressed"]},
                                               "evidence":string,"rationale":string}))
        result["required"] = list(properties)
    return result

def prompt(packet):
    if packet.get("schema_version")==2:
        instruction = ("You are a text-only drafting/review worker. All context is in the packet. "
            "No tools, filesystem access, browsing, delegation or human approval. Return the required JSON, "
            "schema_version 2, echo operation_id and packet_sha256. Content is provisional Markdown, under 450 words. "
            "Use the selected context and accepted decisions; source approval metadata is authoritative even if "
            "an immutable historical artifact says not approved. Do not reopen accepted choices. "
            "Independently identify substantive product choices the supplied evidence cannot settle. "
            "If one blocks your task, return it in questions with 2–3 options, recommendation, consequences, "
            "rationale, stable topic_key and evidence_refs drawn exactly from evidence_ids. "
            "Do not invent a choice just to use the gate; do not silently decide an unresolved substantive choice. "
            "Mark dependent text provisional. One new question is this prototype's per-run limit. "
            "For a NEW question, related_decision_refs=[] and reconsideration_reason is an empty string. "
            "Only propose reconsideration with concrete new evidence, the prior qualified decision references "
            "and a reason. Answered questions must not recur. Workers never answer questions themselves. "
            "Critique independently; findings use unique C1-/C2- IDs and evidence. Revisions apply all accepted "
            "answers and disposition supplied open findings as addressed only when actually resolved. "
            "If a question blocks a finding, leave it open and explain. Do not request a third review.\n"
            )
        if packet.get("prompt_contract") == "product-spec-v1":
            instruction = instruction.replace("Content is provisional Markdown, under 450 words. ",
                "Content is a provisional product specification in Markdown, under 1,200 words. "
                "Make each required capability and exclusion browser-verifiable. Include data fields, "
                "local persistence, supersession semantics, acceptance checks and a short manual walkthrough. "
                "Raise the unresolved editing-policy tradeoff as a structured question; do not supply the human answer. ")
        if packet.get("finding_contract_version") == 2:
            if packet.get("stage") != "draft":
                instruction = instruction.replace(
                    "Raise the unresolved editing-policy tradeoff as a structured question; do not supply the human answer. ",
                    "Honor the recorded editing-policy answer; raise only genuinely new unresolved choices. ")
            instruction = instruction.replace(
                "Critique independently; findings use unique C1-/C2- IDs and evidence. Revisions apply all accepted "
                "answers and disposition supplied open findings as addressed only when actually resolved. "
                "If a question blocks a finding, leave it open and explain. Do not request a third review.\n",
                "Critique independently. Apply accepted answers; leave choices that need human input provisional. "
                "Do not request a third review.\n")
            instruction += ("Finding contract v2: put only genuinely new issues in new_findings, without IDs. "
                "The application assigns durable identities. Reassess supplied issues in assessments using an exact "
                "finding_ref from open_findings or finding_dispositions. Assessment open means still unresolved; "
                "proposed-addressed is only a worker claim and does not resolve the finding. "
                "Give evidence from the current candidate and a rationale. Assess each relevant open finding. "
                "Do not repeat an existing finding as a new finding or fabricate a human decision.\n")
        return instruction + canonical({"response_identity":{"operation_id":packet["operation_id"],"packet_sha256":digest(canonical(packet))},"context_packet":packet})
    return ("You are one text-only worker in a bounded drafting trial. All context is supplied below. "
        "Do not read files, execute tools, browse, delegate, or contact other workers. "
        "Return exactly the required JSON result. Output content is the Markdown artifact, not a filename. "
        "Echo operation_id and packet_sha256 from the response identity below. "
        "The application schedules the planned human decision after the first critique; leave questions empty. "
        "Never choose the unresolved policy yourself. The initial draft must clearly show both conflicting "
        "requirements as provisional so the independent critique can identify their incompatibility. "
        "Critics should identify concrete contradictions or omissions with evidence and unique finding IDs "
        "prefixed C1- or C2-. Revisions must address each supplied open finding and honor the accepted decision. "
        "Do not reopen accepted decisions. Request a second revision only for remaining material defects. "
        "Keep Markdown under 450 words. Never approve on the operator's behalf.\n"
        + canonical({"response_identity":{"operation_id":packet["operation_id"],"packet_sha256":digest(canonical(packet))},"context_packet":packet}))

def verify_role(role):
    provider=role.get("provider")
    if provider not in VERSIONS or not role.get("model") or not role.get("reasoning"):
        raise OperationalError("Real worker requires provider, explicit model and reasoning")
    binary=Path(role.get("binary", ""))
    if not binary.is_absolute() or not binary.is_file():
        raise OperationalError("Worker binary must be an existing absolute path")
    version=subprocess.run([str(binary),"--version"],capture_output=True,text=True,timeout=10)
    if version.returncode or VERSIONS[provider] not in version.stdout:
        raise OperationalError("CLI version changed or unsupported; inspect before dispatch")
    return version.stdout.strip()

def build(directory,packet):
    role=packet["execution"]
    version=verify_role(role)
    provider=packet["provider"]
    if role["provider"]!=provider or role["model"]!=packet["model"]:
        raise OperationalError("Execution identity mismatch")
    schema_path=directory/"output-schema.json"
    atomic(schema_path,schema(packet),immutable=True)
    text=prompt(packet)
    input_path=directory/"prompt.txt"
    atomic(input_path,text,immutable=True)
    # Existing auth remains in HOME/CODEX_HOME. Never copy credential files into a run.
    keys=("PATH","HOME","LANG","LC_ALL","TERM","COLORTERM","CODEX_HOME","CLAUDE_CONFIG_DIR",
          "XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS","SSL_CERT_FILE","SSL_CERT_DIR")
    env={k:os.environ[k] for k in keys if k in os.environ}
    env.update(PYTHONUNBUFFERED="1",DISABLE_AUTOUPDATER="1")
    if provider=="claude":
        argv=[role["binary"],"--print","--safe-mode","--setting-sources","",
              "--strict-mcp-config","--mcp-config",'{"mcpServers":{}}',"--tools","",
              "--permission-mode","dontAsk","--settings",'{"disableAllHooks":true}',
              "--model",role["model"],"--effort",role["reasoning"],
              "--no-session-persistence","--output-format","stream-json","--verbose",
              "--include-partial-messages","--json-schema",canonical(schema(packet))]
        env["CLAUDE_CODE_SAFE_MODE"]="1"
        restrictions={"tools":"empty built-in tool list", "permission_mode":"dontAsk", "hooks":"safe-mode + disableAllHooks", "mcp":"strict empty", "session_history":"disabled"}
    else:
        argv=[role["binary"],"--no-daemon","--ask-for-approval","never","exec",
              "--ignore-user-config","--strict-config","--skip-git-repo-check","--ephemeral",
              "--sandbox","read-only","--model",role["model"],"--json",
              "--output-schema",str(schema_path),"--output-last-message",str(directory/"native-final.json"),
              "-c",'model_reasoning_effort='+json.dumps(role["reasoning"]),
              "-c",'web_search="disabled"',"-c","project_doc_max_bytes=0",
              "-c","skills.include_instructions=false","-c","skills.bundled.enabled=false",
              "-c","tools.update_plan.enabled=false","-c","tools.experimental_request_user_input.enabled=false"]
        for feature in DISABLED: argv += ["--disable",feature]
        argv += ["-"]
        restrictions={"sandbox":"read-only","approval_policy":"never (deny escalation)",
            "features_disabled":list(DISABLED),"web_search":"disabled","user_config":"ignored; existing authentication retained",
            "remaining_tool_caveat":"apply_patch may remain advertised, but read-only sandbox denies writes; any observed tool event fails the stage"}
    return {"argv":argv,"env":env,"stdin":input_path,
        "record":{"provider":provider,"requested_model":role["model"],"reasoning":role["reasoning"],
            "cli_version":version,"argv":argv,"input_transport":"read-only prompt.txt on stdin then EOF; not the terminal",
            "prompt_sha256":digest(text),"packet_sha256":digest(canonical(packet)),"permissions":restrictions,
            "native_retries":"CLI/provider transport retries may occur; controller never re-dispatches a failed/uncertain stage"}}

def inspect_event(provider,event):
    """Treat observed tool activity as scope failure, never as a workflow answer."""
    if provider=="claude":
        if event.get("type")=="system" and event.get("subtype")=="init":
            if event.get("mcp_servers") or any(t!="StructuredOutput" for t in event.get("tools",[])):
                raise OperationalError("Claude advertised tools/MCP outside text-only scope")
            if event.get("permissionMode") not in (None,"dontAsk"):
                raise OperationalError("Claude permission mode mismatch")
        for block in event.get("message",{}).get("content",[]) if isinstance(event.get("message"),dict) else []:
            if isinstance(block,dict) and block.get("type")=="tool_use" and block.get("name")!="StructuredOutput":
                raise OperationalError("Claude attempted a tool outside text-only scope")
    else:
        item=event.get("item",{})
        if item.get("type") in ("command_execution","file_change","mcp_tool_call","web_search","collab_tool_call"):
            raise OperationalError("Codex attempted a tool outside text-only scope")

def finish(provider,events,directory):
    for e in events: inspect_event(provider,e)
    if provider=="claude":
        final=[e for e in events if e.get("type")=="result"]
        if len(final)!=1 or final[0].get("is_error") or final[0].get("subtype")!="success":
            raise OperationalError("Claude did not return one successful structured result")
        result=final[0].get("structured_output")
        if not isinstance(result,dict):
            raise OperationalError("Claude structured_output is absent; no retry")
        init=next((e for e in events if e.get("type")=="system" and e.get("subtype")=="init"),{})
        runtime={"resolved_model":init.get("model"),"model_usage":final[0].get("modelUsage",{}),
            "reported_tools":init.get("tools"),"reported_mcp":init.get("mcp_servers"),
            "reported_permission_mode":init.get("permissionMode"),"num_turns":final[0].get("num_turns"),
            "partial_events":sum(e.get("type")=="stream_event" for e in events)}
    else:
        diagnostics = [e["item"].get("message", "") for e in events if e.get("item", {}).get("type") == "error"]
        # Installed 0.162.1 emits this diagnostic when code mode is deliberately disabled.
        allowed = "Code Mode is unavailable because code-mode host is disabled. Code mode will fail closed; enable `features.code_mode_host` and install `codex-code-mode-host`."
        if any(message != allowed for message in diagnostics):
            raise OperationalError("Codex reported an unexpected item error; no retry")
        if any(e.get("type") in ("turn.failed","error") for e in events) or sum(e.get("type")=="turn.completed" for e in events)!=1:
            raise OperationalError("Codex did not finish exactly one turn; no retry")
        result=json.loads((directory/"native-final.json").read_text())
        runtime={"resolved_model":"not independently reported by exec JSONL; explicit launch model recorded",
                 "thread_id":next((e.get("thread_id") for e in events if e.get("type")=="thread.started"),None),
                 "event_types":sorted({e.get("type","") for e in events}), "diagnostics":diagnostics,
                 "usage":next((e.get("usage") for e in events if e.get("type")=="turn.completed"),None)}
    atomic(directory/"runtime-result.json",runtime,immutable=True)
    return result
