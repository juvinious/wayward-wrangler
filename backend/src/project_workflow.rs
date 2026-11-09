//! Ordinary project setup. Saved preferences become authority only in an exact
//! scope/policy approval. Legacy projects keep their existing policy unchanged.
use crate::{
    codec::canonical_sha256,
    development::git,
    execution::{read, write, write_bytes},
    session_store::Session,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("Missing setup {k}"))
}
pub fn canonical_project(path: &Path) -> Result<PathBuf, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    for p in path.ancestors() {
        if p.join(".waywr/project.toml").is_file() {
            return Ok(p.to_path_buf());
        }
    }
    if let Ok(common) = git(
        &path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) {
        let common = PathBuf::from(common);
        if common.file_name().is_some_and(|n| n == ".git") {
            let root = common.parent().ok_or("Git common root missing")?;
            if root.join(".waywr/project.toml").is_file() {
                return Ok(root.to_path_buf());
            }
        }
    }
    Ok(git(&path, &["rev-parse", "--show-toplevel"])
        .ok()
        .map(PathBuf::from)
        .unwrap_or(path))
}
pub fn exclude_state(project: &Path) -> Result<(), String> {
    let Ok(path) = git(
        project,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/exclude",
        ],
    ) else {
        return Ok(());
    };
    let path = PathBuf::from(path);
    let mut value = fs::read_to_string(&path).unwrap_or_default();
    for line in [".waywr/", ".waywr-migration*"] {
        if !value.lines().any(|v| v == line) {
            if !value.is_empty() && !value.ends_with('\n') {
                value.push('\n')
            }
            value.push_str(line);
            value.push('\n');
        }
    }
    write_bytes(&path, value.as_bytes(), false)
}
pub fn detect(project: &Path) -> Value {
    let head = git(project, &["rev-parse", "--verify", "HEAD^{commit}"]).ok();
    let branch = git(project, &["branch", "--show-current"])
        .ok()
        .filter(|v| !v.is_empty());
    let upstream = git(project, &["rev-parse", "--abbrev-ref", "@{upstream}"]).ok();
    let default = git(
        project,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .ok()
    .and_then(|v| v.strip_prefix("origin/").map(str::to_owned));
    let target = default
        .filter(|b| {
            git(
                project,
                &["show-ref", "--verify", &format!("refs/heads/{b}")],
            )
            .is_ok()
        })
        .or_else(|| {
            upstream
                .as_ref()
                .and_then(|u| u.split_once('/').map(|(_, b)| b.to_owned()))
        })
        .or(branch.clone());
    json!({"head":head,"branch":branch,"recommended_target":target,"upstream":upstream,
        "git_common_directory":git(project,&["rev-parse","--path-format=absolute","--git-common-dir"]).ok(),
        "remotes":git(project,&["remote"]).ok(),"worktrees":git(project,&["worktree","list","--porcelain"]).ok(),
        "dirty":git(project,&["status","--porcelain=v1","--untracked-files=all"]).ok(),
        "input":"Committed baseline excludes uncommitted edits; the primary checkout is never stashed/reset."})
}
pub fn initialize(project: &Path) -> Result<(), String> {
    let path = project.join(".waywr/workflow.json");
    if path.exists() {
        return Ok(());
    }
    let detected = detect(project);
    let settings = json!({"version":1,"baseline":"HEAD","baseline_input":"committed","target":detected["recommended_target"],"completion":"checked-local","human_validation":false,"retention":"retire-clean-owned","remote":null,"repository":null,"merge_method":"squash","generated_outputs":[],"command_recipes":detected["head"].as_str().map(|h|crate::project_knowledge::recipes(project,h)).unwrap_or_default(),"revision":1});
    write(&path, &settings, true)?;
    exclude_state(project)
}
pub fn settings(s: &Session) -> Result<Option<Value>, String> {
    let path = s.project.join(".waywr/workflow.json");
    if !path.exists() {
        return Ok(None);
    }
    let v = read(&path)?;
    validate(&v)?;
    Ok(Some(v))
}
pub fn validate(v: &Value) -> Result<(), String> {
    if v["version"] != 1
        || !matches!(
            v["baseline_input"].as_str(),
            Some("committed" | "initialize-files")
        )
        || !matches!(
            v["completion"].as_str(),
            Some("checked-local" | "github-pr")
        )
        || !v["human_validation"].is_boolean()
        || !matches!(
            v["retention"].as_str(),
            Some("retire-clean-owned" | "retain")
        )
        || v["baseline"]
            .as_str()
            .is_none_or(|s| s.is_empty() || s.starts_with('-') || s.contains('\0'))
    {
        return Err("Invalid project setup; choose baseline input, target, completion and validation explicitly in F3 → Project".into());
    }
    if let Some(target) = v["target"].as_str() {
        if !valid_branch(target) {
            return Err("Invalid target branch".into());
        }
    }
    if v["completion"] == "github-pr" {
        if !matches!(
            v["merge_method"].as_str(),
            Some("merge" | "squash" | "rebase")
        ) || v["remote"]
            .as_str()
            .is_none_or(|v| v.is_empty() || v.starts_with('-') || v.contains(char::is_whitespace))
            || v["repository"].as_str().is_none_or(|v| {
                v.split('/').count() != 2
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-/ .".contains(&b))
                    || v.contains(' ')
            })
        {
            return Err("PR policy requires exact owner/repo, remote and merge method".into());
        }
    }
    if v["command_recipes"].as_array().is_none_or(|a| {
        a.len() > 128
            || a.iter().any(|r| {
                r["argv"].as_array().is_none_or(|a| {
                    a.is_empty()
                        || a.len() > 100
                        || a.iter().any(|v| {
                            v.as_str()
                                .is_none_or(|s| s.len() > 8192 || s.contains('\0'))
                        })
                }) || !matches!(
                    r["kind"].as_str(),
                    Some("check" | "build" | "inspect" | "preview")
                )
            })
    }) {
        return Err(
            "Project recipes need bounded argv arrays and check/build/inspect/preview kinds".into(),
        );
    }
    if let Some(paths)=v.get("generated_outputs") {
        if paths.as_array().is_none_or(|a|a.len()>32||a.iter().any(|v|v.as_str().is_none_or(|p|!crate::project_resources::valid_output_path(p)))){
            return Err("Generated outputs must be exact relative paths, without patterns, state or dependency directories".into());
        }
    }
    Ok(())
}
pub fn valid_branch(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with(['-', '/', '.'])
        && !s.ends_with(['/', '.'])
        && !s.ends_with(".lock")
        && !s.chars().any(char::is_whitespace)
        && !s.contains(['~', '^', ':', '?', '*', '[', '\\'])
        && !s.contains("..")
        && !s.contains("@{")
        && !s.contains("//")
}
fn secret(path: &str) -> bool {
    path.split('/').any(|c| {
        matches!(
            c,
            ".git"
                | ".waywr"
                | ".env"
                | ".aws"
                | ".ssh"
                | ".codex"
                | ".claude"
                | "node_modules"
                | "target"
                | ".venv"
        ) || c.ends_with(".pem")
            || c.ends_with(".key")
            || c.starts_with(".env.")
    })
}
fn input_files(root: &Path) -> Result<BTreeMap<String, String>, String> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<(), String> {
        for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            let p = e.path();
            let rel = p
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("Non UTF8 input path")?
                .to_owned();
            if secret(&rel) {
                continue;
            }
            let ft = e.file_type().map_err(|e| e.to_string())?;
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                visit(root, &p, out)?
            } else if ft.is_file() {
                out.insert(rel, crate::execution::hash_file(&p)?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out)?;
    Ok(out)
}
pub fn policy(s: &Session) -> Result<Option<Value>, String> {
    let Some(mut v) = settings(s)? else {
        return Ok(None);
    };
    let head = git(
        &s.project,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", text(&v, "baseline")?),
        ],
    )
    .ok();
    let target = v["target"].as_str().map(str::to_owned);
    let target_head = target.as_ref().and_then(|t| {
        git(
            &s.project,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{t}^{{commit}}"),
            ],
        )
        .ok()
    });
    let mut blockers = vec![];
    if target.is_none() {
        blockers.push("Select a named target branch in F3 → Project");
    }
    if v["baseline_input"] == "committed" && head.is_none() {
        blockers.push("No committed baseline: select an existing commit or explicitly initialize listed project files in F3 → Project");
    }
    if v["baseline_input"] == "initialize-files"
        && git(&s.project, &["rev-parse", "--verify", "HEAD"]).is_ok()
    {
        blockers.push("Initial-file import is only for a non-Git/unborn project; select a committed baseline for existing Git work");
    }
    if v["baseline_input"] == "initialize-files" {
        let files = input_files(&s.project)?;
        let manifest = json!({"version":1,"project":s.project,"files":files,"excluded":"Waywr state, credential locations, symlinks and dependency/build directories"});
        let digest = hash(&manifest)?;
        let path = s
            .project
            .join(".waywr/setup-inputs")
            .join(format!("{digest}.json"));
        write(&path, &manifest, true)?;
        v["input_manifest"] = json!({"path":path,"sha256":digest,"files":files.len(),"approval":"Initial scope approval authorizes this exact source baseline; changes invalidate it"});
    }
    v["baseline_commit"] = json!(head);
    v["target_commit"] = json!(target_head);
    v["blockers"] = json!(blockers);
    v["ready"] = json!(blockers.is_empty());
    Ok(Some(v))
}
pub fn snapshot(s: &Session) -> Result<Value, String> {
    Ok(
        json!({"version":1,"settings":settings(s)?,"effective":policy(s)?,"observed":detect(&s.project),"applies":"Future request approval; existing approved policies remain frozen"}),
    )
}
pub fn command(s: &Session, body: &Value) -> Result<Value, String> {
    if let Some(writer) = &s.workflow.writer {
        let b = body.clone();
        return writer.call(move |s| command(s, &b));
    }
    crate::conversation_store::valid_command(body, "project.setup").map_err(|e| e.to_string())?;
    let mut c = s.workflow.connection()?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    if let Some(out) = crate::conversation_store::prior(&tx, body)? {
        return Ok(out);
    }
    let previous = settings(s)?;
    let mut next = body["payload"]["settings"].clone();
    validate(&next)?;
    let revision = previous
        .as_ref()
        .map(|v| v["revision"].as_u64().unwrap_or(0))
        .unwrap_or(0);
    if body["payload"]["expected_revision"] != revision {
        return Err("Project setup changed; reload before saving".into());
    }
    next["revision"] = json!(revision + 1);
    // Reversible preferences only. Execution/import/publication wait for scope approval.
    write(&s.project.join(".waywr/workflow.json"), &next, false)?;
    crate::conversation_store::event(
        &tx,
        "project.setup_saved",
        None,
        &json!({"settings":next,"subject_id":body["payload"]["subject_id"],"applies":"future requests only"}),
    )?;
    let out = json!({"status":"accepted","revision":revision+1});
    crate::conversation_store::commit(&tx, body, &out)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(out)
}
pub fn require_ready(policy: &Value) -> Result<(), String> {
    if policy["project_workflow"].is_object() && policy["project_workflow"]["ready"] != true {
        return Err(format!(
            "Project setup needs a decision: {}",
            policy["project_workflow"]["blockers"]
        ));
    }
    Ok(())
}
/// Called only after exact scope/policy approval, inside existing development initialization.
pub fn baseline(s: &Session, w: &Value) -> Result<Option<(String, String, String)>, String> {
    let p = &w["policy"]["project_workflow"];
    if !p.is_object() {
        return Ok(None);
    }
    require_ready(&w["policy"])?;
    let target = text(p, "target")?.to_owned();
    let mut base = p["baseline_commit"].as_str().map(str::to_owned);
    if p["baseline_input"] == "initialize-files" {
        let manifest = read(Path::new(text(&p["input_manifest"], "path")?))?;
        if hash(&manifest)? != p["input_manifest"]["sha256"]
            || manifest["project"] != json!(s.project)
            || json!(input_files(&s.project)?) != manifest["files"]
        {
            return Err(
                "Approved initial source changed; preserve files and revise the baseline decision"
                    .into(),
            );
        }
        let directory = s
            .workflow
            .state
            .join("session/setup")
            .join(w["work_id"].as_str().ok_or("Work identity missing")?);
        let intent = directory.join("intent.json");
        write(
            &intent,
            &json!({"project":s.project,"target":target,"manifest_sha256":p["input_manifest"]["sha256"],"scope":w["scope_base"]}),
            true,
        )?;
        if !s.project.join(".git").exists() {
            git(&s.project, &["init", "-b", &target])?;
        }
        exclude_state(&s.project)?;
        let commit_path = directory.join("baseline.json");
        let commit = if commit_path.exists() {
            text(&read(&commit_path)?, "commit")?.to_owned()
        } else {
            if git(&s.project, &["rev-parse", "HEAD"]).is_ok() {
                return Err(
                    "A commit appeared during initial baselining; preserve and reconcile".into(),
                );
            }
            let index = directory.join("index");
            let files = manifest["files"]
                .as_object()
                .ok_or("Initial file manifest missing")?;
            let run = |args: &[&str]| -> Result<String, String> {
                let out = Command::new("git")
                    .arg("-C")
                    .arg(&s.project)
                    .args(args)
                    .env("GIT_INDEX_FILE", &index)
                    .output()
                    .map_err(|e| e.to_string())?;
                if !out.status.success() {
                    return Err(String::from_utf8_lossy(&out.stderr).to_string());
                }
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
            };
            run(&["read-tree", "--empty"])?;
            for path in files.keys() {
                run(&["add", "--", path])?;
            }
            let tree = run(&["write-tree"])?;
            let commit = git(
                &s.project,
                &[
                    "-c",
                    "user.name=Waywr",
                    "-c",
                    "user.email=waywr@localhost",
                    "commit-tree",
                    &tree,
                    "-m",
                    "Explicitly approved project source baseline",
                ],
            )?;
            write(
                &commit_path,
                &json!({"commit":commit,"tree":tree,"manifest":p["input_manifest"]}),
                true,
            )?;
            commit
        };
        let current = git(
            &s.project,
            &["rev-parse", "--verify", &format!("refs/heads/{target}")],
        )
        .ok();
        if current.as_deref() != Some(&commit) {
            if current.is_some() {
                return Err("Initial target changed; preserve and reconcile".into());
            }
            git(
                &s.project,
                &[
                    "update-ref",
                    &format!("refs/heads/{target}"),
                    &commit,
                    "0000000000000000000000000000000000000000",
                ],
            )?;
        }
        // A new empty index can be initialized; preserve any pre-existing staged input.
        if git(&s.project, &["ls-files"])
            .unwrap_or_default()
            .is_empty()
        {
            git(&s.project, &["read-tree", &commit])?;
        }
        base = Some(commit);
    }
    let base = base.ok_or("Approved baseline unavailable")?;
    git(
        &s.project,
        &["cat-file", "-e", &format!("{base}^{{commit}}")],
    )?;
    if p["target_commit"].is_null() && p["baseline_input"] == "committed" {
        let intent = s
            .workflow
            .state
            .join("session/setup")
            .join(w["work_id"].as_str().ok_or("Work identity missing")?)
            .join("target.json");
        let current = ref_head(&s.project, &target).ok();
        if !intent.exists() && current.is_some() {
            return Err("A previously absent target appeared after approval; preserve and review its baseline".into());
        }
        write(
            &intent,
            &json!({"target":target,"base":base,"scope":w["scope_base"]}),
            true,
        )?;
        if current.as_deref() != Some(&base) {
            if current.is_some() {
                return Err("New target differs from saved creation intent".into());
            }
            git(
                &s.project,
                &[
                    "update-ref",
                    &format!("refs/heads/{target}"),
                    &base,
                    "0000000000000000000000000000000000000000",
                ],
            )?;
        }
    }
    let expected = p["target_commit"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| base.clone());
    Ok(Some((base, target, expected)))
}

pub fn target_checkout(project: &Path, target: &str) -> Result<Option<PathBuf>, String> {
    let listing = git(project, &["worktree", "list", "--porcelain"])?;
    for block in listing.split("\n\n") {
        if block
            .lines()
            .any(|l| l == format!("branch refs/heads/{target}"))
        {
            return block
                .lines()
                .find_map(|l| l.strip_prefix("worktree "))
                .map(PathBuf::from)
                .ok_or("Target worktree location missing".into())
                .map(Some);
        }
    }
    Ok(None)
}
pub fn ref_head(project: &Path, branch: &str) -> Result<String, String> {
    git(
        project,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch}^{{commit}}"),
        ],
    )
}
pub fn advance_target(
    project: &Path,
    target: &str,
    expected: &str,
    commit: &str,
) -> Result<String, String> {
    if !valid_branch(target) {
        return Err("Target branch invalid".into());
    }
    git(project, &["merge-base", "--is-ancestor", expected, commit])?;
    let actual = ref_head(project, target)?;
    if actual == commit {
        return Ok(actual);
    }
    if actual != expected {
        return Err("Target branch advanced since approval/candidate; integrate and check its new revision before delivery".into());
    }
    if let Some(owner) = target_checkout(project, target)? {
        if !git(
            &owner,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?
        .is_empty()
        {
            return Err(format!(
                "Target checkout {} has local changes; preserved. Delivery waits for a clean target checkout or the approved PR policy",
                owner.display()
            ));
        }
        if git(&owner, &["branch", "--show-current"])? != target {
            return Err("Target checkout ownership changed; preserved".into());
        }
        git(&owner, &["merge", "--ff-only", commit])?;
    } else {
        git(
            project,
            &[
                "update-ref",
                &format!("refs/heads/{target}"),
                commit,
                expected,
            ],
        )?;
    }
    ref_head(project, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repo(project: &Path) -> String {
        git(project, &["init", "-b", "trunk"]).unwrap();
        fs::write(project.join("code.txt"), "committed input\n").unwrap();
        git(project, &["add", "code.txt"]).unwrap();
        git(
            project,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "baseline",
            ],
        )
        .unwrap();
        git(project, &["rev-parse", "HEAD"]).unwrap()
    }
    fn commit(project: &Path) -> String {
        git(project, &["add", "--all"]).unwrap();
        git(
            project,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "changed source",
            ],
        )
        .unwrap();
        git(project, &["rev-parse", "HEAD"]).unwrap()
    }
    #[test]
    fn ordinary_setup_retains_legacy_policy_and_makes_validation_choices_explicit() {
        let (_d, s) = crate::provider_selection::recovery_tests::fixture();
        assert!(settings(&s).unwrap().is_none());
        let before = crate::development::policy(&s).unwrap();
        assert!(before["project_workflow"].is_null());
        let head = repo(&s.project);
        initialize(&s.project).unwrap();
        let mut p = settings(&s).unwrap().unwrap();
        assert_eq!(p["target"], "trunk");
        assert_eq!(p["human_validation"], false);
        for (n, validation) in [(1, false), (2, true)] {
            p["human_validation"] = json!(validation);
            let out=command(&s,&json!({"command_id":format!("settings-{n}"),"action":"project.setup","payload":{"expected_revision":n,"settings":p}})).unwrap();
            assert_eq!(out["status"], "accepted");
            let effective = policy(&s).unwrap().unwrap();
            assert_eq!(effective["baseline_commit"], head);
            assert_eq!(effective["human_validation"], validation);
            assert_eq!(effective["ready"], true);
            p = settings(&s).unwrap().unwrap();
        }
        assert!(!s.project.join(".gitignore").exists());
        assert!(
            git(&s.project, &["status", "--porcelain"])
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn ordinary_new_target_creation_reconciles_exact_intent_without_switching_primary() {
        let (_d, s) = crate::provider_selection::recovery_tests::fixture();
        let head = repo(&s.project);
        initialize(&s.project).unwrap();
        let mut v = settings(&s).unwrap().unwrap();
        v["target"] = json!("stable");
        write(&s.project.join(".waywr/workflow.json"), &v, false).unwrap();
        let policy = policy(&s).unwrap().unwrap();
        assert!(policy["target_commit"].is_null());
        let w = json!({"work_id":"new-target","scope_base":"approved","policy":{"project_workflow":policy}});
        let first = baseline(&s, &w).unwrap().unwrap();
        assert_eq!(first.2, head);
        assert_eq!(ref_head(&s.project, "stable").unwrap(), head);
        assert_eq!(baseline(&s, &w).unwrap(), Some(first));
        assert_eq!(
            git(&s.project, &["branch", "--show-current"]).unwrap(),
            "trunk"
        );
    }
    #[test]
    fn ordinary_target_advances_without_touching_dirty_primary_feature_checkout() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let base = repo(root);
        git(root, &["checkout", "-b", "feature"]).unwrap();
        fs::write(root.join("unrelated.txt"), "retain working input").unwrap();
        let tree = root.join("candidate");
        git(
            root,
            &[
                "worktree",
                "add",
                "-b",
                "governing",
                tree.to_str().unwrap(),
                &base,
            ],
        )
        .unwrap();
        fs::write(tree.join("code.txt"), "checked revision\n").unwrap();
        let result = commit(&tree);
        advance_target(root, "trunk", &base, &result).unwrap();
        assert_eq!(ref_head(root, "trunk").unwrap(), result);
        assert_eq!(git(root, &["branch", "--show-current"]).unwrap(), "feature");
        assert_eq!(
            fs::read_to_string(root.join("unrelated.txt")).unwrap(),
            "retain working input"
        );
        assert_eq!(
            fs::read_to_string(root.join("code.txt")).unwrap(),
            "committed input\n"
        );
        assert!(advance_target(root, "trunk", &base, &base).is_err());
        assert_eq!(ref_head(root, "trunk").unwrap(), result);
    }
    #[test]
    fn ordinary_missing_baseline_requires_explicit_exact_file_import_and_preserves_existing_policy()
    {
        let (_d, s) = crate::provider_selection::recovery_tests::fixture();
        fs::write(s.project.join("existing.txt"), "human-owned source").unwrap();
        initialize(&s.project).unwrap();
        assert_eq!(policy(&s).unwrap().unwrap()["ready"], false);
        let mut settings = settings(&s).unwrap().unwrap();
        settings["baseline_input"] = json!("initialize-files");
        settings["target"] = json!("release-line");
        write(&s.project.join(".waywr/workflow.json"), &settings, false).unwrap();
        let p = policy(&s).unwrap().unwrap();
        assert_eq!(p["ready"], true);
        assert!(!s.project.join(".git").exists());
        let w = json!({"work_id":"fixture-source","scope_base":"approved-scope","policy":{"project_workflow":p}});
        fs::write(s.project.join("existing.txt"), "changed after approval").unwrap();
        assert!(baseline(&s, &w).unwrap_err().contains("changed"));
        assert!(!s.project.join(".git").exists());
        fs::write(s.project.join("existing.txt"), "human-owned source").unwrap();
        let (base, target, _) = baseline(&s, &w).unwrap().unwrap();
        assert_eq!(target, "release-line");
        assert_eq!(
            git(&s.project, &["show", &format!("{base}:existing.txt")]).unwrap(),
            "human-owned source"
        );
        assert_eq!(
            fs::read_to_string(s.project.join("existing.txt")).unwrap(),
            "human-owned source"
        );
    }
    #[test]
    fn ordinary_project_index_refreshes_nested_constraints_and_keeps_dirty_evidence_separate() {
        let (_d, s) = crate::provider_selection::recovery_tests::fixture();
        repo(&s.project);
        exclude_state(&s.project).unwrap();
        fs::create_dir_all(s.project.join("nested")).unwrap();
        fs::write(s.project.join("AGENTS.md"), "Root instructions").unwrap();
        fs::write(s.project.join("nested/AGENTS.md"), "Nested constraints").unwrap();
        fs::write(s.project.join("nested/source.go"), "source").unwrap();
        fs::write(s.project.join(".env"), "fixture only").unwrap();
        let first = commit(&s.project);
        let index = crate::project_knowledge::refresh(&s, &first).unwrap();
        assert!(index["paths"].get(".env").is_none());
        let applicable =
            crate::project_knowledge::applicable(&s.project, &first, &["nested/source.go".into()])
                .unwrap();
        assert_eq!(applicable.len(), 2);
        fs::write(
            s.project.join("nested/AGENTS.md"),
            "Updated nested constraints",
        )
        .unwrap();
        let observation = crate::project_knowledge::planning(&s).unwrap();
        assert_eq!(observation["head"], first);
        assert_eq!(
            observation["dirty_observations"][0]["path"],
            "nested/AGENTS.md"
        );
        let second = commit(&s.project);
        let refreshed = crate::project_knowledge::refresh(&s, &second).unwrap();
        assert!(
            refreshed["delta"]["changed_paths"]
                .as_array()
                .unwrap()
                .contains(&json!("nested/AGENTS.md"))
        );
        assert_eq!(
            crate::project_knowledge::applicable(&s.project, &first, &["nested/source.go".into()])
                .unwrap()[1]["text"],
            "Nested constraints"
        );
    }
}
