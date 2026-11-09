//! Metadata-only provider lifecycle checks. Fixtures cannot perform inference.
use serde_json::{Value,json};
use std::{fs,os::unix::{fs::{PermissionsExt,symlink}},path::Path};
use waywr_backend::{adapters,development,execution::{hash_file,write},provider_selection as providers,session_store::Session};

fn client(path:&Path,provider:&str,revision:u32,complete:bool){
    let flags=if complete{
        "--no-daemon --ask-for-approval --disable --ignore-user-config --ignore-rules --ephemeral --skip-git-repo-check --json --color --model --config --sandbox --output-schema --output-last-message --cd --add-dir --print --safe-mode --restricted --disable-slash-commands --setting-sources --strict-mcp-config --mcp-config --tools --permission-mode --permission-prompts --settings --no-session-persistence --output-format --verbose --include-partial-messages --json-schema --system-prompt --effort"
    }else{"--help --version"};
    let version=if provider=="codex"{format!("codex-cli {revision}.0.0")}else{format!("{revision}.0.0 (Claude Code)")};
    fs::write(path,format!("#!/bin/sh\ncase \"$*\" in\n*--help*) printf '%s\\n' '{flags}';;\n*--version*) printf '%s\\n' '{version}';;\n*) echo 'Metadata-only fixture refuses execution' >&2; exit 99;;\nesac\n")).unwrap();
    fs::set_permissions(path,fs::Permissions::from_mode(0o755)).unwrap();
}
fn project(root:&Path,binary:&Path,provider:&str)->Session{
    let state=root.join(".waywr/state");fs::create_dir_all(&state).unwrap();
    let key="0123456789abcdef0123456789abcdef";
    let profile=json!({"provider":provider,"binary":binary,"model":if provider=="codex"{"gpt-6.1-sol"}else{"claude-sonnet-5-5"},"effort":"medium","binary_sha256":hash_file(binary).unwrap(),"context_bytes":98304});
    let coordinator=json!({"provider":"codex","binary":binary,"model":"gpt-6.1-sol","effort":"medium",
        "binary_sha256":hash_file(binary).unwrap(),"context_bytes":98304});
    let preferences=json!({"schema_version":2,"profile":"development-profiles-v1","profiles":{"default":profile,"coordinator":coordinator},
        "coordinator":"coordinator","worker_default":"default","roles":{
            "coordinator":{"default":"coordinator","allowed":["coordinator"]},
            "implementation":{"default":"default","allowed":["default"]},
            "review":{"default":"default","allowed":["default"]}},
        "fallback":{"coordinator":[],"implementation":[],"review":[]},"concurrency":2,
        "dependency_network":false,"main_policy":"checked-fast-forward","quality_preference":""});
    write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":9,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
    write(&root.join(".waywr/coordinator.json"),&preferences,true).unwrap();
    Session::open(root,&state,key,"provider-updates").unwrap()
}
fn selected(s:&Session)->(Value,Value){
    let (prefs,_)=providers::preferences(s).unwrap();
    let policy=json!({"routing":prefs.allowed_policy().unwrap(),"dependency_network":false});
    (providers::execution(s,"implementation",&Value::Null,None,&policy,"new attempt").unwrap().0,policy)
}

#[test]
fn provider_updates_accept_new_versions_without_profile_edits_and_fence_old_attempts(){
    for provider in ["codex","claude"]{
        let root=tempfile::tempdir().unwrap();let binary=root.path().join(provider);
        client(&binary,provider,900,true);
        let s=project(root.path(),&binary,provider);
        let saved=fs::read(root.path().join(".waywr/coordinator.json")).unwrap();
        let bound=providers::current(&s).unwrap();
        let (first,_)=selected(&s);
        development::preflight(&first).expect("New client version with the required interface should work");
        client(&binary,provider,901,true);
        let (next,policy)=selected(&s);
        development::preflight(&next).expect("A CLI update must not require saving F3");
        development::preflight(&providers::effective(&s).unwrap()).unwrap();
        assert!(development::preflight(&first).unwrap_err().contains("changed"),"An old attempt must retain its executable identity");
        assert_eq!(providers::current(&s).unwrap(),bound);
        assert_eq!(fs::read(root.path().join(".waywr/coordinator.json")).unwrap(),saved);
        let work=json!({"policy":policy});
        let mut packet=json!({"operation_kind":"development-worker","provider":provider,"model":next["model"],"execution":next,"policy":policy});
        assert!(providers::packet_authorized(&s,&work,&packet).unwrap());
        packet["execution"]["client"]["locator"]=json!("/unapproved/other-client");
        assert!(!providers::packet_authorized(&s,&work,&packet).unwrap());
    }
}

#[test]
fn provider_updates_follow_stable_cli_links_and_report_missing_capabilities(){
    let root=tempfile::tempdir().unwrap();let first=root.path().join("client-one");
    let next=root.path().join("client-two");let alias=root.path().join("codex");
    client(&first,"codex",900,true);client(&next,"codex",901,true);symlink(&first,&alias).unwrap();
    let s=project(root.path(),&alias,"codex");let (frozen,_)=selected(&s);
    fs::remove_file(&alias).unwrap();symlink(&next,&alias).unwrap();fs::remove_file(&first).unwrap();
    let (current,_)=selected(&s);
    development::preflight(&current).expect("A stable CLI link may point to its updated installation");
    assert!(development::preflight(&frozen).is_err());
    client(&next,"codex",902,false);
    let (unsupported,_)=selected(&s);
    let error=development::preflight(&unsupported).unwrap_err();
    assert!(error.contains("--")&&error.contains("interface"),"Report the missing capability, not an unsupported version: {error}");
}

#[test]
fn provider_updates_do_not_reject_legacy_roles_from_version_text(){
    let root=tempfile::tempdir().unwrap();let binary=root.path().join("client");
    client(&binary,"codex",900,true);
    for provider in ["codex","claude"]{
        let role=json!({"provider":provider,"binary":binary,"model":"m","reasoning":"medium"});
        assert!(adapters::verify_role(&role,"client future-version").is_ok());
    }
}

#[test]
fn provider_updates_legacy_claude_metadata_has_no_permanent_binary_hash_gate(){
    let root=tempfile::tempdir().unwrap();let binary=root.path().join("claude");
    client(&binary,"claude",900,true);
    // Empty local fixture only exercises login-file presence, never authentication.
    fs::write(root.path().join(".credentials.json"),"{}").unwrap();
    let profile=json!({"provider":"claude","binary":binary,"mode":"native","auth_config_dir":root.path()});
    waywr_backend::claude_chat::preflight(&profile).expect("Legacy metadata should accept the actual interface rather than one historical binary hash");
}

#[test]
fn provider_updates_record_the_frozen_client_contract_before_preparing_an_operation(){
    let root=tempfile::tempdir().unwrap();let binary=root.path().join("codex");client(&binary,"codex",900,true);
    let s=project(root.path(),&binary,"codex");let (execution,policy)=selected(&s);
    let packet=json!({"operation_id":"fixture-op","run_id":"fixture-run","stage":"work","role":"implementation",
        "operation_kind":"development-worker","provider":"codex","model":execution["model"],"execution":execution,"policy":policy});
    let directory=s.workflow.state.join("session/development/fixture/operation");
    waywr_backend::operation::prepare_host(&s.workflow.state,&directory,&packet,&s.key).unwrap();
    assert!(s.workflow.state.join("native-client-identity-v1.json").is_file(),
        "Older applications must not ignore the new per-attempt executable identity");
}

#[test]
#[ignore = "Explicit local CLI metadata check; never generates a model response"]
fn provider_updates_installed_cli_metadata(){
    for (provider,key) in [("codex","WAYWR_TEST_CODEX_METADATA_BINARY"),("claude","WAYWR_TEST_CLAUDE_METADATA_BINARY")]{
        let binary=std::env::var(key).expect("Supply the intended CLI path explicitly");
        let execution=providers::with_client(json!({"profile":"development-profiles-v1","provider":provider,"binary":binary}));
        development::preflight(&execution).unwrap();
        println!("{provider}: required local interface accepted; no model request");
    }
}

#[test]
fn provider_updates_missing_historical_locator_is_ready_via_current_path(){
    const FIXTURE:&str="WAYWR_RELOCATION_METADATA_FIXTURE";
    if let Some(root)=std::env::var_os(FIXTURE){
        let root=std::path::PathBuf::from(root);
        let s=Session::open(&root,&root.join(".waywr/state"),"0123456789abcdef0123456789abcdef","provider-updates").unwrap();
        let readiness=s.development_readiness().unwrap();
        assert_eq!(readiness["status"],"available","A removed historical binary locator must not block the resolved client: {readiness}");
        development::preflight(&readiness["profile"]).unwrap();
        return;
    }
    let root=tempfile::tempdir().unwrap();let old=root.path().join("old");let current=root.path().join("current");
    fs::create_dir(&old).unwrap();fs::create_dir(&current).unwrap();
    client(&old.join("codex"),"codex",900,true);
    let s=project(root.path(),&old.join("codex"),"codex");drop(s);
    client(&current.join("codex"),"codex",901,true);fs::remove_dir_all(old).unwrap();
    // A child gets an isolated PATH; parallel tests and the operator environment
    // are never changed with process-global set_var.
    let result=std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact","provider_updates_missing_historical_locator_is_ready_via_current_path","--nocapture"])
        .env(FIXTURE,root.path()).env("PATH",format!("{}:/usr/bin:/bin",current.display())).output().unwrap();
    assert!(result.status.success(),"{}\n{}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));
    let home=root.path().join("home");let local=home.join(".local/bin");fs::create_dir_all(&local).unwrap();
    client(&local.join("codex"),"codex",902,true);
    let result=std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact","provider_updates_missing_historical_locator_is_ready_via_current_path","--nocapture"])
        .env(FIXTURE,root.path()).env("HOME",home).env("PATH","/usr/bin:/bin").output().unwrap();
    assert!(result.status.success(),"Known user-local clients must also resolve:\n{}",String::from_utf8_lossy(&result.stderr));
}
