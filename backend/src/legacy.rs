//! Read-only, byte-preserving inspection of application-owned Python records.
//! LangGraph checkpoint blobs are deliberately opaque.

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Unsupported(&'static str),
    Integrity(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Sql(e) => write!(f, "SQLite: {e}"),
            Self::Json(e) => write!(f, "JSON: {e}"),
            Self::Unsupported(s) => write!(f, "unsupported legacy record: {s}"),
            Self::Integrity(s) => write!(f, "integrity refusal: {s}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<rusqlite::Error> for Error { fn from(e: rusqlite::Error) -> Self { Self::Sql(e) } }
impl From<serde_json::Error> for Error { fn from(e: serde_json::Error) -> Self { Self::Json(e) } }

pub fn sha256(bytes: &[u8]) -> String { crate::codec::sha256(bytes) }

#[derive(Debug, Clone, Serialize)]
pub struct RawRecord {
    pub id: String,
    pub raw: String,
    pub raw_sha256: String,
    pub value: Value,
}

fn raw_record(id: String, raw: String) -> Result<RawRecord, Error> {
    let value = serde_json::from_str(&raw)?;
    Ok(RawRecord { id, raw_sha256: sha256(raw.as_bytes()), raw, value })
}

#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub manifest_raw_sha256: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingLedger {
    pub stage: String,
    pub raw_sha256: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub stage: String,
    pub operation_id: String,
    pub packet_hash: String,
}

#[derive(Debug, Serialize)]
pub struct RunInspection {
    pub run_id: String,
    pub status: String,
    pub envelope: RawRecord,
    pub requests: Vec<RawRecord>,
    pub responses: Vec<RawRecord>,
    pub operations: Vec<Operation>,
    pub artifacts: Vec<Artifact>,
    pub finding_ledgers: Vec<FindingLedger>,
    pub graph_continuation: &'static str,
}

pub struct LegacyReader {
    root: PathBuf,
    db: Connection,
}

impl LegacyReader {
    /// `state_root` is an explicit archived/copied state directory, never a
    /// location-registry lookup. SQLite is opened read-only and query-only.
    pub fn open(state_root: &Path) -> Result<Self, Error> {
        let root = state_root.canonicalize()?;
        let database = root.join("state.sqlite");
        if !database.is_file() || !database.canonicalize()?.starts_with(&root) {
            return Err(Error::Integrity("state database leaves selected root".into()));
        }
        let db = Connection::open_with_flags(
            database,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        db.pragma_update(None, "query_only", "ON")?;
        for table in ["app_runs", "app_operations", "app_requests", "app_responses"] {
            let exists: i64 = db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table], |row| row.get(0),
            )?;
            if exists != 1 { return Err(Error::Unsupported("required application table absent")); }
        }
        let upgraded: i64 = db.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='ww_schema_migrations'",
            [], |row| row.get(0),
        )?;
        if upgraded != 0 { return Err(Error::Unsupported("Rust-upgraded schema")); }
        Ok(Self { root, db })
    }

    fn contained(&self, run_id: &str, relative: &str) -> Result<PathBuf, Error> {
        if relative.is_empty() || Path::new(relative).is_absolute() ||
            Path::new(relative).components().any(|part| !matches!(part, Component::Normal(_)))
        { return Err(Error::Integrity("artifact path is not a normal relative path".into())); }
        let run = self.root.join("runs").join(run_id).canonicalize()?;
        if !run.starts_with(&self.root) { return Err(Error::Integrity("run path leaves state root".into())); }
        let path = run.join(relative).canonicalize()?;
        if !path.starts_with(&run) { return Err(Error::Integrity("artifact path leaves run root".into())); }
        Ok(path)
    }

    pub fn inspect(&self, run_id: &str) -> Result<RunInspection, Error> {
        if run_id.is_empty() || run_id.len() > 64 ||
            !run_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        { return Err(Error::Integrity("invalid run ID".into())); }
        let (envelope_raw, status): (String, String) = self.db.query_row(
            "SELECT envelope,status FROM app_runs WHERE run_id=?1", [run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let envelope = raw_record(run_id.into(), envelope_raw)?;
        if envelope.value.get("run_id").and_then(Value::as_str) != Some(run_id) {
            return Err(Error::Integrity("envelope run ID differs from row".into()));
        }
        if envelope.value.get("protocol_version").and_then(Value::as_u64).unwrap_or(1) != 1 {
            return Err(Error::Unsupported("run protocol version"));
        }
        let requests = self.json_rows("SELECT request_id,payload FROM app_requests WHERE run_id=?1 ORDER BY rowid", run_id)?;
        let responses = self.json_rows("SELECT request_id,payload FROM app_responses WHERE run_id=?1 ORDER BY rowid", run_id)?;
        let mut operations = Vec::new();
        let mut stmt = self.db.prepare("SELECT stage,operation_id,packet_hash FROM app_operations WHERE run_id=?1 ORDER BY rowid")?;
        let rows = stmt.query_map([run_id], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?)))?;
        for row in rows {
            let (stage, operation_id, packet_hash) = row?;
            operations.push(Operation { stage, operation_id, packet_hash });
        }
        for response in &responses {
            let request = requests.iter().find(|candidate| candidate.id == response.id)
                .ok_or_else(|| Error::Integrity(format!("answer lacks request: {}", response.id)))?;
            for field in ["artifact_sha256", "decision_revision", "context_digest"] {
                if request.value.get(field) != response.value.get(field) {
                    return Err(Error::Integrity(format!("decision binding differs for {}: {field}", response.id)));
                }
            }
            if response.value.get("schema_version").and_then(Value::as_u64) != Some(1) {
                return Err(Error::Unsupported("decision record version"));
            }
            let export = self.contained(run_id, &format!("artifacts/decision-{}.json", response.id))?;
            let raw = fs::read(export)?;
            if raw != response.raw.as_bytes() { return Err(Error::Integrity(format!("decision export differs: {}", response.id))); }
            if response.value.get("request_id").and_then(Value::as_str) != Some(&response.id) {
                return Err(Error::Integrity(format!("decision ID differs: {}", response.id)));
            }
        }
        for request in &requests {
            if request.value.get("request_id").and_then(Value::as_str) != Some(&request.id) {
                return Err(Error::Integrity(format!("request ID differs: {}", request.id)));
            }
        }
        let artifact_dir = self.root.join("runs").join(run_id).join("artifacts");
        let mut artifacts = Vec::new();
        if artifact_dir.is_dir() {
            for entry in fs::read_dir(&artifact_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".json") || name.starts_with("decision-") { continue; }
                let raw = fs::read(self.contained(run_id, &format!("artifacts/{name}"))?)?;
                let item: Value = serde_json::from_slice(&raw)?;
                let manifest = item.get("manifest").ok_or(Error::Unsupported("artifact manifest missing"))?;
                if manifest.get("schema_version").and_then(Value::as_u64) != Some(1) {
                    return Err(Error::Unsupported("artifact manifest version"));
                }
                let id = manifest.get("id").and_then(Value::as_str).ok_or(Error::Unsupported("artifact ID missing"))?;
                if name != format!("{id}.json") { return Err(Error::Integrity("artifact manifest ID differs from filename".into())); }
                let path = manifest.get("path").and_then(Value::as_str).ok_or(Error::Unsupported("artifact path missing"))?;
                let expected = manifest.get("sha256").and_then(Value::as_str).ok_or(Error::Unsupported("artifact hash missing"))?;
                let actual = sha256(&fs::read(self.contained(run_id, path)?)?);
                if expected != actual { return Err(Error::Integrity(format!("artifact hash mismatch: {id}"))); }
                let parents = manifest.get("parents").and_then(Value::as_array)
                    .ok_or(Error::Unsupported("artifact parents missing"))?
                    .iter().map(|v| v.as_str().map(str::to_owned).ok_or(Error::Unsupported("non-string parent")))
                    .collect::<Result<Vec<_>,_>>()?;
                artifacts.push(Artifact { id:id.into(), path:path.into(), sha256:actual,
                    manifest_raw_sha256:sha256(&raw), parents });
            }
        }
        artifacts.sort_by(|a,b| a.id.cmp(&b.id));
        for artifact in &artifacts {
            for parent in &artifact.parents {
                if !artifacts.iter().any(|candidate| candidate.id == *parent) {
                    return Err(Error::Integrity(format!("unknown artifact parent {parent}")));
                }
            }
        }
        for response in &responses {
            let digest = response.value.get("artifact_sha256").and_then(Value::as_str)
                .ok_or(Error::Unsupported("decision artifact hash missing"))?;
            if !artifacts.iter().any(|a| a.sha256 == digest) {
                return Err(Error::Integrity(format!("decision references missing artifact: {}", response.id)));
            }
        }
        let quality_dir = self.root.join("runs").join(run_id).join("quality");
        let mut finding_ledgers = Vec::new();
        if quality_dir.is_dir() {
            for entry in fs::read_dir(&quality_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with("-finding-ledger.json") { continue; }
                let raw = fs::read(self.contained(run_id, &format!("quality/{name}"))?)?;
                let value: Value = serde_json::from_slice(&raw)?;
                let stage = name.trim_end_matches("-finding-ledger.json");
                if value.get("schema_version").and_then(Value::as_u64) != Some(1) ||
                    value.get("stage").and_then(Value::as_str) != Some(stage) ||
                    !artifacts.iter().any(|a| a.id == stage)
                { return Err(Error::Integrity(format!("finding ledger provenance invalid: {stage}"))); }
                finding_ledgers.push(FindingLedger { stage:stage.into(), raw_sha256:sha256(&raw), value });
            }
        }
        finding_ledgers.sort_by(|a,b| a.stage.cmp(&b.stage));
        Ok(RunInspection { run_id:run_id.into(), status, envelope, requests,
            responses, operations, artifacts, finding_ledgers,
            graph_continuation:"opaque: legacy release required for unfinished graph" })
    }

    fn json_rows(&self, sql: &str, run_id: &str) -> Result<Vec<RawRecord>, Error> {
        let mut stmt = self.db.prepare(sql)?;
        let rows = stmt.query_map([run_id], |row| Ok((row.get::<_, String>(0)?,row.get::<_, String>(1)?)))?;
        let mut values = Vec::new();
        for row in rows { let (id,raw) = row?; values.push(raw_record(id,raw)?); }
        Ok(values)
    }
}
