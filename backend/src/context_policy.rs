//! Bounded context and question rules. Data access is supplied by a project-scoped reader.
use crate::codec::canonical;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
pub fn inherited(snapshot: &Value) -> Result<Vec<Value>, String> {
    let entries = snapshot["entries"]
        .as_array()
        .ok_or("missing context entries")?;
    let mut out = Vec::new();
    for entry in entries.iter().filter(|e| e["kind"] == "decision") {
        let mut inherited = entry["content"]["response"]
            .as_object()
            .ok_or("invalid inherited response")?
            .clone();
        let mut question = entry["content"]["question"]
            .as_object()
            .ok_or("invalid inherited question")?
            .clone();
        question.insert("topic_key".into(), entry["topic_key"].clone());
        inherited.insert("question".into(), Value::Object(question));
        for key in [
            "qualified_id",
            "source_run",
            "scope",
            "source_record_digest",
        ] {
            inherited.insert(
                key.into(),
                entry
                    .get(key)
                    .ok_or_else(|| format!("missing {key}"))?
                    .clone(),
            );
        }
        out.push(Value::Object(inherited));
    }
    Ok(out)
}
pub fn effective_decisions(state: &Value) -> Result<Vec<Value>, String> {
    let local = state["decisions"].as_array().ok_or("invalid decisions")?;
    let inherited = state["inherited_decisions"]
        .as_array()
        .ok_or("invalid inherited decisions")?;
    let replaced: BTreeSet<&str> = local
        .iter()
        .flat_map(|d| d["supersedes"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    let mut out: Vec<Value> = inherited
        .iter()
        .filter(|d| !replaced.contains(d["qualified_id"].as_str().unwrap_or("")))
        .cloned()
        .collect();
    out.extend(local.iter().cloned());
    Ok(out)
}
pub fn evidence_ids(packet: &Value) -> Result<Vec<String>, String> {
    let mut ids = BTreeSet::from(["brief".to_owned()]);
    for entry in packet["context_snapshot"]["entries"]
        .as_array()
        .ok_or("missing context entries")?
    {
        ids.insert(
            entry["qualified_id"]
                .as_str()
                .ok_or("invalid context ID")?
                .to_owned(),
        );
    }
    let run = packet["run_id"].as_str().ok_or("missing run ID")?;
    if let Some(current) = packet["current_ref"].as_object() {
        ids.insert(format!(
            "{run}/{}",
            current
                .get("id")
                .and_then(Value::as_str)
                .ok_or("invalid current ref")?
        ));
    }
    if let Some(critique) = packet["critique"].as_object() {
        ids.insert(format!(
            "{run}/{}",
            critique["artifact"]["id"]
                .as_str()
                .ok_or("invalid critique ref")?
        ));
    }
    if let Some(findings) = packet
        .get("finding_dispositions")
        .and_then(Value::as_object)
    {
        ids.extend(findings.keys().cloned());
    }
    Ok(ids.into_iter().collect())
}

const FIELDS: [&str; 10] = [
    "id",
    "question",
    "options",
    "recommendation",
    "consequences",
    "topic_key",
    "rationale",
    "evidence_refs",
    "related_decision_refs",
    "reconsideration_reason",
];

pub fn validate_questions(result: &Value, packet: &Value) -> Result<(), String> {
    let questions = result["questions"]
        .as_array()
        .ok_or("questions must be an array")?;
    if questions.len() > 4 {
        return Err("Question output exceeds bounded contract".into());
    }
    let decisions = packet["decisions"].as_array().ok_or("missing decisions")?;
    let run = packet["run_id"].as_str().ok_or("missing run ID")?;
    let known_refs: BTreeSet<String> = decisions
        .iter()
        .map(|d| {
            d["qualified_id"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| d["request_id"].as_str().map(|r| format!("{run}/{r}")))
                .ok_or("invalid decision reference")
        })
        .collect::<Result<_, _>>()?;
    let evidence: BTreeSet<String> = evidence_ids(packet)?.into_iter().collect();
    let mut topics = BTreeSet::new();
    for q in questions {
        let map = q.as_object().ok_or("invalid question")?;
        if map.len() != FIELDS.len() || !FIELDS.iter().all(|key| map.contains_key(*key)) {
            return Err("Invalid version-2 question fields".into());
        }
        for field in FIELDS
            .iter()
            .filter(|f| !matches!(**f, "options" | "evidence_refs" | "related_decision_refs"))
        {
            let value = q[*field].as_str().ok_or("Invalid question text/size")?;
            if value.chars().count() > 2000
                || (value.trim().is_empty() && *field != "reconsideration_reason")
            {
                return Err("Invalid question text/size".into());
            }
        }
        if !id(q["id"].as_str().ok_or("invalid question ID")?) {
            return Err("Use a short alphanumeric ID with hyphens/underscores".into());
        }
        if !topics.insert(q["topic_key"].as_str().unwrap_or("")) {
            return Err("Duplicate question topic in result".into());
        }
        let options = q["options"]
            .as_array()
            .ok_or("Question needs 2–3 distinct options")?;
        let distinct: BTreeSet<_> = options.iter().filter_map(Value::as_str).collect();
        if !(2..=3).contains(&options.len())
            || distinct.len() != options.len()
            || options.iter().any(|x| {
                x.as_str()
                    .is_none_or(|s| s.trim().is_empty() || s.chars().count() > 200)
            })
        {
            return Err("Question needs 2–3 distinct options".into());
        }
        for (field, allowed) in [
            ("evidence_refs", &evidence),
            ("related_decision_refs", &known_refs),
        ] {
            let refs = q[field]
                .as_array()
                .ok_or("Question refers to unknown evidence/decision")?;
            if refs.len() > 12
                || refs
                    .iter()
                    .any(|x| x.as_str().is_none_or(|s| !allowed.contains(s)))
            {
                return Err("Question refers to unknown evidence/decision".into());
            }
        }
        if q["evidence_refs"].as_array().is_none_or(Vec::is_empty) {
            return Err("Question requires evidence".into());
        }
        if !q["reconsideration_reason"]
            .as_str()
            .unwrap_or("")
            .is_empty()
            != !q["related_decision_refs"]
                .as_array()
                .is_none_or(Vec::is_empty)
        {
            return Err("Reconsideration requires prior decision references and a reason".into());
        }
    }
    Ok(())
}

/// Prove a frozen selected snapshot has a stable byte-sized contract. Source
/// selection/revalidation is a separate read-only, project-scoped boundary.
pub fn validate_snapshot(snapshot: &Value) -> Result<(), String> {
    if !matches!(snapshot["schema_version"].as_u64(), Some(1 | 2))
        || snapshot["scope"].as_str().is_none_or(str::is_empty)
        || snapshot["entries"].as_array().is_none()
    {
        return Err("Unsupported or incomplete context snapshot".into());
    }
    let bytes = canonical(snapshot).map_err(|e| e.to_string())?;
    if bytes.len() > 48_000 {
        return Err("Selected context exceeds 48 KB; narrow explicit selection".into());
    }
    let mut seen = BTreeSet::new();
    for entry in snapshot["entries"].as_array().unwrap() {
        let id = entry["qualified_id"]
            .as_str()
            .ok_or("missing qualified context ID")?;
        if !seen.insert(id) {
            return Err("Duplicate context reference".into());
        }
        if entry["scope"] != snapshot["scope"] {
            return Err("Context entry scope mismatch".into());
        }
    }
    Ok(())
}

pub fn frozen_import_packet(imports: &[Value]) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for entry in imports {
        out.push(
            json!({"source_project_key":entry["item"]["source_project_key"],
            "grant_id":entry["grant_id"],"purpose":entry["purpose"],"item":entry["item"]}),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inherited_decision_stays_until_explicit_supersession() {
        let inherited = json!({"qualified_id":"source/D","answer":"keep","supersedes":[]});
        let state = json!({"inherited_decisions":[inherited],"decisions":[]});
        assert_eq!(effective_decisions(&state).unwrap().len(), 1);
        let state = json!({"inherited_decisions":[inherited],"decisions":[{"answer":"new","supersedes":["source/D"]}]});
        assert_eq!(
            effective_decisions(&state).unwrap(),
            vec![state["decisions"][0].clone()]
        );
    }
}
