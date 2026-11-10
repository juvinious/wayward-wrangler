//! Startup renewal of obsolete or stranded sessions. Source/worktrees are not
//! session state. Retired attempts are abandoned, never replayed or reclassified.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs, io,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use waywr_backend::{
    execution::{FileLock, random_token, read, write, write_bytes},
    provider_selection::Preferences,
};

fn err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
fn properties(unit: &str) -> io::Result<BTreeMap<String, String>> {
    let result = Command::new("systemctl")
        .args([
            "--user",
            "show",
            "--property=Id,LoadState,ActiveState,MainPID,ControlGroup,WorkingDirectory,KillMode",
            unit,
        ])
        .output()?;
    if !result.status.success() {
        return Err(err("Cannot inspect session process ownership"));
    }
    Ok(String::from_utf8_lossy(&result.stdout)
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect())
}
fn get<'a>(p: &'a BTreeMap<String, String>, key: &str) -> &'a str {
    p.get(key).map(String::as_str).unwrap_or("")
}
fn empty(p: &BTreeMap<String, String>) -> io::Result<bool> {
    if !matches!(get(p, "ActiveState"), "inactive" | "failed") || get(p, "MainPID") != "0" {
        return Ok(false);
    }
    let group = get(p, "ControlGroup");
    if group.is_empty() {
        return Ok(true);
    }
    let path = Path::new("/sys/fs/cgroup")
        .join(group.trim_start_matches('/'))
        .join("cgroup.events");
    match fs::read_to_string(path) {
        Ok(events) => Ok(events.lines().any(|l| l == "populated 0")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e),
    }
}
fn stop(unit: &str) -> io::Result<()> {
    let result = Command::new("systemctl")
        .args(["--user", "stop", "--no-block", unit])
        .output()?;
    if !result.status.success() {
        return Err(err("Could not stop retired session process"));
    }
    let started = Instant::now();
    let mut killed = false;
    loop {
        if empty(&properties(unit)?)? {
            return Ok(());
        }
        if !killed && started.elapsed() > Duration::from_secs(3) {
            let kill = Command::new("systemctl")
                .args(["--user", "kill", "--kill-whom=all", "--signal=KILL", unit])
                .output()?;
            if !kill.status.success() {
                return Err(err("Could not terminate retired session process"));
            }
            killed = true;
        }
        if started.elapsed() > Duration::from_secs(6) {
            return Err(err("Retired session processes are still stopping"));
        }
        thread::sleep(Duration::from_millis(50));
    }
}
fn op_name(name: &str, key: &str) -> bool {
    name.strip_prefix(&format!("waywr-op-{key}-"))
        .and_then(|s| s.strip_suffix(".service"))
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(a, b)| {
            a.len() == 16
                && b.len() == 32
                && a.bytes()
                    .chain(b.bytes())
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
}
fn stop_session(project: &Path, state: &Path, key: &str) -> io::Result<()> {
    // Prevent new dispatch while terminating this session. No receipt or
    // nonlaunch assertion is fabricated: the whole session is being abandoned.
    let deadline = Instant::now() + Duration::from_secs(3);
    let _fence = loop {
        match FileLock::exclusive(&state.join("supervision/fence.lock")) {
            Ok(lock) => break lock,
            Err(e) if e.starts_with("Already owned:") && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(20))
            }
            Err(e) => return Err(err(e)),
        }
    };
    let service = format!("waywr-{key}.service");
    let current = properties(&service)?;
    if !empty(&current)? {
        let owner = read(&state.join("supervision/owner.json")).map_err(err)?;
        if owner["project_key"] != key {
            return Err(err("Session owner belongs to another project"));
        }
        let release = owner["release_id"]
            .as_str()
            .ok_or_else(|| err("Session owner release missing"))?;
        let pid = get(&current, "MainPID").parse::<u32>().map_err(err)?;
        if pid <= 1
            || get(&current, "Id") != service
            || get(&current, "KillMode") != "control-group"
        {
            return Err(err("Session service ownership is not verifiable"));
        }
        super::rust_runtime::verify_previous_service(
            project,
            state,
            key,
            release,
            &fs::read(format!("/proc/{pid}/cmdline"))?,
            &fs::read_link(format!("/proc/{pid}/exe"))?,
        )?;
        stop(&service)?;
    }
    // Enumerate live units as well as files: an interrupted dispatch may have
    // reached systemd before publishing the rest of its local evidence.
    let result = Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "--all",
            "--plain",
            "--no-legend",
            "--full",
            &format!("waywr-op-{key}-*.service"),
        ])
        .output()?;
    if !result.status.success() {
        return Err(err("Cannot enumerate retired session operations"));
    }
    for line in String::from_utf8_lossy(&result.stdout).lines() {
        let unit = line
            .split_whitespace()
            .next()
            .ok_or_else(|| err("Invalid session unit listing"))?;
        if !op_name(unit, key) {
            return Err(err("Invalid session operation identity"));
        }
        let p = properties(unit)?;
        if empty(&p)? {
            continue;
        }
        let cwd = Path::new(get(&p, "WorkingDirectory")).canonicalize()?;
        if get(&p, "Id") != unit
            || get(&p, "KillMode") != "control-group"
            || !cwd.starts_with(state)
            || cwd == state
        {
            return Err(err(
                "Operation does not belong to the retired state directory",
            ));
        }
        stop(unit)?;
    }
    // Stop/drain the service writer/projector before moving SQLite and logs.
    let _service = FileLock::exclusive(&state.join("session/service.lock")).map_err(err)?;
    Ok(())
}
fn stranded(dir: &Path) -> io::Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() && stranded(&path)? {
            return Ok(true);
        }
        if ty.is_file()
            && entry.file_name() == "uncertain.json"
            && path.parent().is_some_and(|p| {
                p.join("intent.json").is_file()
                    && !p.join("receipt.json").exists()
                    && !p.join("retired.json").exists()
            })
        {
            return Ok(true);
        }
    }
    Ok(false)
}
fn reason(
    project: &Path,
    state: &Path,
    config: &toml::Value,
    manifest: &Value,
) -> io::Result<Option<&'static str>> {
    let prefs = read(&project.join(".waywr/coordinator.json"));
    if !prefs.as_ref().is_ok_and(|v| Preferences::parse(v).is_ok()) {
        return Ok(Some("obsolete-provider-settings"));
    }
    if config
        .get("execution")
        .and_then(|v| v.get("host"))
        .and_then(toml::Value::as_str)
        != Some("systemd-user-v1")
        || config
            .get("conversation")
            .and_then(|v| v.get("schema_version"))
            .and_then(toml::Value::as_integer)
            != Some(6)
    {
        return Ok(Some("obsolete-session-format"));
    }
    if state.join("state-contract.json").exists() {
        let contract = read(&state.join("state-contract.json")).map_err(err)?;
        if contract["project_key"] != config["project_key"].as_str().unwrap_or("") {
            return Err(err("State belongs to another project"));
        }
        if let Some(schema) = contract["state_schema"].as_u64() {
            if schema < manifest["state_schema_min"].as_u64().unwrap_or(0)
                || schema > manifest["state_schema_max"].as_u64().unwrap_or(u64::MAX)
            {
                return Ok(Some("unsupported-session-format"));
            }
        }
    }
    if state.join("state.sqlite").is_file() {
        let saved = waywr_backend::session_store::Session::open_existing(
            project,
            state,
            config["project_key"].as_str().unwrap_or(""),
            "startup-inspection",
        )
        .map_err(err)?;
        if waywr_backend::provider_selection::preferences(&saved)
            .is_ok_and(|(p, _)| p.schema_version != 3)
            || waywr_backend::provider_selection::current(&saved)
                .is_ok_and(|v| !v["profile"]["transport"]["entry"].is_string())
        {
            return Ok(Some("obsolete-coordinator-binding"));
        }
    }
    for name in ["session/coordinator", "session/development", "runs"] {
        if stranded(&state.join(name))? {
            return Ok(Some("abandoned-uncertain-session"));
        }
    }
    Ok(None)
}
fn move_once(from: &Path, to: &Path) -> io::Result<()> {
    match (fs::symlink_metadata(from), fs::symlink_metadata(to)) {
        (Ok(_), Err(e)) if e.kind() == io::ErrorKind::NotFound => {
            fs::rename(from, to)?;
            fs::File::open(from.parent().ok_or_else(|| err("Session parent missing"))?)?
                .sync_all()?;
            fs::File::open(
                to.parent()
                    .ok_or_else(|| err("Retirement parent missing"))?,
            )?
            .sync_all()
        }
        (Err(e), _) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        (Ok(_), Ok(_)) => Err(err("Session retirement destination is already occupied")),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}
fn private_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_dir() => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => fs::create_dir(path)?,
        _ => return Err(err("Session retirement directory must be a real directory")),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
/// Called only on normal service startup, under the project startup lock. It is
/// independent of application version strings and never copies calls/approvals.
pub(super) fn prepare(project: &Path, state: &Path, manifest: &Value) -> io::Result<()> {
    let local = project.join(".waywr");
    let configuration = local.join("project.toml");
    if !configuration.exists() {
        return Ok(());
    }
    let config: toml::Value = toml::from_str(&fs::read_to_string(&configuration)?).map_err(err)?;
    let key = config
        .get("project_key")
        .and_then(toml::Value::as_str)
        .filter(|s| s.len() == 32 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| err("Invalid project identity"))?;
    if state.join("project-identity.json").exists()
        && read(&state.join("project-identity.json")).map_err(err)?
            != json!({"schema_version":1,"project_key":key})
    {
        return Err(err("Session state belongs to another project"));
    }
    if state.join("state-contract.json").exists()
        && read(&state.join("state-contract.json")).map_err(err)?["project_key"] != key
    {
        return Err(err("Session execution contract belongs to another project"));
    }
    let journal = local.join("session-renewal.json");
    let pending = if journal.exists() {
        let v = read(&journal).map_err(err)?;
        (v["phase"] != "complete").then_some(v)
    } else {
        None
    };
    let mut renewal = if let Some(pending) = pending {
        pending
    } else {
        let Some(reason) = reason(project, state, &config, manifest)? else {
            return Ok(());
        };
        if state.exists()
            && !state.join("project-identity.json").is_file()
            && fs::read_dir(state)?.next().is_some()
        {
            return Err(err(
                "Selected state directory has no project identity; cannot retire it",
            ));
        }
        let preferences = match read(&local.join("coordinator.json")) {
            Ok(v) if Preferences::parse(&v).is_ok() => v,
            _ => waywr_backend::model_runtime::initial_preferences().map_err(err)?,
        };
        let token = random_token().map_err(err)?;
        let v = json!({"schema_version":1,"project":project,"state":state,"project_key":key,
            "id":token,"reason":reason,"phase":"prepared","preferences":preferences,
            "notice":"Previous session abandoned; its execution outcome remains unknown. Start new work from project source; do not replay retired calls."});
        write(&journal, &v, false).map_err(err)?;
        v
    };
    let token = renewal["id"]
        .as_str()
        .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| err("Invalid session renewal identity"))?;
    if renewal["schema_version"] != 1
        || renewal["project"] != json!(project)
        || renewal["state"] != json!(state)
        || renewal["project_key"] != key
    {
        return Err(err("Session renewal belongs to another location"));
    }
    Preferences::parse(&renewal["preferences"]).map_err(err)?;
    let retired = local.join("retired-sessions").join(token);
    private_directory(&local.join("retired-sessions"))?;
    private_directory(&retired)?;
    if renewal["phase"] == "prepared" {
        stop_session(project, state, key)?;
        renewal["phase"] = json!("stopped");
        write(&journal, &renewal, false).map_err(err)?;
    }
    if renewal["phase"] == "stopped" {
        // On crash recovery re-verify that no owner restarted before the move.
        if !retired.join("state").exists() {
            stop_session(project, state, key)?;
        }
        for name in ["project.toml", "coordinator.json"] {
            let path = local.join(name);
            if path.exists() && !retired.join(name).exists() {
                fs::copy(path, retired.join(name))?;
            }
        }
        move_once(state, &retired.join("state"))?;
        if state == local.join("state") {
            move_once(&local.join("history"), &retired.join("history"))?;
        }
        for name in ["release.json", "workflow.json", "herdr.json"] {
            move_once(&local.join(name), &retired.join(name))?;
        }
        renewal["phase"] = json!("retired");
        write(&journal, &renewal, false).map_err(err)?;
    }
    if renewal["phase"] != "retired" {
        return Err(err("Unknown session renewal phase"));
    }
    // Keep identity/location stable. Only declarative shared-runtime preferences
    // survive; execution bindings, grants, queues, policies and budgets do not.
    write_bytes(&configuration,format!("schema_version = 2\nproject_key = \"{key}\"\n[execution]\nhost = \"systemd-user-v1\"\n[conversation]\nschema_version = 6\n").as_bytes(),false).map_err(err)?;
    let preferences_path = local.join("coordinator.json");
    if read(&preferences_path).ok().as_ref() != Some(&renewal["preferences"]) {
        write(&preferences_path, &renewal["preferences"], false).map_err(err)?;
    }
    fs::create_dir_all(state)?;
    waywr_backend::project_workflow::initialize(project).map_err(err)?;
    write(&state.join("session-origin.json"),&json!({"schema_version":1,"renewal_id":renewal["id"],
        "reason":renewal["reason"],"previous_session":"abandoned","context":"current project source and new user instructions"}),true).map_err(err)?;
    renewal["phase"] = json!("complete");
    write(&retired.join("retirement.json"), &renewal, false).map_err(err)?;
    write(&journal, &renewal, false).map_err(err)?;
    eprintln!(
        "waywr: Started a fresh session ({}) using current project source and model settings.",
        renewal["reason"].as_str().unwrap_or("session renewal")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_retirement_resumes_without_restoring_calls_or_old_targets() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().canonicalize().unwrap();
        let local = project.join(".waywr");
        let state = local.join("state");
        let key = "a".repeat(32);
        let id = "b".repeat(32);
        let retired = local.join("retired-sessions").join(&id);
        fs::create_dir_all(retired.join("state")).unwrap();
        fs::write(
            retired.join("state/uncertain.json"),
            "original immutable evidence",
        )
        .unwrap();
        fs::write(
            local.join("project.toml"),
            format!("schema_version=2\nproject_key=\"{key}\"\n"),
        )
        .unwrap();
        fs::write(
            local.join("workflow.json"),
            r#"{"target":"obsolete","revision":7}"#,
        )
        .unwrap();
        let preferences = waywr_backend::model_runtime::initial_preferences().unwrap();
        write(&local.join("session-renewal.json"),&json!({"schema_version":1,"project":project,"state":state,
            "project_key":key,"id":id,"phase":"stopped","reason":"obsolete-session-format","preferences":preferences}),false).unwrap();
        let manifest = json!({"state_schema_min":3,"state_schema_max":9});
        prepare(&project, &state, &manifest).unwrap();
        assert_eq!(
            fs::read_to_string(retired.join("state/uncertain.json")).unwrap(),
            "original immutable evidence"
        );
        assert!(!state.join("uncertain.json").exists());
        assert_ne!(
            read(&local.join("workflow.json")).unwrap()["target"],
            "obsolete"
        );
        assert_eq!(read(&local.join("coordinator.json")).unwrap(), preferences);
        assert_eq!(
            read(&local.join("session-renewal.json")).unwrap()["phase"],
            "complete"
        );
        let origin = fs::read(state.join("session-origin.json")).unwrap();
        prepare(&project, &state, &manifest).unwrap();
        assert_eq!(fs::read(state.join("session-origin.json")).unwrap(), origin);
        assert_eq!(
            fs::read_dir(local.join("retired-sessions"))
                .unwrap()
                .count(),
            1
        );
    }
    #[test]
    fn other_project_state_is_never_retired_even_with_obsolete_settings() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path();
        let local = project.join(".waywr");
        let state = local.join("state");
        fs::create_dir_all(&state).unwrap();
        let key = "a".repeat(32);
        fs::write(
            local.join("project.toml"),
            format!("schema_version=2\nproject_key=\"{key}\"\n"),
        )
        .unwrap();
        write(
            &state.join("project-identity.json"),
            &json!({"schema_version":1,"project_key":"c".repeat(32)}),
            true,
        )
        .unwrap();
        assert!(
            prepare(project, &state, &json!({}))
                .unwrap_err()
                .to_string()
                .contains("another project")
        );
        assert!(!local.join("session-renewal.json").exists());
        assert!(!local.join("retired-sessions").exists());
    }
}
