//! Explicit review lineage. Native results stay immutable; effective disposition
//! is an application projection over validated, exact-revision reviewer judgments.
use crate::development_recovery::hash;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disposition {
    pub finding_id: String,
    pub origin_sha256: String,
    pub target_commit: String,
    pub requirement_generation: u64,
    pub disposition: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
}
pub fn schema() -> Value {
    json!({"type":"array","maxItems":128,"items":{"type":"object","additionalProperties":false,
        "properties":{"finding_id":{"type":"string"},"origin_sha256":{"type":"string"},"target_commit":{"type":"string"},"requirement_generation":{"type":"integer"},"disposition":{"type":"string","enum":["blocking","resolved","advisory"]},"rationale":{"type":"string"},"evidence_refs":{"type":"array","items":{"type":"string"}}},
        "required":["finding_id","origin_sha256","target_commit","requirement_generation","disposition","rationale","evidence_refs"]}})
}
fn targets(packet: &Value) -> Vec<Value> {
    packet["review_targets"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|t| t["result_commit"].clone())
        .collect()
}
fn finding(
    unit: &str,
    operation: &Value,
    origin: Value,
    commit: &Value,
    generation: &Value,
    profile: &Value,
    text: &str,
    state: &str,
) -> Result<Value, String> {
    let identity = json!([unit, operation, origin, commit, generation, text]);
    Ok(
        json!({"finding_id":hash(&identity)?,"review_unit_id":unit,"origin_operation":operation,"origin":origin,
        "target_commit":commit,"requirement_generation":generation,"profile_sha256":profile,"text":text,"disposition":state}),
    )
}
/// Old strings get deterministic identities only in newly prepared context. No
/// historical packet, accepted work or original reviewer result is rewritten.
pub fn prior(unit: &Value, packet: &Value) -> Result<Vec<Value>, String> {
    if let Some(ledger) = unit["finding_ledger"].as_array() {
        return Ok(ledger.clone());
    }
    let mut out = vec![];
    let unit_id = unit["unit_id"].as_str().unwrap_or("");
    let profile = if unit["selection"]["profile_sha256"].is_string() {
        unit["selection"]["profile_sha256"].clone()
    } else {
        Value::Null
    };
    let commits = if let Some(c) = unit["review"]["reviewed_commits"]
        .as_array()
        .filter(|a| !a.is_empty())
    {
        c.clone()
    } else {
        targets(packet)
    };
    let generation = unit
        .get("requirement_generation")
        .unwrap_or(&packet["requirement_generation"]);
    for text in unit["evidence_context"]["unresolved_findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        for commit in &commits {
            out.push(finding(unit_id,&unit["evidence_context"]["source_operation"],json!({"kind":"legacy_context_snapshot","sha256":hash(&json!([unit["evidence_context"]["sha256"],text]))?}),commit,generation,&profile,text,"blocking")?);
        }
    }
    for text in unit["review"]["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        for commit in &commits {
            if out
                .iter()
                .any(|f| f["text"] == text && f["target_commit"] == *commit)
            {
                continue;
            }
            out.push(finding(
                unit_id,
                &unit["operation_id"],
                json!({"kind":"legacy_review_projection","sha256":hash(&unit["review"])?}),
                commit,
                generation,
                &profile,
                text,
                if unit["review"]["requires_changes"] == true {
                    "blocking"
                } else {
                    "advisory"
                },
            )?);
        }
    }
    Ok(out)
}
pub fn prepare(packet: &mut Value, unit: &Value, all: &[Value]) -> Result<(), String> {
    let mut ledger = prior(unit, packet)?;
    let names = unit["resolution_target_names"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for name in &names {
        let old = all
            .iter()
            .find(|u| u["name"] == *name && u["role"] == "review" && u["status"] == "reviewed")
            .ok_or("Clarification must name a settled review")?;
        if old["work_id"] != unit["work_id"]
            || old["requirement_generation"] != packet["requirement_generation"]
        {
            return Err("Clarification review belongs to another request/generation".into());
        }
        for f in prior(old, packet)? {
            if !targets(packet).contains(&f["target_commit"]) {
                return Err("Clarification target commit differs from its finding".into());
            }
            if !ledger.iter().any(|v| v["finding_id"] == f["finding_id"]) {
                ledger.push(f)
            }
        }
    }
    packet["finding_contract"] = json!(1);
    packet["prior_findings"] = json!(ledger);
    packet["review_resolution_only"] = json!(!names.is_empty());
    Ok(())
}
fn supplied_for_commit(packet: &Value, citation: &str, commit: &str) -> bool {
    let parts = citation.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "evidence" {
        return false;
    }
    let (Ok(start), Ok(end)) = (parts[2].parse::<u64>(), parts[3].parse::<u64>()) else {
        return false;
    };
    if start > end {
        return false;
    }
    let granted = packet["evidence_index"]["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|e| e["commit"] == commit && (e["sha256"] == parts[1] || e["kind"] == "source"));
    granted
        && packet["evidence_pages"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| {
                p["error"].is_null()
                    && p["sha256"] == parts[1]
                    && p["provenance"]["commit"] == commit
                    && p["offset"].as_u64().is_some_and(|a| a <= start)
                    && p["end"].as_u64().is_some_and(|b| b >= end)
            })
        || packet["evidence_receipts"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| {
                // Receipts only carry hash/ranges; the immutable grant or active source
                // page must independently bind those bytes to the same exact commit.
                let exact = packet["evidence_index"]["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|e| e["sha256"] == parts[1] && e["commit"] == commit)
                    || packet["evidence_pages"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|e| e["sha256"] == parts[1] && e["provenance"]["commit"] == commit);
                exact
                    && p["sha256"] == parts[1]
                    && p["ranges"].as_array().into_iter().flatten().any(|r| {
                        r[0].as_u64().is_some_and(|a| a <= start)
                            && r[1].as_u64().is_some_and(|b| b >= end)
                    })
            })
}
pub fn validate(result: &Value, packet: &Value) -> Result<Vec<Disposition>, String> {
    let dispositions: Vec<Disposition> =
        serde_json::from_value(result["finding_dispositions"].clone())
            .map_err(|e| e.to_string())?;
    if dispositions.len() > 128 {
        return Err("Too many finding dispositions".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for d in &dispositions {
        let old = packet["prior_findings"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|f| f["finding_id"] == d.finding_id)
            .ok_or("Resolution does not name a supplied prior finding")?;
        if !seen.insert(&d.finding_id)
            || !matches!(d.disposition.as_str(), "blocking" | "resolved" | "advisory")
            || d.rationale.trim().is_empty()
            || d.rationale.len() > 8000
            || d.evidence_refs.len() > 32
            || old["origin"]["sha256"] != d.origin_sha256
            || old["target_commit"] != d.target_commit
            || old["requirement_generation"] != d.requirement_generation
            || packet["requirement_generation"] != d.requirement_generation
            || old["profile_sha256"] != packet["execution"]["selection"]["profile_sha256"]
            || old["profile_sha256"].is_null()
            || !targets(packet).contains(&json!(d.target_commit))
        {
            return Err(
                "Finding resolution lineage, revision, generation or selected reviewer differs"
                    .into(),
            );
        }
        if d.disposition != "blocking"
            && (d.evidence_refs.is_empty()
                || d.evidence_refs
                    .iter()
                    .any(|r| !supplied_for_commit(packet, r, &d.target_commit)))
        {
            return Err("Finding resolution requires supplied exact-revision evidence".into());
        }
    }
    if packet["review_resolution_only"] == true
        && dispositions.is_empty()
        && result["evidence_requests"]
            .as_array()
            .is_none_or(Vec::is_empty)
    {
        return Err("Clarification must explicitly dispose of a linked finding; it is not a replacement whole review".into());
    }
    Ok(dispositions)
}
pub fn apply(result: &Value, packet: &Value) -> Result<Value, String> {
    let dispositions = validate(result, packet)?;
    let digest = hash(result)?;
    let mut ledger = packet["prior_findings"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for d in &dispositions {
        let f = ledger
            .iter_mut()
            .find(|f| f["finding_id"] == d.finding_id)
            .unwrap();
        f["disposition"] = json!(d.disposition);
        f["resolution"] =
            json!({"operation_id":packet["operation_id"],"result_sha256":digest,"judgment":d});
    }
    let mut texts = result["findings"].as_array().cloned().unwrap_or_default();
    if result["requires_changes"] == true && texts.is_empty() {
        texts.push(json!(
            "Reviewer requires changes without enumerated findings"
        ));
    }
    for text in texts.iter().filter_map(Value::as_str) {
        for commit in targets(packet) {
            ledger.push(finding(
                packet["assignment"]["unit_id"].as_str().unwrap_or(""),
                &packet["operation_id"],
                json!({"kind":"native_result","sha256":digest}),
                &commit,
                &packet["requirement_generation"],
                &packet["execution"]["selection"]["profile_sha256"],
                text,
                if result["requires_changes"] == true {
                    "blocking"
                } else {
                    "advisory"
                },
            )?);
        }
    }
    let blocking = ledger
        .iter()
        .filter(|f| f["disposition"] == "blocking")
        .map(|f| f["text"].clone())
        .collect::<Vec<_>>();
    let mut effective = result.clone();
    effective["requires_changes"] = json!(!blocking.is_empty());
    for text in blocking {
        if !effective["findings"].as_array().unwrap().contains(&text) {
            effective["findings"].as_array_mut().unwrap().push(text)
        }
    }
    Ok(
        json!({"ledger":ledger,"effective":effective,"native_result_sha256":digest,"validated_dispositions":dispositions}),
    )
}
/// Clarification overlays are read-only. Earlier reviews/events and accepted work
/// are not edited; only this still-current acceptance decision consumes the links.
pub fn effective_blocking_findings(review: &Value, all: &[Value]) -> Vec<Value> {
    let derived;
    let ledger = if let Some(ledger) = review["finding_ledger"].as_array() {
        ledger
    } else {
        let packet = json!({"requirement_generation":review["requirement_generation"],"execution":{"selection":review["selection"]},"review_targets":review["review"]["reviewed_commits"].as_array().map(|a|a.iter().map(|c|json!({"result_commit":c})).collect::<Vec<_>>())});
        derived = prior(review, &packet).unwrap_or_default();
        if derived.is_empty() {
            return vec![];
        }
        &derived
    };
    ledger
        .iter()
        .filter(|f| f["disposition"] == "blocking")
        .filter(|f| {
            !all.iter().any(|r| {
                r["role"] == "review"
                    && r["status"] == "reviewed"
                    && r["review_stale"] != true
                    && r["requirement_generation"] == review["requirement_generation"]
                    && r["work_id"] == review["work_id"]
                    && r["selection"]["profile_sha256"] == f["profile_sha256"]
                    && r["finding_ledger"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|resolved| {
                            resolved["finding_id"] == f["finding_id"]
                                && matches!(
                                    resolved["disposition"].as_str(),
                                    Some("resolved" | "advisory")
                                )
                                && resolved["resolution"]["judgment"]["origin_sha256"]
                                    == f["origin"]["sha256"]
                        })
            })
        })
        .cloned().collect()
}
pub fn effective_requires_changes(review:&Value,all:&[Value])->bool{
    if review["finding_ledger"].as_array().is_none() {
        let packet=json!({"requirement_generation":review["requirement_generation"],"execution":{"selection":review["selection"]},"review_targets":review["review"]["reviewed_commits"].as_array().map(|a|a.iter().map(|c|json!({"result_commit":c})).collect::<Vec<_>>())});
        if prior(review,&packet).unwrap_or_default().is_empty(){return review["review"]["requires_changes"]!=false;}
    }
    !effective_blocking_findings(review,all).is_empty()
}
pub fn require_clear(unit: &Value, all: &[Value]) -> Result<(), String> {
    // Old retained acceptance keeps its semantics until a new finding contract
    // participates. A fresh unrelated pass is never a disposition of prior work.
    if !all.iter().any(|r| r["finding_contract"] == 1) {
        return Ok(());
    }
    let blocked = all
        .iter()
        .filter(|r| {
            r["role"] == "review"
                && r["status"] == "reviewed"
                && r["review_stale"] != true
                && r["work_id"] == unit["work_id"]
                && r["requirement_generation"] == unit["requirement_generation"]
        })
        .any(|r| {
            (r["review"]["reviewed_commits"]
                .as_array()
                .is_some_and(|a| a.contains(&unit["result_commit"]))
                || r["finding_ledger"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|f| f["target_commit"] == unit["result_commit"]))
                && effective_requires_changes(r, all)
        });
    if blocked {
        Err("Unresolved linked review finding prevents acceptance; request an exact reviewer disposition or repair".into())
    } else {
        Ok(())
    }
}
pub const INSTRUCTION: &str = "Finding lineage v1: prior_findings contains stable finding_id, origin.sha256, target_commit, requirement_generation and selected reviewer. Return finding_dispositions, normally []. Each judgment names those exact fields, disposition blocking/resolved/advisory, rationale and evidence_refs. Resolved/advisory judgments require exact supplied evidence:<sha>:<start>:<end> pages for that target commit. Missing dispositions preserve blocking findings even if requires_changes is false. Preserve original wording; do not silently replace findings. A review_resolution_only assignment is a linked clarification, not a new whole review: reviewed_commits stays []; retrieve only necessary exact evidence, then return explicit judgments. Native output remains separate from the application's effective disposition.";

#[cfg(test)]
mod tests {
    use super::*;
    const COMMIT: &str = "02823012af461d75c48689da026935bf0c284b5a";
    fn packet() -> Value {
        let mut p = json!({"operation_kind":"development-review","operation_id":"review-continuation","finding_contract":1,"requirement_generation":3,"assignment":{"unit_id":"cli-review"},"execution":{"selection":{"profile_sha256":"claude-profile"}},"review_targets":[{"result_commit":COMMIT}],"evidence_index":{"version":1,"entries":[{"id":format!("source:{COMMIT}"),"kind":"source","commit":COMMIT}]},"evidence_pages":[{"sha256":"source-hash","offset":0,"end":6029,"total_bytes":6029,"complete":true,"provenance":{"kind":"source","commit":COMMIT}}]});
        let old = finding(
            "cli-review",
            &json!("review-read"),
            json!({"kind":"native_result","sha256":"earlier-review-hash"}),
            &json!(COMMIT),
            &json!(3),
            &json!("claude-profile"),
            "Provisional concern: verify option-like tag handling against exact CLI source",
            "blocking",
        )
        .unwrap();
        p["prior_findings"] = json!([old]);
        p
    }
    fn result(p: &Value) -> Value {
        let f = &p["prior_findings"][0];
        json!({"schema_version":1,"summary":"Exact source confirms the existing parser restriction; advisory under current requirements","reviewed_commits":[COMMIT],"findings":["Option-like tags retain the documented parser restriction"],"requires_changes":false,"evidence_requests":[],"finding_dispositions":[{"finding_id":f["finding_id"],"origin_sha256":f["origin"]["sha256"],"target_commit":COMMIT,"requirement_generation":3,"disposition":"advisory","rationale":"The exact supplied CLI source preserves the existing option-like tag restriction; no new semantics were approved","evidence_refs":["evidence:source-hash:0:6029"]}]})
    }
    #[test]
    fn finding_recorded_cli_retention_case_resolves_explicitly_without_mutating_native_output() {
        let p = packet();
        let raw = result(&p);
        let original = raw.clone();
        let applied = apply(&raw, &p).unwrap();
        assert_eq!(raw, original);
        assert_eq!(raw["requires_changes"], false);
        assert_eq!(applied["effective"]["requires_changes"], false);
        assert_eq!(p["prior_findings"][0]["disposition"], "blocking");
        assert_eq!(applied["ledger"][0]["disposition"], "advisory");
        let mut omitted = raw.clone();
        omitted["finding_dispositions"] = json!([]);
        assert_eq!(
            apply(&omitted, &p).unwrap()["effective"]["requires_changes"],
            true
        );
        let mut extra = p.clone();
        let mut f = extra["prior_findings"][0].clone();
        f["finding_id"] = json!("another-unresolved-finding");
        f["text"] = json!("Independent missing error handling");
        extra["prior_findings"].as_array_mut().unwrap().push(f);
        assert_eq!(
            apply(&raw, &extra).unwrap()["effective"]["requires_changes"],
            true
        );
    }
    #[test]
    fn finding_wrong_revision_generation_profile_lineage_and_unsupplied_evidence_are_refused() {
        let p = packet();
        let raw = result(&p);
        for (field, value) in [
            ("finding_id", json!("unrelated")),
            ("origin_sha256", json!("other-result")),
            ("target_commit", json!("f".repeat(40))),
            ("requirement_generation", json!(4)),
            ("evidence_refs", json!(["evidence:unread:0:9000"])),
        ] {
            let mut bad = raw.clone();
            bad["finding_dispositions"][0][field] = value;
            assert!(apply(&bad, &p).is_err(), "{field}");
        }
        let mut foreign = p.clone();
        foreign["execution"]["selection"]["profile_sha256"] = json!("codex-profile");
        assert!(apply(&raw, &foreign).is_err());
        let mut incomplete = p.clone();
        incomplete["evidence_pages"][0]["end"] = json!(100);
        assert!(apply(&raw, &incomplete).is_err());
    }
    #[test]
    fn finding_linked_clarification_changes_effective_gate_without_whole_review_or_unrelated_pass()
    {
        let mut p = packet();
        p["review_resolution_only"] = json!(true);
        let mut raw = result(&p);
        raw["reviewed_commits"] = json!([]);
        raw["findings"] = json!([]);
        let d = apply(&raw, &p).unwrap();
        crate::development::validate_result(&raw, &p).unwrap();
        let original = json!({"finding_contract":1,"unit_id":"cli-review","work_id":"request","requirement_generation":3,"role":"review","status":"reviewed","selection":{"profile_sha256":"claude-profile"},"finding_ledger":p["prior_findings"],"review":{"reviewed_commits":[COMMIT],"requires_changes":true}});
        let mut clarify = json!({"unit_id":"clarification","work_id":"request","requirement_generation":3,"role":"review","status":"reviewed","selection":{"profile_sha256":"claude-profile"},"finding_ledger":d["ledger"],"review":raw});
        assert!(effective_requires_changes(&original, &[original.clone()]));
        assert!(!effective_requires_changes(
            &original,
            &[original.clone(), clarify.clone()]
        ));
        assert_eq!(original["review"]["requires_changes"], true);
        assert_eq!(effective_blocking_findings(&original,&[original.clone()]).len(),1);
        assert!(effective_blocking_findings(&original,&[original.clone(),clarify.clone()]).is_empty());
        let implementation = json!({"role":"implementation","work_id":"request","requirement_generation":3,"result_commit":COMMIT});
        let unrelated = json!({"finding_contract":1,"role":"review","status":"reviewed","work_id":"request","requirement_generation":3,"review":{"reviewed_commits":[COMMIT],"requires_changes":false},"finding_ledger":[]});
        assert!(require_clear(&implementation, &[original.clone(), unrelated]).is_err());
        require_clear(&implementation, &[original.clone(), clarify.clone()]).unwrap();
        clarify["finding_ledger"] = json!([]);
        assert!(effective_requires_changes(&original, &[clarify]));
        let mut missing = raw.clone();
        missing["finding_dispositions"] = json!([]);
        assert!(validate(&missing, &p).is_err());
    }
    #[test]
    fn finding_legacy_strings_have_stable_derived_ids_and_new_contract_keeps_old_schema_valid() {
        let p = packet();
        let unit = json!({"unit_id":"old","evidence_context":{"source_operation":"old-read","unresolved_findings":["Old provisional concern"]},"selection":{"profile_sha256":"claude-profile"},"requirement_generation":3});
        let a = prior(&unit, &p).unwrap();
        assert_eq!(a, prior(&unit, &p).unwrap());
        assert_eq!(a[0]["origin"]["kind"], "legacy_context_snapshot");
        assert_eq!(a[0]["disposition"], "blocking");
        assert!(unit["finding_ledger"].is_null());
        let old = json!({"operation_kind":"development-review","review_targets":[{"result_commit":COMMIT}]});
        let result = json!({"schema_version":1,"summary":"Old frozen result","reviewed_commits":[COMMIT],"findings":[],"requires_changes":false});
        crate::development::validate_result(&result, &old).unwrap();
        assert!(
            crate::development::result_schema(&old)["properties"]["finding_dispositions"].is_null()
        );
    }
}
