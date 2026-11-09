//! Explicit project-local layout migration, with supervised drain and compatible binding.
use super::{request, rust_bundle, rust_maintenance, rust_runtime, selected_project, session};
use serde_json::{Value, json};
use std::{fs, io, path::Path, process::Command, thread, time::Duration};
use waywr_backend::{
    execution::{FileLock, alive, random_token, read, write},
    project_layout,
    session_store::Session,
};
fn err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
pub fn migrate(root: &Path, manifest: &Value, args: &[String]) -> io::Result<()> {
    let project = selected_project(args)?;
    if args.iter().any(|a| a == "--state-dir") {
        return Err(err(
            "Layout migration requires the project's recorded default state; custom state is preserved and not relocated",
        ));
    }
    let _migration = FileLock::exclusive(&project.join(".waywr-migration.lock")).map_err(err)?;
    let old = project.join(project_layout::LEGACY);
    let new = project.join(project_layout::CANONICAL);
    let journal = project.join(".waywr-migration.json");
    if new.exists() && !journal.exists() {
        return Err(err(
            "Existing .waywr preserved; refusing to combine or overwrite state",
        ));
    }
    let located = if new.exists() {
        new.clone()
    } else {
        old.clone()
    };
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(located.join("project.toml"))?).map_err(err)?;
    let key = config["project_key"]
        .as_str()
        .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| err("Invalid migration project identity"))?;
    let target = args
        .windows(2)
        .find(|a| a[0] == "--release")
        .map(|a| a[1].as_str())
        .unwrap_or(
            manifest["release_id"]
                .as_str()
                .ok_or_else(|| err("Release ID missing"))?,
        );
    if target.is_empty() || target.contains('/') || target.contains("..") {
        return Err(err("Invalid migration release"));
    }
    let destination = root
        .parent()
        .ok_or_else(|| err("Release catalog missing"))?
        .join(target);
    let release = rust_bundle::validate(&destination)?;
    if release["project_layout"] != "waywr-v1" {
        return Err(err(
            "Migration target does not support canonical .waywr layout",
        ));
    }
    let state = located.join("state");
    let binding = read(&located.join("release.json")).map_err(err)?;
    for entry in fs::read_dir(state.join("session/ui-claims"))
        .into_iter()
        .flatten()
    {
        let path = entry?.path();
        if path.extension().is_some_and(|x| x == "json")
            && alive(&read(&path).map_err(err)?["process"])
        {
            return Err(err(
                "A verified live TUI is attached; detach it before layout migration",
            ));
        }
    }
    if state.join("session/outbox").is_dir()
        && fs::read_dir(state.join("session/outbox"))?
            .any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|s| s == "json")))
    {
        return Err(err(
            "Pending client delivery; migration preserved without moving state",
        ));
    }
    let unit = format!("waywr-{key}.service");
    let output = Command::new("systemctl")
        .args(["--user", "show", "--property=ActiveState", "--value", &unit])
        .output()?;
    if !output.status.success() {
        return Err(err("Cannot verify project service before migration"));
    }
    let active = String::from_utf8_lossy(&output.stdout).trim() == "active";
    if active {
        let socket = rust_runtime::service_socket(&project, args, key)?;
        let mut peer = session(
            socket.to_str().ok_or_else(|| err("Non-UTF8 IPC path"))?,
            key,
            binding["release_id"]
                .as_str()
                .ok_or_else(|| err("Binding release missing"))?,
            "command",
        )?;
        let snapshot = request(&mut peer, "snapshot", "migration-snapshot", json!({}))?;
        if snapshot.body["session"]["phase"] == "running" {
            let id = format!("layout-drain-{}", random_token().map_err(err)?);
            let reply = request(
                &mut peer,
                "command",
                &id,
                json!({"command_id":id,"action":"stop_session","expected_state_revision":snapshot.body["state_revision"],"payload":{}}),
            )?;
            if reply.body["status"] != "accepted" {
                return Err(err("Migration drain was not accepted; no state moved"));
            }
        }
        loop {
            let snapshot = request(&mut peer, "snapshot", "migration-drain", json!({}))?;
            if snapshot.body["session"]["phase"] == "paused" {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
        let status = Command::new("systemctl")
            .args(["--user", "stop", &unit])
            .status()?;
        if !status.success() {
            return Err(err("Project service did not stop; state preserved"));
        }
    }
    rust_maintenance::unit_inactive(key)?;
    let _startup = FileLock::exclusive(&state.join("session/start.lock")).map_err(err)?;
    let _fence = FileLock::exclusive(&state.join("supervision/fence.lock")).map_err(err)?;
    let saved = Session::open_existing(&project, &state, key, "layout-maintenance").map_err(err)?;
    if saved.snapshot().map_err(err)?["session"]["phase"] != "paused" {
        return Err(err(
            "Session is not durably paused; stop it through the bound service before migration",
        ));
    }
    saved.quiescent_for_release_change().map_err(err)?;
    rust_maintenance::host_attempts_quiescent(&state, key)?;
    drop(saved);
    let mut record = project_layout::relocate(&project, key).map_err(err)?;
    rust_runtime::relocate_registration(
        &project,
        &old.join("state"),
        &new.join("state"),
        key,
        args,
    )?;
    // Binding change takes its own startup fence after relocation.
    drop(_fence);
    drop(_startup);
    if binding["release_id"] != target {
        let mut upgrade = args.to_vec();
        if !upgrade.iter().any(|a| a == "--release") {
            upgrade.extend(["--release".into(), target.into()]);
        }
        rust_maintenance::change(root, manifest, &upgrade, "upgrade")?;
    }
    record["phase"] = json!("complete");
    record["release_id"] = json!(target);
    write(&journal, &record, false).map_err(err)?;
    println!(
        "{}",
        json!({"status":"migrated","project_key":key,"canonical":new,"history":new.join("history"),"release_id":target,"session":"paused; start normally to inspect or begin your next request"})
    );
    Ok(())
}
