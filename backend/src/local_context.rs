//! Explicit current-project context selection. Only named, approved records and
//! project-local documents enter a frozen packet; the location registry is not read.
use crate::codec::{canonical, canonical_sha256, sha256};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn id(s: &str) -> Result<&str, String> {
    let b = s.as_bytes();
    if b.is_empty()
        || b.len() > 64
        || !b[0].is_ascii_alphanumeric()
        || !b[1..]
            .iter()
            .all(|x| x.is_ascii_alphanumeric() || matches!(x, b'-' | b'_'))
    {
        return Err("Use a short alphanumeric ID with hyphens/underscores".into());
    }
    Ok(s)
}
fn str_field<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .ok_or_else(|| format!("missing or invalid {k}"))
}
fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(err)
}
fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}

#[derive(Debug, Clone)]
pub struct LocalContext {
    pub project: PathBuf,
    pub state: PathBuf,
    pub key: String,
}
impl LocalContext {
    pub fn new(project: &Path, state: &Path, key: &str) -> Result<Self, String> {
        let project = project.canonicalize().map_err(err)?;
        let state = state.canonicalize().map_err(err)?;
        if !project.is_dir() || !state.is_dir() {
            return Err("context project/state unavailable".into());
        }
        if read(&state.join("project-identity.json"))?
            != json!({"schema_version":1,"project_key":key})
        {
            return Err("Selected project/state identity mismatch".into());
        }
        Ok(Self {
            project,
            state,
            key: key.into(),
        })
    }
    fn project_file(&self, path: &Path) -> Result<PathBuf, String> {
        let path = path.canonicalize().map_err(err)?;
        if !path.starts_with(&self.project) || !path.is_file() {
            return Err("Context documents/selection must be existing project-local files".into());
        }
        Ok(path)
    }
    fn db(&self) -> Result<Connection, String> {
        let db = Connection::open_with_flags(
            self.state.join("state.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(err)?;
        db.pragma_update(None, "query_only", "ON").map_err(err)?;
        Ok(db)
    }
    fn record(
        &self,
        db: &Connection,
        table: &str,
        run: &str,
        request: &str,
    ) -> Result<Value, String> {
        if !matches!(table, "app_requests" | "app_responses") {
            return Err("invalid record table".into());
        }
        let raw: String = db
            .query_row(
                &format!("SELECT payload FROM {table} WHERE run_id=?1 AND request_id=?2"),
                (run, request),
                |r| r.get(0),
            )
            .map_err(|_| format!("Missing source request/answer: {run}/{request}"))?;
        serde_json::from_str(&raw).map_err(err)
    }
    fn response(&self, db: &Connection, run: &str, request: &str) -> Result<Value, String> {
        let response = self.record(db, "app_responses", run, request)?;
        let export = self
            .state
            .join("runs")
            .join(run)
            .join("artifacts")
            .join(format!("decision-{request}.json"));
        if export.exists() && read(&export)? != response {
            return Err("Source response export disagrees with canonical record".into());
        }
        Ok(response)
    }
    fn artifact_content(&self, run: &str, reference: &Value) -> Result<String, String> {
        let root = self
            .state
            .join("runs")
            .join(id(run)?)
            .canonicalize()
            .map_err(err)?;
        let path = root
            .join(str_field(reference, "path")?)
            .canonicalize()
            .map_err(err)?;
        if !path.starts_with(&root)
            || !path.is_file()
            || sha256(&fs::read(&path).map_err(err)?) != str_field(reference, "sha256")?
        {
            return Err("Artifact path/hash validation failed".into());
        }
        fs::read_to_string(path).map_err(err)
    }
    pub fn resolve(
        &self,
        selection_path: &Path,
        exclude_run: Option<&str>,
    ) -> Result<Value, String> {
        let path = self.project_file(selection_path)?;
        let bytes = fs::read(&path).map_err(err)?;
        let selection: toml::Value = toml::from_slice(&bytes).map_err(err)?;
        let selection = serde_json::to_value(selection).map_err(err)?;
        if selection["schema_version"] != 1 || selection["scope"].as_str().is_none_or(str::is_empty)
        {
            return Err("Context selection requires version 1 and explicit scope".into());
        }
        let project_id = id(str_field(&selection, "project_id")?)?;
        let scope = str_field(&selection, "scope")?;
        let db = self.db()?;
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        let mut dependencies = BTreeMap::<String, String>::new();
        for spec in selection["entries"].as_array().into_iter().flatten() {
            if spec["reason"].as_str().is_none_or(str::is_empty) || spec["scope"] != scope {
                return Err(
                    "Each context entry needs relevance and matching explicit scope".into(),
                );
            }
            let kind = str_field(spec, "kind")?;
            let mut base = Map::new();
            base.insert("kind".into(), kind.into());
            base.insert("scope".into(), scope.into());
            base.insert("relevance_reason".into(), spec["reason"].clone());
            let mut items = Vec::new();
            if kind == "document" {
                let relative = str_field(spec, "path")?;
                let original = self.project_file(
                    &path
                        .parent()
                        .ok_or("invalid selection parent")?
                        .join(relative),
                )?;
                let original_text = fs::read_to_string(&original).map_err(err)?;
                let original_rel = original
                    .strip_prefix(&self.project)
                    .map_err(err)?
                    .to_string_lossy()
                    .into_owned();
                dependencies.insert(original_rel, sha256(original_text.as_bytes()));
                let doc = if original.file_name().is_some_and(|n| n == "CLAUDE.md")
                    && original_text.contains("[AGENTS.md](AGENTS.md)")
                {
                    self.project_file(
                        &original
                            .parent()
                            .ok_or("invalid document parent")?
                            .join("AGENTS.md"),
                    )?
                } else {
                    original
                };
                let text = fs::read_to_string(&doc).map_err(err)?;
                let relative = doc
                    .strip_prefix(&self.project)
                    .map_err(err)?
                    .to_string_lossy()
                    .into_owned();
                dependencies.insert(relative.clone(), sha256(text.as_bytes()));
                base.insert("qualified_id".into(), format!("document:{relative}").into());
                base.insert("source_path".into(), relative.into());
                base.insert(
                    "source_record_digest".into(),
                    sha256(text.as_bytes()).into(),
                );
                base.insert("content".into(), text.into());
                items.push(Value::Object(base));
            } else {
                let run = id(str_field(spec, "run")?)?;
                let (raw, status): (String, String) = db
                    .query_row(
                        "SELECT envelope,status FROM app_runs WHERE run_id=?1",
                        [run],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(|_| "Selected source run is not approved".to_owned())?;
                if status != "approved" {
                    return Err("Selected source run is not approved".into());
                }
                let source: Value = serde_json::from_str(&raw).map_err(err)?;
                if source
                    .get("project_key")
                    .is_some_and(|key| key != &self.key)
                {
                    return Err("Selected source run belongs to another project identity".into());
                }
                let approval_id = id(spec["approval"].as_str().unwrap_or("A1"))?;
                let approval = self.response(&db, run, approval_id)?;
                let final_request = self.record(&db, "app_requests", run, approval_id)?;
                if approval["outcome"] != "approved"
                    || approval["artifact_sha256"] != final_request["artifact_sha256"]
                    || approval["decision_revision"] != final_request["decision_revision"]
                {
                    return Err("Source approval does not bind its final artifact/decisions".into());
                }
                self.artifact_content(run, &final_request["artifact"])?;
                base.insert("source_run".into(), run.into());
                base.insert(
                    "authority_ref".into(),
                    format!("{run}/{approval_id}").into(),
                );
                base.insert(
                    "authority_digest".into(),
                    hash(&json!({"request":final_request,"response":approval}))?.into(),
                );
                if kind == "decision" {
                    let key = id(str_field(spec, "id")?)?;
                    let request = self.record(&db, "app_requests", run, key)?;
                    let response = self.response(&db, run, key)?;
                    let accepted = final_request["decisions"].as_array().is_some_and(|a| {
                        a.iter()
                            .any(|d| d["request_id"] == key && d["answer"] == response["answer"])
                    });
                    if request["kind"] != "choice" || response["outcome"] != "answered" || !accepted
                    {
                        return Err("Decision is not accepted in approved source context".into());
                    }
                    if response["artifact_sha256"] != request["artifact_sha256"]
                        || response["decision_revision"] != request["decision_revision"]
                    {
                        return Err("Source decision binding mismatch".into());
                    }
                    base.insert("qualified_id".into(), format!("{run}/{key}").into());
                    base.insert("topic_key".into(), spec["topic_key"].clone());
                    base.insert(
                        "source_record_digest".into(),
                        hash(&json!({"request":request,"response":response}))?.into(),
                    );
                    base.insert(
                        "content".into(),
                        json!({"question":request["question"],"response":response}),
                    );
                    items.push(Value::Object(base));
                } else if kind == "artifact" {
                    let key = id(str_field(spec, "id")?)?;
                    let artifact = read(
                        &self
                            .state
                            .join("runs")
                            .join(run)
                            .join("artifacts")
                            .join(format!("{key}.json")),
                    )?;
                    let reference = &artifact["manifest"];
                    let text = self.artifact_content(run, reference)?;
                    if *reference != final_request["artifact"] {
                        return Err("Selected artifact is not the approved source version".into());
                    }
                    base.insert(
                        "qualified_id".into(),
                        format!("{run}/{}", str_field(reference, "id")?).into(),
                    );
                    base.insert("artifact_sha256".into(), reference["sha256"].clone());
                    base.insert("source_record_digest".into(), hash(reference)?.into());
                    base.insert("content".into(), text.into());
                    items.push(Value::Object(base));
                } else if kind == "findings" {
                    let source_findings = final_request["findings"]
                        .as_object()
                        .ok_or("invalid source findings")?;
                    let ids = spec["ids"].as_array().ok_or("missing finding IDs")?;
                    let selected = if ids == &vec![json!("open")] {
                        source_findings
                            .iter()
                            .filter(|(_, v)| v["status"] == "open")
                            .map(|(k, _)| k.clone())
                            .collect::<Vec<_>>()
                    } else {
                        ids.iter()
                            .map(|v| v.as_str().ok_or("invalid finding ID").map(str::to_owned))
                            .collect::<Result<Vec<_>, _>>()?
                    };
                    for key in selected {
                        let finding = source_findings
                            .get(&key)
                            .ok_or("Selected finding is missing or no longer open")?;
                        if finding["status"] != "open" {
                            return Err("Selected finding is missing or no longer open".into());
                        }
                        let origin = id(str_field(finding, "origin")?)?;
                        let origin_record = read(
                            &self
                                .state
                                .join("runs")
                                .join(run)
                                .join("artifacts")
                                .join(format!("{origin}.json")),
                        )?;
                        self.artifact_content(run, &origin_record["manifest"])?;
                        let ledger_path = self
                            .state
                            .join("runs")
                            .join(run)
                            .join("quality")
                            .join(format!("{origin}-finding-ledger.json"));
                        let supported = if ledger_path.exists() {
                            let ledger = read(&ledger_path)?;
                            ledger["result_sha256"] == hash(&origin_record["result"])?
                                && ledger["new_findings"].as_array().is_some_and(|a| {
                                    a.iter().any(|v| {
                                        v["id"] == key && v["evidence"] == finding["evidence"]
                                    })
                                })
                        } else {
                            origin_record["result"]["findings"]
                                .as_array()
                                .is_some_and(|a| {
                                    a.iter().any(|v| {
                                        v["id"] == key && v["evidence"] == finding["evidence"]
                                    })
                                })
                        };
                        if !supported {
                            return Err("Finding origin does not support its evidence".into());
                        }
                        let mut item = base.clone();
                        item.insert("qualified_id".into(), format!("{run}/{key}").into());
                        item.insert("source_record_digest".into(), hash(finding)?.into());
                        let mut content =
                            finding.as_object().ok_or("invalid source finding")?.clone();
                        content.insert("id".into(), format!("{run}/{key}").into());
                        content.insert("source_id".into(), key.into());
                        item.insert("content".into(), Value::Object(content));
                        items.push(Value::Object(item));
                    }
                    dependencies.insert(format!("findings:{run}"), hash(&final_request)?);
                } else {
                    return Err("Unknown context entry kind".into());
                }
            }
            for entry in items {
                let qualified = str_field(&entry, "qualified_id")?;
                if !seen.insert(qualified.to_owned()) {
                    if kind == "document" {
                        continue;
                    }
                    return Err("Duplicate context reference".into());
                }
                entries.push(entry);
            }
        }
        let mut topics = BTreeSet::new();
        for entry in entries.iter().filter(|e| e["kind"] == "decision") {
            if !topics.insert(str_field(entry, "topic_key")?) {
                return Err("Conflicting decisions selected for one topic".into());
            }
        }
        // An explicit supersession has authority only in an approved run of
        // this selection's project_id; unrelated projects are never queried.
        let mut stmt = db
            .prepare("SELECT run_id,envelope FROM app_runs WHERE status='approved'")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(err)?;
        for row in rows {
            let (run, raw) = row.map_err(err)?;
            if exclude_run == Some(run.as_str()) {
                continue;
            }
            let env: Value = serde_json::from_str(&raw).map_err(err)?;
            if env["context_snapshot"]["project_id"] != project_id {
                continue;
            }
            let mut responses = db
                .prepare("SELECT payload FROM app_responses WHERE run_id=?1")
                .map_err(err)?;
            let values = responses
                .query_map([&run], |r| r.get::<_, String>(0))
                .map_err(err)?;
            for value in values {
                let response: Value = serde_json::from_str(&value.map_err(err)?).map_err(err)?;
                if response["supersedes"].as_array().is_some_and(|a| {
                    a.iter()
                        .any(|v| v.as_str().is_some_and(|id| seen.contains(id)))
                }) {
                    return Err("Selected context has an explicit approved supersession".into());
                }
            }
        }
        let relative = path
            .strip_prefix(&self.project)
            .map_err(err)?
            .to_string_lossy()
            .into_owned();
        let snapshot = json!({"schema_version":2,"location_format":2,"project_key":self.key,
            "project_id":project_id,"scope":scope,"selection_path":relative,
            "selection_digest":sha256(&bytes),"dependencies":dependencies,"entries":entries});
        if canonical(&snapshot).map_err(err)?.len() > 48_000 {
            return Err("Selected context exceeds 48 KB; narrow explicit selection".into());
        }
        Ok(snapshot)
    }
    pub fn check(
        &self,
        snapshot: &Value,
        expected_digest: &str,
        exclude_run: Option<&str>,
    ) -> Result<(), String> {
        if snapshot["schema_version"] != 2 || snapshot["project_key"] != self.key {
            return Err("Stale context; no continuation: Project identity changed".into());
        }
        let selection =
            self.project_file(&self.project.join(str_field(snapshot, "selection_path")?))?;
        let recomputed = self
            .resolve(&selection, exclude_run)
            .map_err(|e| format!("Stale context; no continuation: {e}"))?;
        if hash(snapshot)? != expected_digest || recomputed != *snapshot {
            return Err(
                "Stale context; no continuation: Selected sources or selection changed".into(),
            );
        }
        Ok(())
    }
}
