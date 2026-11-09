"""Native adapter contract parity using stubs and saved events; no model calls."""
import argparse
import json
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace
from unittest.mock import patch

from agent_workflow import native
from agent_workflow.adapters import real
from agent_workflow.storage import canonical,digest

def rust(binary,action,value):
    return json.loads(subprocess.check_output([str(binary),action],input=json.dumps(value).encode()))

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    binary=args.binary.resolve();evidence=args.evidence.resolve()
    evidence.mkdir(parents=True,exist_ok=False)
    cases=[]
    for schema_version,finding_version,contract,stage,provider,model in [
        (1,1,None,"draft","claude","sonnet"),
        (2,1,None,"draft","claude","sonnet"),
        (2,2,None,"draft","claude","sonnet"),
        (2,1,"product-spec-v1","revision-1","codex","gpt-6.1-sol"),
        (2,2,"product-spec-v1","draft","claude","sonnet"),
        (2,2,"product-spec-v1","revision-1","codex","gpt-6.1-sol"),
    ]:
        role={"provider":provider,"model":model,"reasoning":"high",
              "binary":str(evidence/(provider+"-stub"))}
        packet={"schema_version":schema_version,"finding_contract_version":finding_version,
                "operation_id":"fixture-op","stage":stage,"provider":provider,"model":model,
                "execution":role,"brief":"Unicode café and 1.0"}
        if contract:packet["prompt_contract"]=contract
        py_schema=real.schema(packet)
        rs_schema=rust(binary,"adapter_schema",{"packet":packet})
        assert rs_schema=={"ok":True,"value":py_schema}
        py_prompt=real.prompt(packet)
        rs_prompt=rust(binary,"adapter_prompt",{"packet":packet})
        assert rs_prompt=={"ok":True,"value":py_prompt}
        directory=evidence/(f"case-{len(cases)}")
        directory.mkdir()
        with patch.object(real,"verify_role",return_value="stub version"):
            py_argv=real.build(directory,packet)["argv"]
        rs_argv=rust(binary,"adapter_argv",{"packet":packet,"directory":str(directory)})
        assert rs_argv=={"ok":True,"value":py_argv}
        cases.append({"schema_version":schema_version,"finding_version":finding_version,
            "contract":contract,"stage":stage,"provider":provider,
            "schema_sha256":digest(canonical(py_schema)),"prompt_sha256":digest(py_prompt),
            "argv_count":len(py_argv)})
    for provider,version in (("claude",real.VERSIONS["claude"]),("codex",real.VERSIONS["codex"])):
        binary_file=evidence/(provider+"-stub")
        binary_file.write_text("#!/bin/sh\nprintf '%s\\n' '"+provider+" "+version+"'\n")
        binary_file.chmod(0o700)
        role={"provider":provider,"model":"fixture","reasoning":"high","binary":str(binary_file)}
        py=real.verify_role(role)
        rs=rust(binary,"adapter_verify",{"role":role,"version_output":py})
        assert rs=={"ok":True,"value":py}
        stale=rust(binary,"adapter_verify",{"role":role,"version_output":"old unsupported version"})
        assert stale["ok"] is False
    source_env={"HOME":"/fixture/home","PATH":"/fixture/bin","LANG":"C.UTF-8",
                "HERDR_PANE_ID":"must-not-leak","CODEX_HOME":"/fixture/auth-existing"}
    for provider in ("claude","codex"):
        with patch.dict(os.environ,source_env,clear=True),patch.object(real,"verify_role",return_value="stub"):
            packet={"schema_version":2,"operation_id":"fixture","provider":provider,"model":"fixture",
                    "execution":{"provider":provider,"model":"fixture","reasoning":"high",
                                 "binary":str(evidence/(provider+"-stub"))}}
            directory=evidence/(provider+"-env")
            directory.mkdir()
            py=real.build(directory,packet)["env"]
        rs=rust(binary,"adapter_env",{"provider":provider,"source_env":source_env})
        assert rs=={"ok":True,"value":py},(provider,py,rs)
        assert "HERDR_PANE_ID" not in py
    events=[
        ("claude",{"type":"system","subtype":"init","tools":["Bash"]},False),
        ("claude",{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read"}]}},False),
        ("codex",{"type":"item.started","item":{"type":"command_execution"}},False),
        ("claude",{"type":"system","subtype":"init","tools":["StructuredOutput"],"mcp_servers":[],"permissionMode":"dontAsk"},True),
        ("codex",{"type":"item.completed","item":{"type":"agent_message","text":"safe"}},True),
    ]
    for provider,event,expected in events:
        try:real.inspect_event(provider,event);py=True
        except Exception:py=False
        rs=rust(binary,"adapter_event",{"provider":provider,"event":event})
        assert py==expected==rs["ok"]
    result={"content":"candidate","operation_id":"fixture"}
    for provider,stream,final in [
        ("claude",[{"type":"system","subtype":"init","model":"sonnet","tools":[],
                   "mcp_servers":[],"permissionMode":"dontAsk"},{"type":"stream_event"},
                   {"type":"result","subtype":"success","structured_output":result}],None),
        ("codex",[{"type":"thread.started","thread_id":"fixture"},{"type":"turn.completed",
                   "usage":{"input_tokens":3}}],canonical(result)),
    ]:
        directory=evidence/(provider+"-finish")
        directory.mkdir()
        if final:(directory/"native-final.json").write_text(final)
        py_result=real.finish(provider,stream,directory)
        py_runtime=json.loads((directory/"runtime-result.json").read_text())
        rs=rust(binary,"adapter_finish",{"provider":provider,"events":stream,"native_final":final})
        assert rs=={"ok":True,"value":{"result":py_result,"runtime":py_runtime}},(provider,rs,py_runtime)
        cases.append({"finish":provider,"runtime_sha256":digest(canonical(py_runtime))})
    bad=[("claude",[{"type":"result","subtype":"error"}],None),
         ("codex",[{"type":"turn.failed"}],canonical(result)),
         ("codex",[{"type":"item.completed","item":{"type":"error","message":"unexpected"}},
                   {"type":"turn.completed"}],canonical(result))]
    for provider,stream,final in bad:
        rs=rust(binary,"adapter_finish",{"provider":provider,"events":stream,"native_final":final})
        assert not rs["ok"]
    plan_envelope={"project_key":"a"*32,"project":"/fixture/project","run_id":"run",
        "brief_path":"brief.md","brief_sha256":"b","config_path":"workflow.toml",
        "config_sha256":"c","rules_sha256":"r","context_snapshot":{"selection_path":"context.toml"},
        "context_digest":"x","application_version":"0.3.0","protocol_version":1,
        "target":{"workspace":"fixture"},"config":{"prompt_contract":"product-spec-v1",
            "review_limit":2,"max_invocations":5,"roles":{"author":{"provider":"claude","model":"sonnet",
            "reasoning":"medium","binary":"/fixture/claude"},"critic":{"provider":"codex",
            "model":"gpt-6.1-sol","reasoning":"high","binary":"/fixture/codex"}}}}
    plan_digest=digest(canonical(native.plan(plan_envelope)))
    assert rust(binary,"native_check",{"envelope":plan_envelope,"supplied_digest":plan_digest})["ok"]
    changed=json.loads(json.dumps(plan_envelope));changed["config"]["roles"]["critic"]["reasoning"]="medium"
    assert not rust(binary,"native_check",{"envelope":changed,"supplied_digest":plan_digest})["ok"]
    assert not native.plan(changed)==native.plan(plan_envelope)
    (evidence/"adapter-comparison.json").write_text(json.dumps({"schema_version":1,
        "baseline":__import__("agent_workflow").__file__,"cases":cases,
        "event_cases":len(events),"invalid_finishes":len(bad),"stale_plan_refused":True},indent=2)+"\n")
    print("Stage 2 adapters: schemas/prompts/argv, stub versions, env, events, finishes and stale plan passed")

if __name__=="__main__":main()
