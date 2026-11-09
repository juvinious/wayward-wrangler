//! Explicit, read-only cross-project selection. No registry or model discovery.
use crate::codec::{canonical, canonical_sha256, sha256};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_FOREIGN_BYTES: usize = 16 * 1024;
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
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .ok_or_else(|| format!("missing or invalid {k}"))
}
fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}
fn hash(value: &Value) -> Result<String, String> {
    canonical_sha256(value).map_err(err)
}
fn content(path: &Path, root: &Path, expected: &str) -> Result<String, String> {
    let path = path.canonicalize().map_err(err)?;
    if !path.starts_with(root) || sha256(&fs::read(&path).map_err(err)?) != expected {
        return Err("Source artifact hash/path changed".into());
    }
    fs::read_to_string(path).map_err(err)
}

#[derive(Debug, Clone)]
pub struct SourceBinding {
    pub project: PathBuf,
    pub state: PathBuf,
    pub key: String,
}
impl SourceBinding {
    pub fn new(project: &Path, state: Option<&Path>) -> Result<Self, String> {
        let project = project.canonicalize().map_err(err)?;
        if !project.is_dir() {
            return Err("Explicit source project is not a directory".into());
        }
        let config =
            fs::read_to_string(project.join(".waywr/project.toml")).map_err(err)?;
        let config: toml::Value = toml::from_str(&config).map_err(err)?;
        let key = config
            .get("project_key")
            .and_then(toml::Value::as_str)
            .ok_or("Source project identity is invalid")?;
        if config
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            != Some(1)
            || key.len() != 32
            || !key
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("Source project identity is invalid".into());
        }
        let state = state
            .unwrap_or(&project.join(".waywr/state"))
            .canonicalize()
            .map_err(err)?;
        let marker = read(&state.join("project-identity.json"))?;
        if marker != json!({"schema_version":1,"project_key":key}) {
            return Err("Source project/state identity mismatch".into());
        }
        if !state.join("state.sqlite").is_file() {
            return Err("Source state database is unavailable".into());
        }
        Ok(Self {
            project,
            state,
            key: key.to_owned(),
        })
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
    fn record(db: &Connection, table: &str, run: &str, request: &str) -> Result<Value, String> {
        if !matches!(table, "app_requests" | "app_responses") {
            return Err("unsupported source table".into());
        }
        let raw: String = db
            .query_row(
                &format!("SELECT payload FROM {table} WHERE run_id=?1 AND request_id=?2"),
                (run, request),
                |r| r.get(0),
            )
            .map_err(|_| format!("Selected source record is missing: {run}/{request}"))?;
        serde_json::from_str(&raw).map_err(err)
    }
    pub fn select(&self, spec: &Value) -> Result<Value, String> {
        let kind = string(spec, "kind")?;
        let run = id(string(spec, "run")?)?;
        let item_id = id(string(spec, "id")?)?;
        if !matches!(kind, "artifact" | "decision" | "finding") {
            return Err(
                "Cross-project import supports one selected artifact, decision or finding".into(),
            );
        }
        let db = self.db()?;
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
        let envelope: Value = serde_json::from_str(&raw).map_err(err)?;
        if envelope["project_key"] != self.key {
            return Err("Source run belongs to another project identity".into());
        }
        let final_request = Self::record(&db, "app_requests", run, "A1")?;
        let approval = Self::record(&db, "app_responses", run, "A1")?;
        if approval["outcome"] != "approved"
            || approval["artifact_sha256"] != final_request["artifact_sha256"]
            || approval["decision_revision"] != final_request["decision_revision"]
        {
            return Err("Source final approval binding is invalid".into());
        }
        let source_root = self
            .state
            .join("runs")
            .join(run)
            .canonicalize()
            .map_err(err)?;
        if !source_root.starts_with(&self.state) {
            return Err("Source run leaves state root".into());
        }
        let (record_hash, version_hash, selected_content) = match kind {
            "artifact" => {
                let reference = &final_request["artifact"];
                if reference["id"] != item_id {
                    return Err("Only the exact approved source artifact may be imported".into());
                }
                let manifest = read(
                    &source_root
                        .join("artifacts")
                        .join(format!("{item_id}.json")),
                )?["manifest"]
                    .clone();
                if manifest != *reference {
                    return Err("Source artifact manifest disagrees with approval".into());
                }
                let path = source_root.join(string(reference, "path")?);
                let text = content(&path, &source_root, string(reference, "sha256")?)?;
                (
                    hash(reference)?,
                    string(reference, "sha256")?.to_owned(),
                    json!(text),
                )
            }
            "decision" => {
                let request = Self::record(&db, "app_requests", run, item_id)?;
                let response = Self::record(&db, "app_responses", run, item_id)?;
                let accepted = final_request["decisions"]
                    .as_array()
                    .ok_or("missing source decisions")?
                    .iter()
                    .any(|d| d["request_id"] == item_id && d["answer"] == response["answer"]);
                if request["kind"] != "choice"
                    || response["outcome"] != "answered"
                    || !accepted
                    || response["artifact_sha256"] != request["artifact_sha256"]
                    || response["decision_revision"] != request["decision_revision"]
                {
                    return Err(
                        "Selected source decision is not accepted in approved context".into(),
                    );
                }
                let record_hash = hash(&json!({"request":request,"response":response}))?;
                (
                    record_hash.clone(),
                    record_hash,
                    json!({"question":request["question"],"response":response}),
                )
            }
            "finding" => {
                let finding = final_request["findings"]
                    .get(item_id)
                    .ok_or("Selected source finding is not open")?;
                if finding["status"] != "open" {
                    return Err("Selected source finding is not open".into());
                }
                let origin = id(string(finding, "origin")?)?;
                let artifact = read(&source_root.join("artifacts").join(format!("{origin}.json")))?;
                let ledger_path = source_root
                    .join("quality")
                    .join(format!("{origin}-finding-ledger.json"));
                let supported = if ledger_path.exists() {
                    let ledger = read(&ledger_path)?;
                    ledger["result_sha256"] == hash(&artifact["result"])?
                        && ledger["new_findings"].as_array().is_some_and(|a| {
                            a.iter()
                                .any(|x| x["id"] == item_id && x["evidence"] == finding["evidence"])
                        })
                } else {
                    artifact["result"]["findings"].as_array().is_some_and(|a| {
                        a.iter()
                            .any(|x| x["id"] == item_id && x["evidence"] == finding["evidence"])
                    })
                };
                if !supported {
                    return Err("Selected finding origin evidence is invalid".into());
                }
                let mut content = finding.as_object().ok_or("invalid finding")?.clone();
                content.insert("id".into(), format!("{}/{run}/{item_id}", self.key).into());
                let record_hash = hash(finding)?;
                (record_hash.clone(), record_hash, Value::Object(content))
            }
            _ => unreachable!(),
        };
        let item = json!({"schema_version":1,"source_project_key":self.key,"source_run":run,
            "kind":kind,"record_id":item_id,
            "qualified_id":format!("{}/{run}/{item_id}",self.key),
            "source_record_sha256":record_hash,"version_sha256":version_hash,
            "approval_sha256":final_request["artifact_sha256"],"content":selected_content});
        if canonical(&item).map_err(err)?.len() > MAX_FOREIGN_BYTES {
            return Err("Selected foreign item exceeds 16 KiB; narrow the request".into());
        }
        Ok(item)
    }
}

pub struct GrantReader {
    pub state: PathBuf,
    pub recipient_key: String,
}
impl GrantReader {
    fn root(&self) -> PathBuf {
        self.state.join("session/context-access")
    }
    fn path(&self, grant_id: &str) -> Result<PathBuf, String> {
        Ok(self
            .root()
            .join("grants")
            .join(format!("{}.json", id(grant_id)?)))
    }
    fn revoked(&self, grant_id: &str) -> Result<bool, String> {
        Ok(self
            .root()
            .join("revocations")
            .join(format!("{}.json", id(grant_id)?))
            .exists())
    }
    pub fn active_for_task(&self, task_id: &str) -> Result<Vec<Value>, String> {
        let mut paths = fs::read_dir(self.root().join("grants"))
            .map(|x| {
                x.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|s| s == "json"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        paths.sort();
        let mut selected = Vec::new();
        for path in paths {
            let grant = read(&path)?;
            if grant["schema_version"] != 1 || grant["recipient_project_key"] != self.recipient_key
            {
                return Err("Unknown or foreign context grant record".into());
            }
            if grant["task_id"] == task_id && !self.revoked(string(&grant, "grant_id")?)? {
                selected.push(grant);
            }
        }
        let size = selected
            .iter()
            .map(|x| canonical(&x["item"]).map(|b| b.len()).map_err(err))
            .sum::<Result<usize, _>>()?;
        if size > MAX_FOREIGN_BYTES {
            return Err("Selected foreign context exceeds 16 KiB across grants".into());
        }
        Ok(selected)
    }
    pub fn check_frozen(&self, imports: &[Value]) -> Result<(), String> {
        for frozen in imports {
            let grant_id = string(frozen, "grant_id")?;
            let grant = read(&self.path(grant_id)?)?;
            let item_hash = string(frozen, "item_sha256")?;
            if grant["recipient_project_key"] != self.recipient_key
                || self.revoked(grant_id)?
                || hash(&grant["item"])? != item_hash
                || hash(&frozen["item"])? != item_hash
            {
                return Err(
                    "Cross-project grant was revoked or changed; dependent dispatch stopped".into(),
                );
            }
            if grant["dependency_mode"] == "must_match_source" {
                let binding = SourceBinding::new(
                    Path::new(string(&grant, "source_project")?),
                    Some(Path::new(string(&grant, "source_state")?)),
                )?;
                let selected = binding.select(&grant["selector"])?;
                if hash(&selected)? != item_hash {
                    return Err("Selected source item changed; dependent dispatch stopped".into());
                }
            }
        }
        Ok(())
    }
}
