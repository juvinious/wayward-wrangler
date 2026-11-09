//! Canonical project layout relocation. Call only under stopped-service/startup fences.
//! Immutable packets, receipts, approvals and log frames are never rewritten.
use crate::execution::{read, write};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};
pub const LEGACY: &str = ".wayward-wrangler";
pub const CANONICAL: &str = ".waywr";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn history_root(project: &Path, state: &Path) -> PathBuf {
    if state == project.join(".waywr/state") {
        project.join(".waywr/history")
    } else {
        state.join("history")
    }
}
pub fn valid_alias(path: &Path, target: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
        && path.canonicalize().ok().as_deref() == Some(target)
}
pub fn check(project: &Path) -> Result<(), String> {
    let old = project.join(LEGACY);
    let new = project.join(CANONICAL);
    if project.join(".waywr-migration.json").exists() {
        let journal = read(&project.join(".waywr-migration.json"))?;
        if journal["phase"] != "complete" {
            return Err(
                "Project layout migration is incomplete; run waywr session migrate to reconcile it"
                    .into(),
            );
        }
    }
    if fs::symlink_metadata(&old).is_ok() && !valid_alias(&old, &new) {
        return Err(if fs::symlink_metadata(&new).is_ok(){"Both .waywr and .wayward-wrangler exist; preserved without combining. Reconcile their ownership before migration"}else{"Legacy project layout detected; run waywr session migrate before startup"}.into());
    }
    if fs::symlink_metadata(&new).is_ok_and(|m| !m.is_dir() || m.file_type().is_symlink()) {
        return Err("Canonical .waywr must be a real project directory".into());
    }
    if new.is_dir()&&!new.join("project.toml").is_file()&&fs::read_dir(&new).map_err(err)?.next().is_some(){
        return Err("Existing .waywr has unrelated or incomplete contents; preserved without initializing into it".into())
    }
    Ok(())
}
pub fn relocate(project: &Path, key: &str) -> Result<Value, String> {
    let old = project.join(LEGACY);
    let new = project.join(CANONICAL);
    let journal = project.join(".waywr-migration.json");
    if !journal.exists() {
        if fs::symlink_metadata(&new).is_ok() {
            return Err(
                "Existing .waywr preserved; migration will not overwrite or combine it".into(),
            );
        }
        if !fs::symlink_metadata(&old).map_err(err)?.is_dir() {
            return Err("Legacy root must be a real directory".into());
        }
        let worktrees = Command::new("git")
            .args(["worktree", "list", "--porcelain", "-z"])
            .current_dir(project)
            .output()
            .map_err(err)?;
        let trees = if worktrees.status.success() {
            String::from_utf8_lossy(&worktrees.stdout)
                .split('\0')
                .filter_map(|l| l.strip_prefix("worktree "))
                .filter_map(|p| Path::new(p).strip_prefix(&old).ok())
                .map(|p| new.join(p))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        write(
            &journal,
            &json!({"schema_version":1,"project":project,"project_key":key,"phase":"prepared","worktrees":trees,"preservation":"Immutable evidence retains original bytes and legacy paths via aliases"}),
            true,
        )?;
    }
    let mut j = read(&journal)?;
    if j["schema_version"] != 1
        || j["project"] != project.to_string_lossy().as_ref()
        || j["project_key"] != key
    {
        return Err("Migration journal identity mismatch".into());
    }
    if !new.exists() {
        fs::rename(&old, &new).map_err(err)?;
    }
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(new.join("project.toml")).map_err(err)?).map_err(err)?;
    if config["project_key"].as_str() != Some(key) {
        return Err("Moved project identity differs; preserve and inspect".into());
    }
    if fs::symlink_metadata(&old).is_err() {
        symlink(CANONICAL, &old).map_err(err)?;
    }
    if !valid_alias(&old, &new) {
        return Err("Legacy compatibility alias differs; preserve and inspect".into());
    }
    j["phase"] = json!("moved");
    write(&journal, &j, false)?;
    let state = new.join("state");
    let previous = state.join("history");
    let history = new.join("history");
    if previous.is_dir() && !valid_alias(&previous, &history) {
        if fs::symlink_metadata(&history).is_ok() {
            return Err("Two history directories exist; preserved without combining".into());
        }
        fs::rename(&previous, &history).map_err(err)?;
    }
    if history.exists() && fs::symlink_metadata(&previous).is_err() {
        symlink("../history", &previous).map_err(err)?;
    }
    if fs::symlink_metadata(&previous).is_ok() && !valid_alias(&previous, &history) {
        return Err("Legacy history alias differs; preserve and inspect".into());
    }
    // Keep operator migration metadata out of application commits without editing its tracked files.
    let exclude = Command::new("git")
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/exclude",
        ])
        .current_dir(project)
        .output()
        .map_err(err)?;
    if exclude.status.success() {
        let path = PathBuf::from(String::from_utf8_lossy(&exclude.stdout).trim());
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
            return Err("Git local exclude changed; preserve and inspect".into());
        }
        let mut text = if path.exists() {
            fs::read_to_string(&path).map_err(err)?
        } else {
            String::new()
        };
        for line in ["/.waywr/", "/.wayward-wrangler", "/.waywr-migration*"] {
            if !text.lines().any(|l| l == line) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n')
                }
                text.push_str(line);
                text.push('\n');
            }
        }
        crate::execution::write_bytes(&path, text.as_bytes(), false)?;
    }
    if let Some(trees) = j["worktrees"].as_array().filter(|a| !a.is_empty()) {
        let mut command = Command::new("git");
        command.args(["worktree", "repair"]).current_dir(project);
        for tree in trees {
            let path=Path::new(tree.as_str().ok_or("Invalid worktree path")?);
            if !path.canonicalize().map_err(err)?.starts_with(new.join("worktrees")){return Err("Recorded migration worktree leaves canonical worktree root".into())}
            command.arg(path);
        }
        let result = command.output().map_err(err)?;
        if !result.status.success() {
            return Err(format!(
                "Git worktree repair failed; preserved for retry: {}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
    }
    if state.join("state.sqlite").is_file() {
        let mut c = Connection::open(state.join("state.sqlite")).map_err(err)?;
        let tx = c.transaction().map_err(err)?;
        let has = |table: &str| {
            tx.query_row(
                "SELECT exists(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |r| r.get::<_, bool>(0),
            )
            .map_err(err)
        };
        if has("development_records")? {
            let rows = {
                let mut q = tx
                    .prepare("SELECT kind,id,record_json FROM development_records")
                    .map_err(err)?;
                q.query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(err)?
            };
            for (kind, id, raw) in rows {
                let mut v: Value = serde_json::from_str(&raw).map_err(err)?;
                for field in ["worktree", "integration_worktree"] {
                    if let Some(relative) = v[field]
                        .as_str()
                        .and_then(|p| Path::new(p).strip_prefix(&old).ok())
                    {
                        v[field] = json!(new.join(relative));
                    }
                }
                tx.execute(
                    "UPDATE development_records SET record_json=?1 WHERE kind=?2 AND id=?3",
                    params![v.to_string(), kind, id],
                )
                .map_err(err)?;
            }
        }
        if has("request_history_requests")? {
            let rows = {
                let mut q = tx
                    .prepare("SELECT request_id,record_json FROM request_history_requests")
                    .map_err(err)?;
                q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                    .map_err(err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(err)?
            };
            for (id, raw) in rows {
                let mut v: Value = serde_json::from_str(&raw).map_err(err)?;
                for field in ["log_path", "summary_path"] {
                    if let Some(path) = v[field].as_str() {
                        let path = Path::new(path);
                        if let Ok(relative) = path
                            .strip_prefix(old.join("state/history"))
                            .or_else(|_| path.strip_prefix(&previous))
                        {
                            v[field] = json!(history.join(relative));
                        }
                    }
                }
                tx.execute(
                    "UPDATE request_history_requests SET record_json=?1 WHERE request_id=?2",
                    params![v.to_string(), id],
                )
                .map_err(err)?;
            }
        }
        tx.commit().map_err(err)?;
    }
    j["phase"] = json!("references-repaired");
    write(&journal, &j, false)?;
    Ok(j)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().unwrap();
        let key = "a".repeat(32);
        let old = d.path().join(LEGACY);
        fs::create_dir_all(old.join("state/history")).unwrap();
        fs::write(
            old.join("project.toml"),
            format!("schema_version = 2\nproject_key = \"{key}\"\n"),
        )
        .unwrap();
        (d, key)
    }
    #[test]
    fn migration_conflict_preserves_both_directories() {
        let (d, key) = setup();
        fs::create_dir(d.path().join(CANONICAL)).unwrap();
        fs::write(d.path().join(".waywr/unrelated"), "keep").unwrap();
        assert!(relocate(d.path(), &key).is_err());
        assert_eq!(
            fs::read_to_string(d.path().join(".waywr/unrelated")).unwrap(),
            "keep"
        );
        assert!(d.path().join(".wayward-wrangler/project.toml").exists());
        assert!(!d.path().join(".waywr-migration.json").exists());
    }
    #[test]
    fn migration_repairs_linked_worktrees_and_preserves_dirty_input_and_immutable_evidence() {
        let (d, key) = setup();
        let p = d.path();
        let old = p.join(LEGACY);
        let state = old.join("state");
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(p)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@local",
            "commit",
            "--allow-empty",
            "-qm",
            "baseline",
        ]);
        let tree = old.join("worktrees/worker");
        run(&["worktree", "add", "-qb", "worker", tree.to_str().unwrap()]);
        fs::write(tree.join("partial.txt"), "uncommitted worker input").unwrap();
        fs::write(
            state.join("history/request.log"),
            "CONVERSATION actual text\nTASKS assigned\nOUTCOMES observed\nSUMMARY recorded\n",
        )
        .unwrap();
        fs::write(state.join("receipt.json"), "immutable exact bytes").unwrap();
        let c = Connection::open(state.join("state.sqlite")).unwrap();
        c.execute_batch("CREATE TABLE approvals(body TEXT);INSERT INTO approvals VALUES('exact saved approval');CREATE TABLE request_history_requests(request_id TEXT,record_json TEXT);CREATE TABLE development_records(kind TEXT,id TEXT,record_json TEXT);").unwrap();
        c.execute("INSERT INTO request_history_requests VALUES('req',?1)",[json!({"log_path":state.join("history/request.log"),"summary_path":state.join("history/request.summary.md")}).to_string()]).unwrap();
        c.execute(
            "INSERT INTO development_records VALUES('unit','worker',?1)",
            [json!({"worktree":tree,"authorization":{"hash":"unchanged"}}).to_string()],
        )
        .unwrap();
        drop(c);
        let result = relocate(p, &key).unwrap();
        assert_eq!(result["phase"], "references-repaired");
        assert!(valid_alias(&old, &p.join(CANONICAL)));
        assert_eq!(
            fs::read_to_string(state.join("receipt.json")).unwrap(),
            "immutable exact bytes"
        );
        assert_eq!(
            fs::read_to_string(p.join(".waywr/history/request.log")).unwrap(),
            fs::read_to_string(state.join("history/request.log")).unwrap()
        );
        let newtree = p.join(".waywr/worktrees/worker");
        assert_eq!(
            fs::read_to_string(newtree.join("partial.txt")).unwrap(),
            "uncommitted worker input"
        );
        let out = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&newtree)
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("partial.txt"));
        let c = Connection::open(p.join(".waywr/state/state.sqlite")).unwrap();
        assert_eq!(
            c.query_row("SELECT body FROM approvals", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "exact saved approval"
        );
        let raw: String = c
            .query_row(
                "SELECT record_json FROM request_history_requests",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&raw).unwrap()["log_path"],
            json!(p.join(".waywr/history/request.log"))
        );
        drop(c);
        assert!(relocate(p, &key).is_ok());
    }
    #[test]
    fn migration_reconciles_interruption_after_root_move() {
        let (d, key) = setup();
        let p = d.path();
        write(&p.join(".waywr-migration.json"),&json!({"schema_version":1,"project":p,"project_key":key,"phase":"prepared","worktrees":[]}),true).unwrap();
        fs::rename(p.join(LEGACY), p.join(CANONICAL)).unwrap();
        assert!(check(p).is_err());
        assert!(relocate(p, &key).is_ok());
        assert!(valid_alias(&p.join(LEGACY), &p.join(CANONICAL)));
    }
}
