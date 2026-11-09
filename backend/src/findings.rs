//! Pure, versioned finding interpretation. No worker output grants approval.
use crate::codec::{canonical_sha256, sha256};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("invalid {key}"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("invalid {key}"))
}
fn object(value: &Value) -> Result<&Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| "findings must be an object".into())
}
fn digest(value: &Value) -> Result<String, String> {
    canonical_sha256(value).map_err(|e| e.to_string())
}
pub fn durable_id(operation_id: &str, index: usize) -> String {
    format!(
        "F-{}",
        &sha256(format!("{operation_id}:{index}").as_bytes())[..16]
    )
}

/// Classify saved output. Legacy reused IDs require a separately authenticated
/// interpretation; the result itself is never modified to make it acceptable.
pub fn classify(
    stage: &str,
    prior: &Value,
    packet: &Value,
    result: &Value,
    interpretation: Option<&Value>,
) -> Result<Value, String> {
    let prior = object(prior)?;
    let operation_id = string(packet, "operation_id")?;
    let version = packet
        .get("finding_contract_version")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let mut new = Vec::new();
    let mut assessments = Vec::new();
    let mut seen_new = BTreeSet::new();
    let mut seen_refs = BTreeSet::new();
    if version == 2 {
        if interpretation.is_some() {
            return Err("Finding interpretation applies only to a saved legacy result".into());
        }
        for (index, item) in array(result, "new_findings")?.iter().enumerate() {
            let id = durable_id(operation_id, index);
            if prior.contains_key(&id) || !seen_new.insert(id.clone()) {
                return Err(format!("Application finding identity collision {id}"));
            }
            let mut value = object(item)?.clone();
            value.insert("id".into(), id.into());
            value.insert("source_index".into(), index.into());
            new.push(Value::Object(value));
        }
        for (index, item) in array(result, "assessments")?.iter().enumerate() {
            let reference = string(item, "finding_ref")?;
            if !prior.contains_key(reference) {
                return Err(format!("Assessment refers to unknown finding {reference}"));
            }
            if !seen_refs.insert(reference.to_owned()) {
                return Err(format!("Conflicting assessments for finding {reference}"));
            }
            assessments.push(
                json!({"finding_ref":reference,"assessment":field(item,"assessment")?,
                "evidence":field(item,"evidence")?,"rationale":field(item,"rationale")?,
                "source_field":"assessments","source_index":index}),
            );
        }
    } else if version == 1 {
        let mut mappings = BTreeMap::new();
        if let Some(interpretation) = interpretation {
            for entry in array(interpretation, "entries")? {
                let index = field(entry, "source_index")?
                    .as_u64()
                    .ok_or("invalid source_index")?;
                if mappings.insert(index as usize, entry).is_some() {
                    return Err("Conflicting interpretation entries".into());
                }
            }
        }
        for (index, item) in array(result, "findings")?.iter().enumerate() {
            let id = string(item, "id")?;
            if prior.contains_key(id) {
                let mapping = mappings
                    .get(&index)
                    .ok_or_else(|| format!("Ambiguous reused finding ID {id} in {stage}"))?;
                if string(mapping, "finding_ref")? != id
                    || !matches!(
                        string(mapping, "assessment")?,
                        "open" | "reverified-addressed"
                    )
                {
                    return Err(format!("Ambiguous reused finding ID {id} in {stage}"));
                }
                if digest(item)? != string(mapping, "source_item_sha256")? {
                    return Err(format!("Saved assessment text changed for {id}"));
                }
                if !seen_refs.insert(id.to_owned()) {
                    return Err(format!("Conflicting assessments for finding {id}"));
                }
                assessments.push(
                    json!({"finding_ref":id,"assessment":field(mapping,"assessment")?,
                    "evidence":field(item,"evidence")?,"rationale":field(item,"requested_change")?,
                    "source_field":"findings","source_index":index}),
                );
            } else {
                if !seen_new.insert(id.to_owned()) || mappings.contains_key(&index) {
                    return Err(format!("Ambiguous new finding identity {id}"));
                }
                let mut value = object(item)?.clone();
                value.insert("source_index".into(), index.into());
                new.push(Value::Object(value));
            }
        }
        let mapped: BTreeSet<_> = assessments
            .iter()
            .filter(|a| a["source_field"] == "findings")
            .filter_map(|a| a["source_index"].as_u64())
            .map(|x| x as usize)
            .collect();
        if mappings.keys().copied().collect::<BTreeSet<_>>() != mapped {
            return Err("Interpretation contains unmatched finding entries".into());
        }
        for (index, item) in array(result, "dispositions")?.iter().enumerate() {
            let id = string(item, "id")?;
            if !prior.contains_key(id) && !seen_new.contains(id) {
                return Err(format!("Unknown finding disposition {id}"));
            }
            if !seen_refs.insert(id.to_owned()) {
                return Err(format!("Conflicting assessments for finding {id}"));
            }
            assessments.push(json!({"finding_ref":id,"assessment":"addressed",
                "evidence":field(item,"rationale")?,"rationale":field(item,"rationale")?,
                "source_field":"dispositions","source_index":index}));
        }
    } else {
        return Err("Unsupported finding contract version".into());
    }

    let output_hash = digest(result)?;
    let packet_hash = digest(packet)?;
    for item in &mut assessments {
        let reference = string(item, "finding_ref")?.to_owned();
        let original = prior.get(&reference);
        let original_hash =
            original.map(|value| sha256(string(value, "evidence").unwrap_or("").as_bytes()));
        let map = item.as_object_mut().ok_or("invalid assessment")?;
        map.insert("stage".into(), stage.into());
        map.insert("operation_id".into(), operation_id.into());
        map.insert("packet_sha256".into(), packet_hash.clone().into());
        map.insert("result_sha256".into(), output_hash.clone().into());
        map.insert(
            "original_evidence_sha256".into(),
            original_hash.map(Value::String).unwrap_or(Value::Null),
        );
    }
    let covered: BTreeSet<_> = assessments
        .iter()
        .filter_map(|a| a["finding_ref"].as_str())
        .collect();
    let missing = if stage.starts_with("revision") {
        prior
            .iter()
            .filter(|(id, value)| value["status"] == "open" && !covered.contains(id.as_str()))
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    Ok(
        json!({"new":new,"assessments":assessments,"missing":missing,
        "protocol_version":version,"result_sha256":output_hash}),
    )
}

pub fn apply(
    prior: &Value,
    classified: &Value,
    ref_id: &str,
    stage: &str,
    reviewed: &Value,
) -> Result<Value, String> {
    let mut findings = object(prior)?.clone();
    for item in array(classified, "new")? {
        let id = string(item, "id")?;
        if findings.contains_key(id) {
            return Err(format!("finding ID collision {id}"));
        }
        let mut value = object(item)?.clone();
        value.remove("source_index");
        value.insert("status".into(), "open".into());
        value.insert("origin".into(), ref_id.into());
        value.insert("reviewed".into(), reviewed.clone());
        findings.insert(id.to_owned(), Value::Object(value));
    }
    for item in array(classified, "assessments")? {
        if classified["protocol_version"] == 1
            && item["source_field"] == "dispositions"
            && stage.starts_with("revision")
        {
            let id = string(item, "finding_ref")?;
            let value = findings
                .get_mut(id)
                .ok_or_else(|| format!("unknown finding {id}"))?;
            value["status"] = "addressed".into();
            value["rationale"] = field(item, "rationale")?.clone();
        }
    }
    Ok(Value::Object(findings))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_ids_are_stable_and_worker_claims_do_not_close_issues() {
        let prior = json!({"C1-2":{"id":"C1-2","status":"open","evidence":"original"}});
        let packet = json!({"operation_id":"op","finding_contract_version":2});
        let result = json!({"new_findings":[{"evidence":"new","severity":"minor","requested_change":"fix"}],
            "assessments":[{"finding_ref":"C1-2","assessment":"proposed-addressed","evidence":"candidate","rationale":"claim"}]});
        let classified = classify("revision-2", &prior, &packet, &result, None).unwrap();
        let updated = apply(
            &prior,
            &classified,
            "revision-2",
            "revision-2",
            &Value::Null,
        )
        .unwrap();
        assert_eq!(updated["C1-2"]["status"], "open");
        assert_eq!(updated["C1-2"]["evidence"], "original");
        assert!(updated.get(durable_id("op", 0)).is_some());
    }
}
