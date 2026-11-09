//! Conservative retirement of positively owned, settled temporary resources.
use crate::execution::{hash_file, write};
use crate::{
    development::git,
    execution::{read, FileLock},
    session_store::Session,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};
fn t<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("Missing resource {k}"))
}
pub fn retire(s: &Session, w: &Value) -> Result<(), String> {
    if w["policy"]["project_workflow"]["retention"] != "retire-clean-owned"
        || s.phase()?["phase"] != "running"
    {
        return Ok(());
    }
    let all = s
        .development_records("unit")?
        .into_iter()
        .filter(|u| u["work_id"] == w["work_id"])
        .collect::<Vec<_>>();
    for u in &all {
        if u["resource_owned"] != true
            || u["resource_owner"] == "human"
            || !matches!(
                u["status"].as_str(),
                Some("integrated" | "accepted" | "superseded" | "cancelled" | "failed")
            )
        {
            continue;
        }
        if u["recovery"].is_object() && u["recovery"]["phase"] != "resolved" {
            continue;
        }
        if all.iter().any(|other| {
            !matches!(
                other["status"].as_str(),
                Some(
                    "integrated" | "accepted" | "reviewed" | "superseded" | "cancelled" | "failed"
                )
            ) && ["start_after", "accept_after", "review_target_names"]
                .iter()
                .any(|k| other[k].as_array().is_some_and(|a| a.contains(&u["name"])))
        }) {
            continue;
        }
        if w["obligations"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|o| o["owner"] == u["unit_id"] && o["status"] != "resolved")
        {
            continue;
        }
        let id = t(u, "unit_id")?;
        let old = s.development_record("cleanup", id)?;
        if old.as_ref().is_some_and(|r| r["status"] == "retired") {
            continue;
        }
        let path = Path::new(t(u, "worktree")?);
        let branch = t(u, "branch")?;
        if u["resource_intent"]["path"] != u["worktree"]
            || u["resource_intent"]["branch"] != u["branch"]
            || !path.starts_with(s.project.join(".waywr/worktrees").join(t(w, "work_id")?))
        {
            continue;
        }
        let operation = crate::development::op_dir(
            s,
            t(w, "work_id")?,
            u["operation_id"].as_str().unwrap_or(""),
        );
        let settled = crate::host_attempt::Attempt::load(&s.workflow.state, &operation)
            .ok()
            .and_then(|a| read(&a.directory.join("completion.json")).ok())
            .is_some_and(|c| c["known_completion"] == true && c["settled"] == true);
        if !settled && u["retirement"]["proof"].is_null() {
            continue;
        }
        let _fence = FileLock::exclusive(&s.workflow.state.join("supervision/fence.lock"))?;
        let _gate = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
        let mut record=old.unwrap_or(json!({"work_id":w["work_id"],"request_id":w["request_id"],"unit_id":id,"path":path,"branch":branch,"status":"checking"}));
        if record["status"] != "intent" {
            if !path.exists() {
                continue;
            }
            retire_generated(s, path, &w["policy"], &u["generated_resources"])?;
            let dirty = git(
                path,
                &[
                    "status",
                    "--porcelain=v1",
                    "--ignored=matching",
                    "--untracked-files=all",
                ],
            )?;
            if !dirty.is_empty() {
                record["status"] = json!("retained");
                record["reason"] = json!("Dirty, untracked or ignored content preserved");
                s.development_save("cleanup", id, &record, "development.resource_retained")?;
                continue;
            }
            if git(path, &["branch", "--show-current"])? != branch {
                continue;
            }
            let head = git(path, &["rev-parse", "HEAD"])?;
            if crate::project_workflow::ref_head(&s.project, branch)? != head {
                continue;
            }
            let archive = format!("refs/waywr/archive/{}/{}", t(w, "work_id")?, id);
            if let Ok(existing) = git(&s.project, &["rev-parse", "--verify", &archive]) {
                if existing != head {
                    return Err("Resource archive identity collision; retain resources".into());
                }
            } else {
                git(
                    &s.project,
                    &[
                        "update-ref",
                        &archive,
                        &head,
                        "0000000000000000000000000000000000000000",
                    ],
                )?;
            }
            record["head"] = json!(head);
            record["archive_ref"] = json!(archive);
            record["status"] = json!("intent");
            s.development_save("cleanup", id, &record, "development.cleanup_intent")?;
        }
        let head = t(&record, "head")?;
        if path.exists() {
            if git(path, &["rev-parse", "HEAD"])? != head
                || !git(
                    path,
                    &[
                        "status",
                        "--porcelain=v1",
                        "--ignored=matching",
                        "--untracked-files=all",
                    ],
                )?
                .is_empty()
            {
                continue;
            }
            git(&s.project, &["worktree", "remove", t(u, "worktree")?])?;
        }
        if let Ok(actual) = crate::project_workflow::ref_head(&s.project, branch) {
            if actual != head
                || crate::project_workflow::target_checkout(&s.project, branch)?.is_some()
            {
                continue;
            }
            git(
                &s.project,
                &["update-ref", "-d", &format!("refs/heads/{branch}"), head],
            )?;
        }
        record["status"] = json!("retired");
        s.development_save("cleanup", id, &record, "development.resource_retired")?;
    }
    if w["status"] == "delivered" && !crate::candidate::writes_held(w) && w["pending"].is_null() {
        retire_governing(s, w)?;
    }
    Ok(())
}
fn retire_governing(s: &Session, w: &Value) -> Result<(), String> {
    let id = format!("{}-governing", t(w, "work_id")?);
    let previous = s.development_record("cleanup", &id)?;
    if previous.as_ref().is_some_and(|r| r["status"] == "retired") {
        return Ok(());
    }
    let branch = t(w, "integration_branch")?;
    let path = Path::new(t(w, "integration_worktree")?);
    let head = t(w, "head")?;
    let _fence = FileLock::exclusive(&s.workflow.state.join("supervision/fence.lock"))?;
    let _git = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
    let mut record=previous.unwrap_or(json!({"work_id":w["work_id"],"request_id":w["request_id"],"path":path,"branch":branch,"head":head,"status":"checking"}));
    if let Some(owner) = crate::project_workflow::target_checkout(&s.project, branch)? {
        if owner != path {
            record["status"] = json!("retained");
            record["reason"] = json!(format!(
                "Governing branch remains checked out at {}; no forced checkout",
                owner.display()
            ));
            s.development_save("cleanup", &id, &record, "development.resource_retained")?;
            return Ok(());
        }
    }
    if path.exists() {
        retire_generated(
            s,
            path,
            &w["policy"],
            if w["generated_resources"].is_object() {
                &w["generated_resources"]
            } else {
                &w["checks"]["generated_resources"]
            },
        )?;
    }
    if path.exists()
        && (git(path, &["rev-parse", "HEAD"])? != head
            || !git(
                path,
                &[
                    "status",
                    "--porcelain=v1",
                    "--ignored=matching",
                    "--untracked-files=all",
                ],
            )?
            .is_empty())
    {
        record["status"] = json!("retained");
        record["reason"] =
            json!("Governing checkout changed or contains local/unclassified generated content");
        s.development_save("cleanup", &id, &record, "development.resource_retained")?;
        return Ok(());
    }
    let archive = format!("refs/waywr/archive/{}/governing", t(w, "work_id")?);
    if let Ok(saved) = git(&s.project, &["rev-parse", "--verify", &archive]) {
        if saved != head {
            return Err("Governing archive collision; resources retained".into());
        }
    } else {
        git(
            &s.project,
            &[
                "update-ref",
                &archive,
                head,
                "0000000000000000000000000000000000000000",
            ],
        )?;
    }
    record["archive_ref"] = json!(archive);
    record["status"] = json!("intent");
    s.development_save("cleanup", &id, &record, "development.cleanup_intent")?;
    if path.exists() {
        git(
            &s.project,
            &["worktree", "remove", t(w, "integration_worktree")?],
        )?;
    }
    if let Ok(current) = crate::project_workflow::ref_head(&s.project, branch) {
        if current != head {
            return Err("Governing branch changed after delivery; no deletion".into());
        }
        git(
            &s.project,
            &["update-ref", "-d", &format!("refs/heads/{branch}"), head],
        )?;
    }
    record["status"] = json!("retired");
    s.development_save("cleanup", &id, &record, "development.resource_retired")?;
    Ok(())
}

// Output roots are exact approved project-relative directories, never globs or
// ignore-based authority. A before/after inventory records only newly produced,
// ignored ordinary files in an exclusively service-owned execution checkout.
pub fn output_roots(policy: &Value) -> Vec<String> {
    policy["project_workflow"]["generated_outputs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}
pub fn valid_output_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\0', '*', '?', '[', ']'])
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !path.split('/').any(|c| {
            matches!(
                c,
                ".git" | ".waywr" | ".env" | ".ssh" | "node_modules" | "vendor" | ".venv" | "venv"
            )
        })
}
fn inventory(
    tree: &Path,
    roots: &[String],
    ignored_only: bool,
) -> Result<BTreeMap<String, String>, String> {
    let root = tree.canonicalize().map_err(|e| e.to_string())?;
    let args = if ignored_only {
        vec![
            "ls-files",
            "-z",
            "--others",
            "--ignored",
            "--exclude-standard",
        ]
    } else {
        vec!["ls-files", "-z", "--others"]
    };
    let mut files = BTreeMap::new();
    for path in git(tree, &args)?.split('\0').filter(|p| !p.is_empty()) {
        if !roots
            .iter()
            .any(|r| valid_output_path(r) && Path::new(path).starts_with(r))
        {
            continue;
        }
        let file = root.join(path);
        // Reject symlinked parents, special files, nested Git repositories and
        // tracked paths; none becomes disposable through an ignore rule.
        let mut parent = file.as_path();
        let mut safe = true;
        while parent != root {
            let m = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
            if m.file_type().is_symlink() || parent.join(".git").exists() {
                safe = false;
                break;
            }
            parent = parent.parent().ok_or("Output escapes tree")?;
        }
        if !safe
            || !fs::symlink_metadata(&file)
                .map_err(|e| e.to_string())?
                .is_file()
        {
            continue;
        }
        if file.canonicalize().map_err(|e| e.to_string())? != root.join(path) {
            continue;
        }
        files.insert(path.to_owned(), hash_file(&file)?);
    }
    Ok(files)
}
pub fn begin_generated(
    tree: &Path,
    policy: &Value,
    directory: &Path,
    previous: &Value,
) -> Result<(), String> {
    let roots = output_roots(policy);
    if roots.is_empty() {
        return Ok(());
    }
    for r in &roots {
        if !valid_output_path(r) {
            return Err("Unsafe generated output declaration".into());
        }
    }
    let files = inventory(tree, &roots, false)?;
    let mut owned_before = Vec::new();
    if let Some(path) = previous["path"].as_str().map(Path::new) {
        if hash_file(path)? != previous["sha256"] {
            return Err("Prior generated manifest hash changed; preserve outputs".into());
        }
        let prior = read(path)?;
        if prior["tree"] == json!(tree)
            && prior["roots"] == json!(roots)
            && prior["head"]
                .as_str()
                .is_some_and(|h| git(tree, &["merge-base", "--is-ancestor", h, "HEAD"]).is_ok())
        {
            for (path, digest) in prior["files"]
                .as_object()
                .ok_or("Prior generated files missing")?
            {
                if files.get(path).map(String::as_str) == digest.as_str() {
                    owned_before.push(path.clone());
                }
            }
        }
    }
    write(
        &directory.join("generated-before.json"),
        &json!({"version":1,"tree":tree,"roots":roots,"files":files,"owned_before":owned_before,"previous":previous}),
        true,
    )
}
pub fn finish_generated(tree: &Path, policy: &Value, directory: &Path) -> Result<Value, String> {
    let path = directory.join("generated-before.json");
    if !path.exists() {
        return Ok(Value::Null);
    }
    let before = read(&path)?;
    let roots = output_roots(policy);
    if before["tree"] != json!(tree) || before["roots"] != json!(roots) {
        return Err("Generated output authority changed; retained".into());
    }
    let mut files = inventory(tree, &roots, true)?;
    files.retain(|p, _| {
        before["files"].get(p).is_none()
            || before["owned_before"]
                .as_array()
                .is_some_and(|a| a.contains(&json!(p)))
    });
    let after = directory.join("generated-after.json");
    write(
        &after,
        &json!({"version":1,"tree":tree,"roots":roots,"head":git(tree,&["rev-parse","HEAD"])?,"before_sha256":hash_file(&path)?,"files":files}),
        true,
    )?;
    Ok(json!({"path":after,"sha256":hash_file(&after)?}))
}
fn retire_generated(
    s: &Session,
    tree: &Path,
    policy: &Value,
    reference: &Value,
) -> Result<(), String> {
    let Some(path) = reference["path"].as_str().map(Path::new) else {
        return Ok(());
    };
    if !path.starts_with(&s.workflow.state) || hash_file(path)? != reference["sha256"] {
        return Err("Generated manifest identity conflict; preserve worktree".into());
    }
    let manifest = read(path)?;
    let roots = output_roots(policy);
    if manifest["tree"] != json!(tree)
        || manifest["roots"] != json!(roots)
        || manifest["head"] != git(tree, &["rev-parse", "HEAD"])?
    {
        return Ok(());
    }
    let current = inventory(tree, &roots, true)?;
    for (relative, digest) in manifest["files"]
        .as_object()
        .ok_or("Generated manifest files missing")?
    {
        if current.get(relative).map(String::as_str) != digest.as_str() {
            continue;
        }
        // Recheck immediately before unlink; no recursive deletion or force clean.
        let file = tree.join(relative);
        if hash_file(&file)? != *digest {
            continue;
        }
        fs::remove_file(&file).map_err(|e| e.to_string())?;
        let mut parent = file.parent();
        while let Some(p) = parent {
            if p == tree {
                break;
            }
            if fs::remove_dir(p).is_err() {
                break;
            }
            parent = p.parent();
        }
    }
    Ok(())
}
