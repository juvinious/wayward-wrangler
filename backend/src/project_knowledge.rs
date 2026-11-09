//! Incremental exact-revision project map, separately labelled mutable observations.
use crate::{
    development::git,
    execution::{read, write},
    session_store::Session,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
pub fn allowed_path(path: &str) -> bool {
    !path.split('/').any(|c| {
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
        ) || c.starts_with(".env.")
            || c.ends_with(".pem")
            || c.ends_with(".key")
            || c == "credentials.json"
            || c == "auth.json"
    })
}
fn instruction(path: &str) -> bool {
    matches!(path.rsplit('/').next(), Some("AGENTS.md" | "CLAUDE.md"))
}
fn convention(path: &str) -> bool {
    instruction(path)
        || matches!(
            path.rsplit('/').next(),
            Some(
                "README.md"
                    | "CONTRIBUTING.md"
                    | "Cargo.toml"
                    | "Cargo.lock"
                    | "rust-toolchain.toml"
                    | "package.json"
                    | "package-lock.json"
                    | "pnpm-lock.yaml"
                    | "pyproject.toml"
                    | "uv.lock"
                    | "go.mod"
                    | "go.sum"
                    | "Makefile"
                    | "CMakeLists.txt"
                    | "Gemfile"
                    | "pom.xml"
                    | "build.gradle"
            )
        )
        || path.starts_with(".github/workflows/")
}
pub fn index(project: &Path, commit: &str) -> Result<Value, String> {
    let mut paths = BTreeMap::new();
    let mut omitted = vec![];
    for entry in git(project, &["ls-tree", "-rz", "--full-tree", commit])?
        .split('\0')
        .filter(|v| !v.is_empty())
    {
        let (meta, path) = entry.split_once('\t').ok_or("Invalid Git path metadata")?;
        let fields = meta.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("Invalid Git object entry".into());
        }
        if !allowed_path(path) || !matches!(fields[0], "100644" | "100755") {
            omitted.push(json!({"path":path,"reason":"excluded credential/state/dependency path or non-ordinary Git blob"}));
            continue;
        }
        paths.insert(
            path.to_owned(),
            json!({"blob":fields[2],"instruction":instruction(path),"convention":convention(path)}),
        );
    }
    Ok(
        json!({"version":1,"commit":commit,"paths":paths,"omissions":omitted,"completeness":"All eligible tracked ordinary blobs at this commit; credential/state/dependency paths, symlinks and submodules omitted explicitly"}),
    )
}
pub fn refresh(s: &Session, commit: &str) -> Result<Value, String> {
    let path = s.workflow.state.join("knowledge/project-index.json");
    let prior = read(&path).unwrap_or(Value::Null);
    if prior["commit"] == commit {
        return Ok(prior);
    }
    let mut next = index(&s.project, commit)?;
    let paths = next["paths"].as_object().ok_or("Index paths missing")?;
    let changed = paths
        .iter()
        .filter(|(p, v)| prior["paths"][p.as_str()]["blob"] != v["blob"])
        .map(|(p, _)| p.clone())
        .collect::<Vec<_>>();
    let removed = prior["paths"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(p, _)| !paths.contains_key(*p))
        .map(|(p, _)| p.clone())
        .collect::<Vec<_>>();
    next["delta"] =
        json!({"previous_commit":prior["commit"],"changed_paths":changed,"removed_paths":removed});
    // Cache is derived evidence; no approval or requirements live here.
    write(&path, &next, false)?;
    Ok(next)
}
pub fn applicable(project: &Path, commit: &str, paths: &[String]) -> Result<Vec<Value>, String> {
    let index = index(project, commit)?;
    let mut out = vec![];
    for (path, meta) in index["paths"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| v["instruction"] == true)
    {
        let parent = Path::new(path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_str()
            .unwrap_or("");
        if !parent.is_empty()
            && !paths.iter().any(|p| {
                p == parent
                    || p.starts_with(&format!("{parent}/"))
                    || parent.starts_with(&format!("{}/", p.trim_end_matches('/')))
            })
        {
            continue;
        }
        let content = git(project, &["show", &format!("{commit}:{path}")])?;
        out.push(json!({"path":path,"commit":commit,"blob":meta["blob"],"text":content,"label":"Applicable project instructions at exact source; subordinate to frozen scope/policy"}));
    }
    Ok(out)
}
pub fn planning(s: &Session) -> Result<Value, String> {
    let Some(head) = git(&s.project, &["rev-parse", "HEAD"]).ok() else {
        return Ok(
            json!({"project":s.project,"head":null,"documents":[],"tracked_files":[],"setup":crate::project_workflow::snapshot(s)?,"missing":"No committed source; choose baseline in F3 → Project"}),
        );
    };
    let index = refresh(s, &head)?;
    let mut documents = vec![];
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "Makefile",
    ] {
        if index["paths"].get(path).is_none() {
            continue;
        }
        let text = git(&s.project, &["show", &format!("{head}:{path}")])?;
        let excerpt = text.chars().take(4096).collect::<String>();
        documents.push(json!({"path":path,"commit":head,"blob":index["paths"][path]["blob"],"truncated":excerpt.len()<text.len(),"excerpt":excerpt,"full":"Exact source retrieval by path"}));
    }
    let dirty=git(&s.project,&["diff","--name-only","HEAD"])?.lines().filter(|p|allowed_path(p)).map(|p|json!({"path":p,"sha256":std::fs::symlink_metadata(s.project.join(p)).ok().filter(|m|m.file_type().is_file()).and_then(|_|crate::execution::hash_file(&s.project.join(p)).ok()),"source":"mutable working input, excluded from committed baseline"})).collect::<Vec<_>>();
    let files = index["paths"].as_object().ok_or("Missing path index")?;
    Ok(
        json!({"project":s.project,"head":head,"documents":documents,"tracked_files":files.keys().take(160).collect::<Vec<_>>(),"project_map":{"eligible_files":files.len(),"instruction_paths":files.iter().filter(|(_,v)|v["instruction"]==true).map(|(p,_)|p.clone()).collect::<Vec<_>>(),"convention_paths":files.iter().filter(|(_,v)|v["convention"]==true).map(|(p,_)|p.clone()).collect::<Vec<_>>(),"evidence_id":format!("project-index:{head}"),"cache":s.workflow.state.join("knowledge/project-index.json"),"delta":index["delta"],"omissions":index["omissions"]},"dirty_observations":dirty,"setup":crate::project_workflow::snapshot(s)?,"provenance":"Incremental committed path/blob map with separate mutable observations; project context never grants execution authority"}),
    )
}
pub fn recipe_allowed(policy: &Value, argv: &[String]) -> bool {
    policy["project_workflow"]["command_recipes"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|r| {
            matches!(r["kind"].as_str(), Some("check" | "build" | "inspect"))
                && r["argv"] == json!(argv)
        })
}

pub fn recipes(project: &Path, commit: &str) -> Vec<Value> {
    let mut out = vec![];
    let exists =
        |name: &str| git(project, &["cat-file", "-e", &format!("{commit}:{name}")]).is_ok();
    if exists("Cargo.toml") {
        for args in [
            vec!["cargo", "build", "--locked"],
            vec!["cargo", "test", "--locked"],
        ] {
            out.push(json!({"kind":if args[1]=="build"{"build"}else{"check"},"argv":args,"source":"Cargo.toml; suggested, not executed"}))
        }
    }
    if let Ok(p) = git(project, &["show", &format!("{commit}:package.json")])
        .and_then(|v| serde_json::from_str::<Value>(&v).map_err(|e| e.to_string()))
    {
        let manager = if exists("pnpm-lock.yaml") {
            "pnpm"
        } else if exists("yarn.lock") {
            "yarn"
        } else {
            "npm"
        };
        for script in ["test", "build", "lint", "typecheck"] {
            if p["scripts"][script].is_string() {
                out.push(json!({"kind":if script=="build"{"build"}else{"check"},"argv":[manager,"run",script],"source":"package.json scripts and lockfile; suggested, not executed"}));
            }
        }
    }
    if exists("go.mod") {
        out.push(json!({"kind":"check","argv":["go","test","./..."],"source":"go.mod; suggested, not executed"}));
        out.push(json!({"kind":"build","argv":["go","build","./..."],"source":"go.mod; suggested, not executed"}));
    }
    out
}
