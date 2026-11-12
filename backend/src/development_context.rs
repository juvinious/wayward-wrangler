//! Bounded, read-only evidence delivery. References are capabilities of an exact
//! frozen packet, never paths supplied by a provider. No execution authority here.
use crate::{
    codec::{canonical_sha256, sha256},
    execution::{read, write, write_bytes},
    session_store::Session,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path},
    process::{Command, Stdio},
};

pub const MEMORY_LIMIT: usize = 4 * 1024 * 1024;
const OBJECT_LIMIT: usize = 32 * 1024 * 1024;
const PAGE: usize = 16 * 1024;
pub const INSTRUCTION: &str = "Context delivery v1: Begin from working_summary and evidence_index. You can request application-mediated read-only evidence by returning evidence_requests [{evidence_id,path,cursor}], with action retrieve for coordinator decisions. Review retrieval responses use reviewed_commits:[]; final reviews use exact reviewed_commits and evidence_requests:[]. path is only for source entries, relative to their exact frozen commit. Empty cursor starts; use next_cursor exactly for following pages. Requests can read source, full diffs, checks and relevant saved history. Retrieval automatically continues under this same approval; it grants no writes or arbitrary filesystem access. Preserve concrete unresolved findings across retrieval turns in findings/reason. Return a concise working summary of what you learned for the next turn. Never integrate a submitted commit or finish a review before its entire diff has been delivered (check complete and delivered ranges); retrieve all missing pages. Completeness means supplied bytes, not proof of comprehension or exhaustive source review. If material needed for a conclusion was omitted, retrieve it; do not claim exhaustive review of omitted material. Superseded work is retained evidence, not permission to replay or recreate it.";

fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
pub const DISCUSSION_INSTRUCTION: &str = "For conversation responses, use action {kind:retrieve_evidence,payload:{requests:[{evidence_id,path,cursor}]}} under the normal conversation schema; do not use development decision fields. A service evidence-continuation message is not a new human instruction or permission. Preserve the source request; do not reclassify or replay any earlier approval.";
pub const MEMORY_INSTRUCTION: &str = "Working memory v2: evidence_pages retains the active read set across retrieval turns. context_summary must contain concise public factual notes: evidence identities, concrete observations, unresolved issues and the next decision, not merely narration of what you intend to read. Do not expose hidden reasoning or invent facts. Returned pages are already present; use their cursors only for missing ranges. Once enough evidence is available, decide the next authorized action rather than rereading it. release_evidence lists supplied SHA256 content hashes whose raw pages you no longer need after recording their useful facts in context_summary; use [] normally. Releases retain immutable evidence, delivery receipts and unresolved findings. New requested pages are added after releases. Complete diff delivery is necessary for review, but does not establish comprehension. Keep factual notes concise as the read set grows; summarize and release unneeded raw pages before reaching application/model bounds.";
fn compact(v: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(v).map_err(|e| e.to_string())
}
fn git_bytes(project: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut data = vec![];
    let result = child
        .stdout
        .take()
        .ok_or("Missing Git output")?
        .take((OBJECT_LIMIT + 1) as u64)
        .read_to_end(&mut data);
    if data.len() > OBJECT_LIMIT || result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Evidence object exceeds application reader memory limit or could not be read; no truncation supplied".into());
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Exact Git evidence is unavailable; no mutable-file substitution".into());
    }
    Ok(data)
}
fn snapshot(s: &Session, bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > OBJECT_LIMIT {
        return Err(
            "Evidence snapshot exceeds reader memory bound; select a narrower immutable artifact"
                .into(),
        );
    }
    let digest = sha256(bytes);
    let file = s
        .workflow
        .state
        .join("session/context-objects")
        .join(format!("{digest}.blob"));
    write_bytes(&file, bytes, true)?;
    Ok(json!({"sha256":digest,"bytes":bytes.len()}))
}
fn add(
    s: &Session,
    entries: &mut Vec<Value>,
    id: &str,
    kind: &str,
    title: &str,
    bytes: &[u8],
    extra: Value,
) -> Result<(), String> {
    let mut entry = snapshot(s, bytes)?;
    entry["id"] = json!(id);
    entry["kind"] = json!(kind);
    entry["title"] = json!(title);
    for (k, v) in extra.as_object().into_iter().flatten() {
        entry[k] = v.clone();
    }
    entries.push(entry);
    Ok(())
}
fn file_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take((OBJECT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > OBJECT_LIMIT {
        return Err("Evidence exceeds reader memory limit; no partial snapshot substituted".into());
    }
    Ok(bytes)
}
fn compact_unit(u: &Value) -> Result<Value, String> {
    let mut out = u.clone();
    let mut omitted = vec![];
    for key in [
        "instructions",
        "review_diff",
        "diff",
        "receipt",
        "runtime_usage",
        "retained_input_snapshot",
        "continuation_snapshot",
        "worker_report",
        "submission",
        "review",
        "planned_selection",
        "selection",
        "profile_override",
        "evidence_context",
        "context_summary",
        "native_review",
    ] {
        if out.as_object_mut().unwrap().remove(key).is_some() {
            omitted.push(key);
        }
    }
    out["source_record"] = json!({"sha256":hash(u)?,"evidence_id":format!("unit:{}",u["unit_id"].as_str().unwrap_or("")),"omitted_fields":omitted});
    let mut findings = if u["review"]["requires_changes"] == true {
        u["review"]["findings"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        vec![]
    };
    for finding in u["evidence_context"]["unresolved_findings"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if !findings.contains(finding) {
            findings.push(finding.clone())
        }
    }
    if let Some(ledger)=u["finding_ledger"].as_array(){findings=ledger.iter().filter(|f|f["disposition"]=="blocking").map(|f|f["text"].clone()).collect();}
    if !findings.is_empty() {
        out["unresolved_findings"] = json!(findings);
    }
    out["outcome_summary"] = json!({"implementation":u["submission"]["summary"],"review":u["review"]["summary"],"failed_report":u["worker_report"]["summary"],"requires_changes":u["review"]["requires_changes"]});
    if u["progress"].is_object(){out["progress"]=crate::development_recovery::context_progress(u);}
    if u["recovery"].is_object(){out["recovery"]=crate::development_recovery::summary(&u["recovery"]);}
    Ok(out)
}
fn compact_workflow(w: &Value) -> Result<Value, String> {
    let mut out = w.clone();
    if w["progress"].is_object(){out["progress"]=crate::development_recovery::context_progress(w);}
    if w["recovery"].is_object(){out["recovery"]=crate::development_recovery::summary(&w["recovery"]);}
    let mut omitted = vec![];
    for key in [
        "policy",
        "profile",
        "authorization",
        "coordinator_selection",
        "last_coordinator_result",
        "last_obsolete_result",
        "interrupted_coordinator",
        "workspace_recoveries",
        "generated_resources",
        "checks",
        "evidence_context",
        "context_summary",
    ] {
        if out.as_object_mut().unwrap().remove(key).is_some() {
            omitted.push(key);
        }
    }
    out["source_record"] =
        json!({"sha256":hash(w)?,"evidence_id":"workflow","omitted_fields":omitted});
    if let Some(all) = out["obligations"].as_array() {
        out["obligations"]=json!(all.iter().map(|o|{
        if o["status"]!="resolved" {o.clone()} else {json!({"name":o["name"],"status":o["status"],"owner":o["owner"],"resolution_ref":hash(&o["resolution"]).ok()})}
    }).collect::<Vec<_>>());
    }
    Ok(out)
}

/// Build an evidence capability index before freezing the operation. Full record
/// snapshots are immutable; Git reads later use only listed full commit IDs.
pub fn prepare(s: &Session, directory: &Path, packet: &mut Value) -> Result<(), String> {
    if packet["execution"]["profile"] != "development-profiles-v1" {
        return Ok(());
    }
    if directory.join("packet.json").exists() {
        return Ok(());
    } // frozen operations must never be rewritten
    packet["recovery_contract"]=json!(1);
    packet["convergence_contract"]=json!(1);
    crate::development::retain_convergence_contract(s)?;
    crate::development_recovery::retain_contract(s)?;
    let mut entries = vec![];
    let mut commits = BTreeSet::new();
    let discussion = packet["operation_kind"] == "conversation";
    if discussion {
        packet["scope_base"] = packet["selected_context"]["scope"]["base"].clone();
        let subject = packet["selected_context"]["scope"]["subject_id"]
            .as_str()
            .ok_or("Missing discussion subject")?;
        let request: Option<String> = s
            .workflow
            .connection()?
            .query_row(
                "SELECT request_id FROM request_history_requests WHERE subject_id=?1",
                [subject],
                |r| r.get(0),
            )
            .ok();
        packet["request_id"] = json!(request);
    }
    let work_id = if discussion {
        packet["selected_context"]["scope"]["target_task_id"]
            .as_str()
            .unwrap_or("")
    } else {
        packet["run_id"]
            .as_str()
            .ok_or("Missing context request identity")?
    };
    let w = s
        .development_record("workflow", work_id)?
        .unwrap_or(Value::Null);
    let owner = if discussion {
        s.development_record(
            "discussion_context",
            packet["selected_context"]["scope"]["subject_id"]
                .as_str()
                .unwrap(),
        )?
        .unwrap_or(Value::Null)
    } else if matches!(
        packet["operation_kind"].as_str(),
        Some("development-review" | "development-worker")
    ) {
        packet["assignment"].clone()
    } else {
        w.clone()
    };
    let mut previous = resolve(s, &owner["evidence_context"])?;
    if previous["scope_base"] != packet["scope_base"]
        || previous["requirement_generation"] != packet["requirement_generation"]
    {
        // Historical findings remain visible, but old delivery cannot satisfy a
        // review under a changed requirement generation or scope.
        previous["pages"] = json!([]);
        previous["working_pages"] = json!([]);
        previous["delivered"] = json!([]);
    }
    let service_continuation = discussion
        && crate::coordination::turn_context(s, packet["turn_id"].as_str().unwrap_or(""))?["intent_kind"]
            == "evidence_continuation";
    for c in [
        &w["head"],
        &packet["integration_head"],
        &packet["assignment"]["base_commit"],
    ] {
        if let Some(c) = c.as_str() {
            commits.insert(c.to_owned());
        }
    }
    if discussion && commits.is_empty() {
        if let Ok(head) = crate::development::git(&s.project, &["rev-parse", "HEAD"]) {
            commits.insert(head);
        }
    }
    let all = s
        .development_records("unit")?
        .into_iter()
        .filter(|u| u["work_id"] == work_id)
        .collect::<Vec<_>>();
    let visible = if packet["operation_kind"] == "development-review" {
        packet["review_targets"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else if packet["operation_kind"] == "development-worker" {
        vec![packet["assignment"].clone()]
    } else {
        all.clone()
    };
    if packet["operation_kind"] == "development-review" {
        commits.clear();
    }
    for u in &visible {
        for key in ["base_commit", "result_commit"] {
            if let Some(c) = u[key].as_str() {
                commits.insert(c.into());
            }
        }
        let uid = u["unit_id"].as_str().ok_or("Missing unit identity")?;
        add(
            s,
            &mut entries,
            &format!("unit:{uid}"),
            "record",
            uid,
            &compact(u)?,
            json!({"unit_id":uid}),
        )?;
        if let (Some(base), Some(head)) = (u["base_commit"].as_str(), u["result_commit"].as_str()) {
            let bytes = git_bytes(
                &s.project,
                &["diff", "--no-ext-diff", "--no-textconv", base, head],
            )?;
            add(
                s,
                &mut entries,
                &format!("diff:{uid}"),
                "diff",
                u["title"].as_str().unwrap_or(uid),
                &bytes,
                json!({"unit_id":uid,"base_commit":base,"commit":head}),
            )?;
        }
    }
    for c in commits {
        if c.len() != 40 || !c.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Evidence requires exact commit IDs".into());
        }
        let tree = git_bytes(&s.project, &["ls-tree", "-r", "--name-only", &c])?;
        add(
            s,
            &mut entries,
            &format!("tree:{c}"),
            "tree",
            "Tracked source paths at exact commit",
            &tree,
            json!({"commit":c}),
        )?;
        let index=crate::project_knowledge::index(&s.project,&c)?;
        add(s,&mut entries,&format!("project-index:{c}"),"knowledge","Complete eligible project path/instruction/convention map",&compact(&index)?,json!({"commit":c}))?;
        entries.push(json!({"id":format!("source:{c}"),"kind":"source","commit":c,"title":"Read a tracked blob at this exact commit using path; no filesystem traversal"}));
    }
    if !w.is_null() {
        add(
            s,
            &mut entries,
            "workflow",
            "record",
            "Full saved workflow and authority",
            &compact(&w)?,
            json!({}),
        )?;
        add(
            s,
            &mut entries,
            "checks",
            "checks",
            "Exact saved combined check results",
            &compact(&w["checks"])?,
            json!({"commit":w["checks"]["commit"]}),
        )?;
        for (i, check) in w["checks"]["result"]["checks"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            for stream in ["stdout", "stderr"] {
                if let (Some(path), Some(expected)) = (
                    check[stream].as_str(),
                    check[format!("{stream}_sha256")].as_str(),
                ) {
                    let path = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
                    if !path
                        .starts_with(s.workflow.state.canonicalize().map_err(|e| e.to_string())?)
                    {
                        return Err("Check evidence leaves project state".into());
                    }
                    let bytes = file_bytes(&path)?;
                    if sha256(&bytes) != expected {
                        return Err("Check evidence hash changed".into());
                    }
                    add(
                        s,
                        &mut entries,
                        &format!("check:{i}:{stream}"),
                        "check-output",
                        "Immutable captured check output",
                        &bytes,
                        json!({"commit":w["checks"]["commit"]}),
                    )?;
                }
            }
        }
    }
    for r in crate::request_history::requests(s)?
        .into_iter()
        .filter(|r| r["request_id"] == packet["request_id"])
    {
        if let Some(log) = r["log_path"].as_str().filter(|p| Path::new(p).is_file()) {
            let path = Path::new(log).canonicalize().map_err(|e| e.to_string())?;
            let history = crate::project_layout::history_root(&s.project, &s.workflow.state)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !path.starts_with(history) {
                return Err("Readable history leaves this project's evidence root".into());
            }
            let bytes = file_bytes(&path)?;
            add(
                s,
                &mut entries,
                "history",
                "history",
                "Request history snapshot at preparation",
                &bytes,
                json!({}),
            )?;
        } else {
            let c = s.workflow.connection()?;
            let mut q=c.prepare("SELECT e.seq,e.type,e.body FROM session_events e JOIN request_history_events h USING(seq) WHERE h.request_id=?1 ORDER BY e.seq").map_err(|e|e.to_string())?;
            let mut bytes = vec![];
            for row in q
                .query_map([r["request_id"].as_str()], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
            {
                let (seq, kind, body) = row.map_err(|e| e.to_string())?;
                let body: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                bytes.extend(compact(
                    &json!({"seq":seq,"type":kind,"body":crate::request_history::public(&body)}),
                )?);
                bytes.push(b'\n');
                if bytes.len() > OBJECT_LIMIT {
                    return Err("History snapshot exceeds reader memory bound; select a narrower immutable record".into());
                }
            }
            add(
                s,
                &mut entries,
                "history",
                "history",
                "Redacted request journal snapshot; readable projection not yet published",
                &bytes,
                json!({}),
            )?;
        }
    }
    if let Some(all) = packet["knowledge"].as_array() {
        for (i, k) in all.iter().enumerate() {
            add(
                s,
                &mut entries,
                &format!("knowledge:{i}"),
                "knowledge",
                "Accepted project knowledge at exact source commit",
                &compact(k)?,
                json!({"commit":k["source_commit"]}),
            )?;
        }
        packet["knowledge"] = json!(
            all.iter()
                .rev()
                .take(2)
                .map(crate::development::discussion_knowledge)
                .collect::<Vec<_>>()
        );
    }
    if previous["scope_base"] == packet["scope_base"] {
        for entry in &mut entries {
            if let Some(old) = previous["entries"].as_array().and_then(|all| {
                all.iter()
                    .find(|old| old["id"] == entry["id"] && old["kind"] == entry["kind"])
            }) {
                let mut retained = old["retained_snapshots"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                if !old["sha256"].is_null() && old["sha256"] != entry["sha256"] {
                    let mut old = old.clone();
                    old.as_object_mut().unwrap().remove("retained_snapshots");
                    if !retained.iter().any(|v| v["sha256"] == old["sha256"]) {
                        retained.push(old)
                    }
                }
                if !retained.is_empty() {
                    entry["retained_snapshots"] = json!(retained);
                }
            }
        }
    }
    let manifest = json!({"version":1,"project_key":s.key,"request_id":packet["request_id"],"scope_base":packet["scope_base"],"requirement_generation":packet["requirement_generation"],"entries":entries});
    write(&directory.join("context-evidence.json"), &manifest, true)?;
    packet["evidence_index"] = json!({"version":1,"manifest_sha256":hash(&manifest)?,"manifest_file":directory.join("context-evidence.json"),"entries":entries,"page_bytes":PAGE,"retrieval":"Return evidence_requests; application supplies immutable pages in the next turn under the same approval"});
    let active = service_continuation
        || packet["operation_kind"] == "development-review"
        || w["evidence_pending"] == true;
    packet["working_summary"] = json!({"prior_turn":if packet["operation_kind"]=="development-coordinator" && w["context_summary"].is_object() {w["context_summary"].clone()} else if active {previous["summary"].clone()} else {Value::Null},"source_request":if discussion&&!service_continuation||previous["source_request"].is_null(){packet["selected_context"]["current_message"].clone()}else{previous["source_request"].clone()},"goal":packet["requirements"]["objective"],"head":w["head"],"status":w["status"],"attention":w["attention"],"unresolved_findings":previous["unresolved_findings"]});
    if let Some(commit)=w["head"].as_str().or(packet["assignment"]["base_commit"].as_str()) {
        let paths=if let Some(paths)=packet["assignment"]["paths"].as_array(){paths.iter().filter_map(|p|p.as_str().map(str::to_owned)).collect::<Vec<_>>()}else{packet["review_targets"].as_array().into_iter().flatten().flat_map(|u|u["paths"].as_array().into_iter().flatten()).filter_map(|p|p.as_str().map(str::to_owned)).collect::<Vec<_>>()};
        packet["applicable_instructions"]=json!(crate::project_knowledge::applicable(&s.project,commit,&paths)?);
    }
    if w["policy"]["project_workflow"].is_object(){packet["ordinary_project_contract"]=json!(1);}
    packet["working_summary"]["integration_checkout"] = crate::development::integration_checkout(s,&w);
    if packet["operation_kind"]=="development-coordinator" { packet["convergence"] = crate::development::convergence(s,&w,&all); }
    if packet["finding_contract"]==1{packet["working_summary"]["unresolved_findings"]=json!(packet["prior_findings"].as_array().into_iter().flatten().filter(|f|f["disposition"]=="blocking").map(|f|f["text"].clone()).collect::<Vec<_>>());}
    packet["evidence_pages"] = json!(if active {
        working_pages(s, &previous)?
    } else {
        vec![]
    });
    packet["evidence_receipts"] = previous["delivered"]
        .as_array()
        .cloned()
        .map(Value::from)
        .unwrap_or(json!([]));
    if packet["operation_kind"] == "development-coordinator" {
        packet["workflow"] = compact_workflow(&w)?;
        packet["units"] = json!(
            all.iter()
                .map(compact_unit)
                .collect::<Result<Vec<_>, _>>()?
        );
    }
    if packet["operation_kind"]=="development-coordinator"{for (i,u) in all.iter().enumerate(){if u["role"]=="review"{
        packet["units"][i]["effective_requires_changes"]=json!(crate::review_findings::effective_requires_changes(u,&all));
        packet["units"][i]["unresolved_findings"]=json!(crate::review_findings::effective_blocking_findings(u,&all).iter().map(|f|f["text"].clone()).collect::<Vec<_>>());
        if let Some(ledger)=packet["units"][i].as_object_mut().unwrap().remove("finding_ledger") {
            packet["units"][i]["original_findings_reference"]=json!({"sha256":hash(&ledger)?,"evidence_id":format!("unit:{}",u["unit_id"].as_str().unwrap_or("")),"label":"Original ledger retained; unresolved_findings applies validated linked dispositions"});
        }
    }}}
    if packet["assignment"].is_object() {
        let assignment = packet["assignment"].clone();
        packet["assignment"] = compact_unit(&assignment)?;
        // Instructions are mandatory for an implementation/review assignment.
        packet["assignment"]["instructions"] = assignment["instructions"].clone();
    }
    if let Some(targets) = packet["review_targets"].as_array() {
        packet["review_targets"] = json!(
            targets
                .iter()
                .map(compact_unit)
                .collect::<Result<Vec<_>, _>>()?
        );
    }
    if packet["routing_preferences"].is_object() {
        let p = packet["routing_preferences"].take();
        packet["routing_preferences"] = json!({"revision":p["revision"],"routing":"Approved exact profiles/allowed roles are in policy; select a profile ID there. Frozen current selection is execution.selection", "defaults":p["preferences"]["roles"],"worker_default":p["preferences"]["worker_default"]});
    }
    if !packet["checks"].is_null() {
        packet["checks"] = json!({"commit":w["checks"]["commit"],"passed":w["checks"]["result"]["passed"],"evidence_id":"checks"});
    }
    let omitted = json!([{"kind":"completed/superseded detail and released raw pages","reason":"Concise outcomes and exact evidence references replace repeated policy/routing/results; active reads and unresolved findings remain","available":"evidence_index; previous exchange files"}]);
    packet["context_delivery"] = json!({"version":1,"memory_version":2,"serialization":"compact JSON","omissions":omitted,"required_information":"Current requirements, one policy, exact execution selection, unresolved findings and current state retained","review_scope":"Only supplied/retrieved evidence; no exhaustive source-review claim"});
    // Supply fitting relevant diffs immediately; larger reads remain explicit
    // indexed pages. Never displace requested continuation pages or required data.
    if packet["evidence_pages"]
        .as_array()
        .is_some_and(Vec::is_empty)
        && packet["operation_kind"] != "development-worker"
    {
        let target = packet["execution"]["context_bytes"]
            .as_u64()
            .unwrap_or(96 * 1024) as usize;
        for entry in manifest["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|e| e["kind"] == "diff")
        {
            let relevant = packet["operation_kind"] == "development-review"
                || packet["units"].as_array().is_some_and(|all| {
                    all.iter()
                        .any(|u| u["unit_id"] == entry["unit_id"] && u["status"] == "submitted")
                });
            if !relevant {
                continue;
            }
            let mut request = Request {
                evidence_id: entry["id"].as_str().unwrap().into(),
                path: String::new(),
                cursor: String::new(),
            };
            loop {
                let supplied = page(s, &manifest, &request)?;
                let next = supplied["next_cursor"].as_str().unwrap().to_owned();
                packet["evidence_pages"]
                    .as_array_mut()
                    .unwrap()
                    .push(supplied);
                if compact(packet)?.len() > target {
                    packet["evidence_pages"].as_array_mut().unwrap().pop();
                    break;
                }
                if next.is_empty() {
                    break;
                }
                request.cursor = next;
            }
        }
    }
    // Do not silently discard required content to hit a preference.
    if discussion {
        let mut citations = packet["selected_context"]["evidence_allowlist"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for supplied in packet["evidence_pages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["error"].is_null())
        {
            if let Some(citation) = supplied["citation"].as_str() {
                if !citations.contains(&json!(citation)) {
                    citations.push(json!(citation));
                }
            }
        }
        packet["selected_context"]["evidence_allowlist"] = json!(citations);
        packet["context_sha256"] = json!(hash(&packet["selected_context"])?);
    }
    diagnose(packet, directory)?;
    write(
        &s.workflow.state.join("context-delivery-v1.json"),
        &json!({"version":1,"contract":"evidence-v1"}),
        true,
    )?;
    write(
        &s.workflow.state.join("context-memory-v2.json"),
        &json!({"version":2,"contract":"working-memory-v2"}),
        true,
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub evidence_id: String,
    pub path: String,
    pub cursor: String,
}
pub fn request_schema() -> Value {
    json!({"type":"array","maxItems":8,"items":{"type":"object","properties":{"evidence_id":{"type":"string"},"path":{"type":"string"},"cursor":{"type":"string"}},"required":["evidence_id","path","cursor"],"additionalProperties":false}})
}
pub fn validate_requests(value: &Value) -> Result<Vec<Request>, String> {
    let all: Vec<Request> = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if all.len() > 8
        || all
            .iter()
            .any(|r| r.evidence_id.len() > 256 || r.path.len() > 4096 || r.cursor.len() > 128)
    {
        return Err("Evidence request exceeds bounded read protocol".into());
    }
    Ok(all)
}
fn page(s: &Session, manifest: &Value, request: &Request) -> Result<Value, String> {
    let mut entry = manifest["entries"]
        .as_array()
        .and_then(|a| a.iter().find(|e| e["id"] == request.evidence_id))
        .ok_or("Evidence reference not granted by this frozen packet")?;
    if let Some((sha, _)) = request.cursor.split_once(':') {
        if entry["kind"] != "source" && entry["sha256"] != sha {
            entry = entry["retained_snapshots"]
                .as_array()
                .and_then(|all| all.iter().find(|v| v["sha256"] == sha))
                .ok_or("Cursor snapshot not in this frozen evidence grant")?;
        }
    }
    let (bytes, meta) = if entry["kind"] == "source" {
        let path = Path::new(&request.path);
        if !crate::project_knowledge::allowed_path(&request.path)
            || request.path.is_empty()
            || !path.components().all(|c| matches!(c, Component::Normal(_)))
            || path.starts_with(".git")
            || path.starts_with(".waywr")
            || path.starts_with(".wayward-wrangler")
        {
            return Err("Source path is not a contained tracked source name".into());
        }
        let commit = entry["commit"]
            .as_str()
            .ok_or("Missing frozen source commit")?;
        let bytes = git_bytes(
            &s.project,
            &["cat-file", "blob", &format!("{commit}:{}", request.path)],
        )?;
        let meta = snapshot(s, &bytes)?;
        (bytes, meta)
    } else {
        if !request.path.is_empty() {
            return Err("Only source references accept a path".into());
        }
        let digest = entry["sha256"].as_str().ok_or("Missing evidence hash")?;
        let bytes = file_bytes(
            &s.workflow
                .state
                .join("session/context-objects")
                .join(format!("{digest}.blob")),
        )?;
        if sha256(&bytes) != digest {
            return Err("Immutable evidence object changed; read refused".into());
        }
        (bytes, entry.clone())
    };
    let digest = meta["sha256"].as_str().ok_or("Missing content identity")?;
    let offset = if request.cursor.is_empty() {
        0
    } else {
        let (sha, n) = request
            .cursor
            .split_once(':')
            .ok_or("Invalid evidence cursor")?;
        if sha != digest {
            return Err("Evidence cursor refers to a different immutable snapshot".into());
        }
        n.parse::<usize>().map_err(|_| "Invalid evidence offset")?
    };
    if offset > bytes.len() {
        return Err("Evidence offset exceeds object".into());
    }
    let mut end = (offset + PAGE).min(bytes.len());
    let text = std::str::from_utf8(&bytes).ok();
    if let Some(text) = text {
        if !text.is_char_boundary(offset) {
            return Err("Cursor splits UTF-8".into());
        }
        while !text.is_char_boundary(end) {
            end -= 1;
        }
    }
    let encoding = if text.is_some() { "utf8" } else { "hex" };
    let data = if let Some(text) = text {
        text[offset..end].to_owned()
    } else {
        bytes[offset..end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    let mut provenance = entry.clone();
    provenance.as_object_mut().unwrap().remove("retained_snapshots");
    Ok(
        json!({"evidence_id":request.evidence_id,"path":request.path,"sha256":digest,"offset":offset,"end":end,"total_bytes":bytes.len(),"encoding":encoding,"data":data,"citation":format!("evidence:{digest}:{offset}:{end}"),"complete":end==bytes.len(),"next_cursor":if end==bytes.len(){String::new()}else{format!("{digest}:{end}")},"provenance":provenance}),
    )
}
/// Called only after a validated completed coordinator/reviewer result. No native
/// attempt is replayed; caller saves continuation state then prepares a fresh turn.
fn supplied_ranges(page: &Value) -> Vec<(u64, u64)> {
    if let Some(all) = page["ranges"].as_array() {
        return all
            .iter()
            .filter_map(|v| Some((v[0].as_u64()?, v[1].as_u64()?)))
            .collect();
    }
    match (page["offset"].as_u64(), page["end"].as_u64()) {
        (Some(a), Some(b)) => vec![(a, b)],
        _ => vec![],
    }
}
fn delivery_receipts(
    prior: &Value,
    pages: &Value,
    packet_hash: &str,
) -> Result<Vec<Value>, String> {
    let mut grouped: BTreeMap<String, Value> = BTreeMap::new();
    for page in prior
        .as_array()
        .into_iter()
        .flatten()
        .chain(pages.as_array().into_iter().flatten())
    {
        if !page["error"].is_null() || page["sha256"].is_null() {
            continue;
        }
        let key = hash(&json!([page["evidence_id"], page["path"], page["sha256"]]))?;
        let entry=grouped.entry(key).or_insert_with(||json!({"evidence_id":page["evidence_id"],"path":page["path"],"sha256":page["sha256"],"total_bytes":page["total_bytes"],"ranges":[],"latest_packet_sha256":packet_hash}));
        for (a, b) in supplied_ranges(page) {
            entry["ranges"].as_array_mut().unwrap().push(json!([a, b]));
        }
    }
    for value in grouped.values_mut() {
        let mut all = supplied_ranges(value);
        all.sort_unstable();
        let mut merged: Vec<(u64, u64)> = vec![];
        for (a, b) in all {
            if let Some(last) = merged.last_mut() {
                if a <= last.1 {
                    last.1 = last.1.max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }
        value["ranges"] = json!(merged);
        value["complete"] =
            json!(merged.first().is_some_and(
                |r| r.0 == 0 && r.1 >= value["total_bytes"].as_u64().unwrap_or(u64::MAX)
            ));
    }
    Ok(grouped.into_values().collect())
}
/// Validate public memory fields only for packets whose frozen schema requests
/// them. Old completed v1 attempts retain their original result contract.
pub fn validate_memory(value: &Value) -> Result<(), String> {
    let summary = value["context_summary"]
        .as_str()
        .ok_or("Missing public context summary")?;
    let releases = value["release_evidence"]
        .as_array()
        .ok_or("Missing evidence release list")?;
    if summary.len() > 16384
        || releases.len() > 128
        || releases.iter().any(|v| {
            v.as_str()
                .is_none_or(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
        })
    {
        return Err("Invalid bounded public context summary or evidence hashes".into());
    }
    if !releases.is_empty() && summary.trim().is_empty() {
        return Err("Summarize useful facts before releasing raw evidence".into());
    }
    Ok(())
}
fn union_pages(target: &mut Vec<Value>, pages: &Value) -> Result<(), String> {
    let mut keys = target
        .iter()
        .map(|p| {
            hash(&json!([
                p["evidence_id"],
                p["path"],
                p["sha256"],
                p["offset"],
                p["end"],
                p["error"]
            ]))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    for p in pages.as_array().into_iter().flatten() {
        if keys.insert(hash(&json!([
            p["evidence_id"],
            p["path"],
            p["sha256"],
            p["offset"],
            p["end"],
            p["error"]
        ]))?) {
            target.push(p.clone());
        }
    }
    if compact(&json!(target))?.len() > MEMORY_LIMIT {
        return Err(
            "Active read set exceeds application memory bound; no evidence silently discarded"
                .into(),
        );
    }
    Ok(())
}
/// Backfill active reads from immutable v1 exchanges, without replaying any call.
/// v2 stores the union explicitly, including deliberate releases.
fn working_pages(s: &Session, context: &Value) -> Result<Vec<Value>, String> {
    let mut current = context.clone();
    let mut pages = vec![];
    let mut seen = BTreeSet::new();
    loop {
        if current["scope_base"] != context["scope_base"]
            || current["requirement_generation"] != context["requirement_generation"]
        {
            break;
        }
        if current["working_pages"].is_array() {
            union_pages(&mut pages, &current["working_pages"])?;
            break;
        }
        union_pages(&mut pages, &current["pages"])?;
        let Some(path) = current["previous_exchange"]["path"].as_str() else {
            break;
        };
        if !seen.insert(path.to_owned()) {
            return Err("Cycle in immutable evidence exchanges".into());
        }
        current = resolve(
            s,
            &json!({"state_file":path,"sha256":current["previous_exchange"]["sha256"]}),
        )?;
    }
    Ok(pages)
}
pub fn retrieve(
    s: &Session,
    directory: &Path,
    packet: &Value,
    requests: &[Request],
    prior: &Value,
    summary: &str,
    findings: &Value,
) -> Result<Value, String> {
    retrieve_with_memory(
        s,
        directory,
        packet,
        requests,
        prior,
        summary,
        findings,
        &[],
    )
}
pub fn retrieve_with_memory(
    s: &Session,
    directory: &Path,
    packet: &Value,
    requests: &[Request],
    prior: &Value,
    summary: &str,
    findings: &Value,
    releases: &[String],
) -> Result<Value, String> {
    let previous_exchange = if prior["state_file"].is_string() {
        json!({"path":prior["state_file"],"sha256":prior["sha256"]})
    } else {
        Value::Null
    };
    let mut prior = resolve(s, prior)?;
    if prior["scope_base"] != packet["scope_base"]
        || prior["requirement_generation"] != packet["requirement_generation"]
    {
        prior["delivered"] = json!([]);
        prior["pages"] = json!([]);
        prior["working_pages"] = json!([]);
    }
    let manifest = read(&directory.join("context-evidence.json"))?;
    if hash(&manifest)? != packet["evidence_index"]["manifest_sha256"]
        || manifest["project_key"] != s.key
        || manifest["scope_base"] != packet["scope_base"]
        || manifest["request_id"] != packet["request_id"]
    {
        return Err("Frozen evidence grant identity changed".into());
    }
    let delivered = delivery_receipts(
        &prior["delivered"],
        &packet["evidence_pages"],
        &hash(packet)?,
    )?;
    let mut unresolved = prior["unresolved_findings"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for f in findings.as_array().into_iter().flatten() {
        if !unresolved.contains(f) {
            unresolved.push(f.clone());
        }
    }
    let mut pages = vec![];
    for request in requests {
        match page(s,&manifest,request){Ok(p)=>pages.push(p),Err(error)=>pages.push(json!({"evidence_id":request.evidence_id,"path":request.path,"error":error,"complete":false,"no_bytes_supplied":true}))}
    }
    let mut active = if packet["context_delivery"]["memory_version"] == 2 {
        vec![]
    } else {
        working_pages(s, &prior)?
    };
    union_pages(&mut active, &packet["evidence_pages"])?;
    for release in releases {
        if !packet["evidence_pages"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(delivered.iter())
            .any(|p| p["sha256"] == *release)
        {
            return Err("Cannot release evidence absent from supplied input".into());
        }
    }
    active.retain(|p| !releases.iter().any(|h| p["sha256"] == *h));
    union_pages(&mut active, &json!(pages))?;
    let result = json!({"version":1,"memory_version":2,"summary":summary,"source_request":packet["working_summary"]["source_request"],"pages":pages,"working_pages":active,"released_evidence":releases,"delivered":delivered,"unresolved_findings":unresolved,"source_operation":packet["operation_id"],"scope_base":packet["scope_base"],"requirement_generation":packet["requirement_generation"],"entries":manifest["entries"],"previous_exchange":previous_exchange});
    write(&directory.join("context-exchange.json"), &result, true)?;
    Ok(result)
}
pub fn reference(directory: &Path, context: &Value) -> Result<Value, String> {
    let mut value = audit(context);
    value["version"] = json!(1);
    value["state_file"] = json!(directory.join("context-exchange.json"));
    value["sha256"] = json!(hash(context)?);
    Ok(value)
}
fn resolve(s: &Session, context: &Value) -> Result<Value, String> {
    let Some(path) = context["state_file"].as_str() else {
        return Ok(context.clone());
    };
    let path = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(s.workflow.state.canonicalize().map_err(|e| e.to_string())?) {
        return Err("Evidence continuation leaves project state".into());
    }
    let full = read(&path)?;
    if hash(&full)? != context["sha256"] {
        return Err("Immutable evidence continuation hash changed".into());
    }
    Ok(full)
}
/// Only immutable supplied/read ranges count as evidence progress. Repeated
/// workflow/history snapshots and model narration do not create a useful delta.
pub(crate) fn progress_evidence(s:&Session,context:&Value)->Result<Value,String>{
    let context=resolve(s,context)?;
    let receipts=delivery_receipts(&context["delivered"],&context["pages"],"progress projection; not authority")?;
    let mut selected=std::collections::BTreeSet::new();
    for r in receipts{
        let entry=context["entries"].as_array().into_iter().flatten().find(|e|e["id"]==r["evidence_id"]);
        let kind=entry.and_then(|e|e["kind"].as_str()).unwrap_or("");
        if matches!(kind,"source"|"diff"|"tree"|"check-output"|"checks"|"knowledge"){
            selected.insert(serde_json::to_string(&json!([kind,r["sha256"],r["path"],r["ranges"]])).map_err(|e|e.to_string())?);
        }
    }
    Ok(json!(selected))
}
pub fn audit(context: &Value) -> Value {
    json!({"summary":context["summary"],"source_operation":context["source_operation"],"scope_base":context["scope_base"],"unresolved_findings":context["unresolved_findings"],"pages":context["pages"].as_array().map(|all|all.iter().map(|p|{let mut p=p.clone();if let Some(o)=p.as_object_mut(){o.remove("data");}p}).collect::<Vec<_>>()),"delivery_receipts":context["delivered"].as_array().map(Vec::len),"state_file":context["state_file"],"sha256":context["sha256"]})
}
pub fn diagnostics(s: &Session) -> Result<Vec<Value>, String> {
    let mut files = vec![];
    for root in [
        s.workflow.state.join("session/development"),
        s.workflow.state.join("session/coordinator"),
    ] {
        if !root.is_dir() {
            continue;
        }
        for work in fs::read_dir(root).map_err(|e| e.to_string())? {
            let work = work.map_err(|e| e.to_string())?.path();
            if !work.is_dir() {
                continue;
            }
            for op in fs::read_dir(work).map_err(|e| e.to_string())? {
                let path = op
                    .map_err(|e| e.to_string())?
                    .path()
                    .join("context-diagnostics.json");
                if let Ok(meta) = fs::metadata(&path) {
                    files.push((meta.modified().ok(), path));
                }
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files
        .into_iter()
        .take(8)
        .map(|(_, path)| {
            let mut value = read(&path)?;
            value["diagnostics_file"] = json!(path);
            Ok(value)
        })
        .collect()
}
pub fn diff_delivered(packet: &Value, unit: &Value) -> bool {
    let Some(entry) = packet["evidence_index"]["entries"]
        .as_array()
        .and_then(|a| {
            a.iter().find(|e| {
                e["kind"] == "diff"
                    && e["unit_id"] == unit["unit_id"]
                    && e["commit"] == unit["result_commit"]
                    && e["base_commit"] == unit["base_commit"]
            })
        })
    else {
        return false;
    };
    let mut ranges = packet["evidence_receipts"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(packet["evidence_pages"].as_array().into_iter().flatten())
        .filter(|p| {
            p["sha256"] == entry["sha256"]
                && p["evidence_id"] == entry["id"]
                && p["error"].is_null()
        })
        .flat_map(supplied_ranges)
        .collect::<Vec<_>>();
    ranges.sort_unstable();
    let mut through = 0;
    for (start, end) in ranges {
        if start > through {
            break;
        }
        through = through.max(end);
    }
    through >= entry["bytes"].as_u64().unwrap_or(u64::MAX)
}

/// Native tokenizer/account-specific effective capacity is not available through
/// these CLI transports. Keep that uncertainty separate from application bounds.
pub fn diagnose(packet: &Value, directory: &Path) -> Result<Value, String> {
    let schema = crate::development::result_schema(packet);
    let instruction = crate::development::instruction(packet);
    let bytes = compact(packet)?;
    let schema_bytes = compact(&schema)?.len();
    let instruction_bytes = instruction.len() + if packet["ordinary_project_contract"]==1{crate::candidate::context_instruction(packet).len()}else{0}
        + if packet["finding_contract"]==1{crate::review_findings::INSTRUCTION.len()}else if packet["recovery_contract"]==1&&packet["operation_kind"]=="development-coordinator"{crate::development_recovery::INSTRUCTION.len()}else{0}
        + if packet["context_delivery"]["memory_version"] == 2
            && matches!(
                packet["operation_kind"].as_str(),
                Some("development-coordinator" | "development-review")
            )
        {
            MEMORY_INSTRUCTION.len()
        } else {
            0
        }
        + if packet["context_delivery"]["version"] == 1
            && packet["operation_kind"] != "development-worker"
        {
            INSTRUCTION.len()
                + if packet["operation_kind"] == "conversation" {
                    DISCUSSION_INSTRUCTION.len()
                } else {
                    0
                }
        } else {
            0
        };
    let text = serde_json::to_string(packet).map_err(|e| e.to_string())?;
    let chars = text.chars().count() + schema.to_string().chars().count() + instruction_bytes;
    let estimated = (chars + 2) / 3;
    let worker = packet["operation_kind"] == "development-worker";
    let published:Option<(usize,&str)>=None; // Capacity is reported by the runtime catalog/usage, never a core model table.
    let sections=packet.as_object().into_iter().flatten()
        .map(|(k,v)|Ok((k.clone(),json!(compact(v)?.len()))))
        .collect::<Result<serde_json::Map<_,_>,String>>()?;
    let response_reserve = if published.is_some() { 128000 } else { 32768 };
    let reserve = response_reserve + if worker { 65536 } else { 8192 } + 16384;
    let known_input_bytes = bytes.len() + schema_bytes + instruction_bytes + 64;
    let report = json!({"version":1,"role":packet["role"],"profile_id":packet["execution"]["selection"]["profile_id"],"provider":packet["provider"],"model":packet["model"],"packet_bytes":bytes.len(),"known_input_bytes":known_input_bytes,"section_bytes":sections,"instructions_bytes":instruction_bytes,"schema_bytes":schema_bytes,"prompt_overhead":"Native client/system/tool framing unknown; explicit instructions counted once, schemas separately","token_estimate":{"value":estimated,"with_reserved_headroom":estimated+reserve,"method":"Heuristic Unicode characters / 3; not a tokenizer count or guaranteed upper bound","tokenizer":"unavailable for this exact native transport; no byte/token equivalence","uncertainty":"high; native receipt usage is observed separately"},"published_model_capacity":published.map(|(n,url)|json!({"tokens":n,"source":url,"effective_native_capacity":"unknown; API documentation is not account/CLI negotiation"})),"headroom":{"response_tokens":response_reserve,"tool_result_tokens":if worker{65536}else{8192},"client_system_tokens":16384,"meaning":"Published maximum output reserved for known models; application reserves for tools/system are estimates, not provider guarantees. Workers' later tool traffic is native-client managed."},"working_target_bytes":packet["execution"]["context_bytes"],"target_semantics":"soft working-set preference; mandatory data may exceed it","application_memory_limit_bytes":MEMORY_LIMIT,"transport":"Evidence pages are bounded independently; packet files do not use the TUI IPC frame","omissions":packet["context_delivery"]["omissions"],"retrieval_available":packet["evidence_index"]["version"]==1&&!worker,"supplied":audit(&json!({"pages":packet["evidence_pages"]}))["pages"],"evidence_manifest_sha256":packet["evidence_index"]["manifest_sha256"]});
    write(&directory.join("context-diagnostics.json"), &report, false)?;
    if known_input_bytes > MEMORY_LIMIT {
        return Err(format!(
            "Context exceeds application memory bound of {MEMORY_LIMIT} bytes; required information preserved, Inspect context diagnostics"
        ));
    }
    if let Some((capacity, _)) = published {
        if estimated + reserve > capacity {
            return Err("Estimated prompt plus response/tool/system headroom exceeds published capacity; exact native capacity is unknown. Inspect context diagnostics; no truncation or launch".into());
        }
    }
    Ok(report)
}
