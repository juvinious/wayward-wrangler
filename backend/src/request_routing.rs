//! Human request-specific role choices, distinct from broad permission and project defaults.
use crate::{
    codec::{canonical_sha256, sha256},
    conversation_store::{encode, event},
    provider_selection,
    session_store::Session,
};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::{Value, json};

fn choices(s: &Session, text: &str) -> Result<Value, String> {
    let t = text.to_lowercase();
    if !t.trim_start().starts_with("use ")
        || !t.contains("for this work") && !t.contains("for this request")
        || text.contains(['?', '`', '>', '"'])
    {
        return Ok(Value::Null);
    }
    if [" if ", " unless ", " except ", " or "]
        .iter()
        .any(|w| t.contains(w))
    {
        return Ok(Value::Null);
    }
    let (prefs, _) = provider_selection::preferences(s)?;
    let words = t
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    let mut current = None;
    let mut selected = serde_json::Map::new();
    for word in words {
        let names = prefs
            .profiles
            .iter()
            .filter(|(name, p)| name.as_str() == word || p.provider == word)
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>();
        if names.len() == 1 {
            current = Some(names[0].clone());
            continue;
        }
        let role = match word {
            "review" | "reviewer" | "reviewing" => "review",
            "coordinator" | "coordinating" | "coordination" => "coordinator",
            "implementation" | "implementer" | "implementing" => "implementation",
            _ => continue,
        };
        let Some(name) = current.as_ref() else {
            continue;
        };
        if !prefs.role_allowed(role).contains(name) {
            return Err(format!(
                "Configure {name} for {role} in F3 before selecting it for this request"
            ));
        }
        if role == "coordinator" && provider_selection::current(s)?["profile_id"] != *name {
            return Err("Switch the coordinator through the existing safe handover before pinning that coordinator for this request".into());
        }
        if selected
            .get(role)
            .is_some_and(|v: &Value| v["profile_id"] != *name)
        {
            return Err("Conflicting role choices need one explicit profile".into());
        }
        let profile = prefs.profile_value(name)?;
        selected.insert(role.into(),json!({"mode":"pin","profile_id":name,"profile_sha256":canonical_sha256(&profile).map_err(|e|e.to_string())?,"profile":profile}));
    }
    Ok(if selected.is_empty() {
        Value::Null
    } else {
        Value::Object(selected)
    })
}
pub fn save_source(
    tx: &Transaction<'_>,
    s: &Session,
    subject: &str,
    message: &str,
    text: &str,
) -> Result<bool, String> {
    if !provider_selection::enabled(s) {
        return Ok(false);
    }
    let roles = choices(s, text)?;
    if roles.is_null() {
        return Ok(false);
    }
    let (request, task): (String, Option<String>) = tx
        .query_row(
            "SELECT request_id,task_id FROM request_history_requests WHERE subject_id=?1",
            [subject],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if let Some(task) = &task {
        let active: bool = tx
            .query_row(
                "SELECT exists(SELECT 1 FROM development_records WHERE kind='workflow' AND id=?1)",
                [task],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active {
            return Ok(false);
        } // Active changes use existing explicit task/policy controls; never rewrite frozen work.
    }
    let raw: Option<String> = tx
        .query_row(
            "SELECT record_json FROM development_records WHERE kind='request_routing' AND id=?1",
            [subject],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut record = raw
        .map(|v| serde_json::from_str::<Value>(&v).map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(
            json!({"schema_version":1,"subject_id":subject,"request_id":request,"roles":{}}),
        );
    if record["sources"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v["message_id"] == message))
    {
        return Ok(false);
    }
    let source = json!({"message_id":message,"text_sha256":sha256(text.as_bytes()),"text":text});
    for (role, v) in roles.as_object().unwrap() {
        let mut v = v.clone();
        v["source"] = json!({"message_id":message,"text_sha256":sha256(text.as_bytes())});
        record["roles"][role] = v;
    }
    let mut sources = record["sources"].as_array().cloned().unwrap_or_default();
    sources.push(source);
    record["sources"] = json!(sources);
    tx.execute("INSERT INTO development_records(kind,id,work_id,record_json) VALUES('request_routing',?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET record_json=excluded.record_json",params![subject,task,encode(&record)?]).map_err(|e|e.to_string())?;
    let (seq, _) = event(
        tx,
        "providers.request_roles_saved",
        task.as_deref(),
        &json!({"request_id":request,"subject_id":subject,"roles":record["roles"],"source_message_id":message,"execution_authorized":false,"project_defaults_changed":false}),
    )?;
    crate::request_history::tag(tx, seq, &request, "human request role choice / service")?;
    Ok(true)
}
pub fn policy(s: &Session, task: &str) -> Result<Value, String> {
    let mut policy = crate::development::policy(s)?;
    let raw:Option<String>=s.workflow.connection()?.query_row("SELECT w.record_json FROM request_history_requests r JOIN development_records w ON w.kind='request_routing' AND w.id=r.subject_id WHERE r.task_id=?1",[task],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if let Some(raw) = raw {
        policy["request_roles"] =
            serde_json::from_str::<Value>(&raw).map_err(|e| e.to_string())?["roles"].clone();
    }
    Ok(policy)
}
/// Explicit stopped-project reconciliation of one retained human directive, never a new turn or approval.
pub fn reconcile(s: &Session, request: &str, message: &str) -> Result<Value, String> {
    let mut c = s.workflow.connection()?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (subject,text):(String,String)=tx.query_row("SELECT r.subject_id,m.text FROM request_history_requests r JOIN dialogue_inputs d USING(subject_id) JOIN conversation_turns t USING(turn_id) JOIN conversation_messages m ON m.message_id=t.message_id WHERE r.request_id=?1 AND m.message_id=?2 AND m.role='user'",params![request,message],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    if choices(s, &text)?.is_null() {
        return Err(
            "Retained input contains no supported explicit request-role choice; no changes".into(),
        );
    }
    let changed = save_source(&tx, s, &subject, message, &text)?;
    tx.commit().map_err(|e| e.to_string())?;
    crate::coordination::refresh(s)?;
    crate::request_history::project(s)?;
    Ok(
        json!({"status":"request_routing_saved","changed":changed,"request_id":request,"source_message_id":message,"approval_submitted":false,"continuation_queued":false}),
    )
}
pub fn requires_contract(s: &Session) -> Result<bool, String> {
    s.workflow
        .connection()?
        .query_row(
            "SELECT exists(SELECT 1 FROM development_records WHERE kind='request_routing')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())
}

pub fn require_review(policy: &Value, unit: &Value, all: &[Value]) -> Result<(), String> {
    if unit["role"]!="review"{crate::review_findings::require_clear(unit,all)?;}
    let pin = &policy["request_roles"]["review"];
    if pin.is_null() || unit["role"] == "review" {
        return Ok(());
    }
    let covered = all.iter().any(|r| {
        r["role"] == "review"
            && r["status"] == "reviewed"
            && r["requirement_base"] == unit["requirement_base"]
            && r["requirement_generation"] == unit["requirement_generation"]
            && r["review_stale"] != true
            && r["selection"]["profile_sha256"] == pin["profile_sha256"]
            && !crate::review_findings::effective_requires_changes(r,all)
            && r["review"]["reviewed_commits"]
                .as_array()
                .is_some_and(|a| a.contains(&unit["result_commit"]))
    });
    if covered {
        Ok(())
    } else {
        Err(format!(
            "Exact submitted commit needs completed {} review with blocking findings resolved before integration/delivery",
            pin["profile_id"].as_str().unwrap_or("selected reviewer")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_roles_preserve_proposal_and_enforce_review_without_narrowing_grants() {
        let (_dir, s) = crate::provider_selection::recovery_tests::fixture();
        let (prefs, rev) = provider_selection::preferences(&s).unwrap();
        let mut p = prefs.value();
        p["roles"]["review"]["allowed"] = json!(["codex", "claude"]);
        provider_selection::save(&s,&json!({"command_id":"setup-role-options","action":"providers.save","payload":{"expected_revision":rev,"preferences":p}})).unwrap();
        let text = "Use Claude as the reviewer for this work, with Codex coordinating and implementing. Update the proposal before I approve it.";
        let sent=s.conversation_submit(&json!({"command_id":"human-roles","action":"submit_message","payload":{"subject_id":"discussion","text":text}})).unwrap();
        let c = s.workflow.connection().unwrap();
        let request: String = c
            .query_row(
                "SELECT request_id FROM request_history_requests WHERE subject_id='discussion'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut writer = s.workflow.connection().unwrap();
        let tx = writer.transaction().unwrap();
        let proposal = crate::scope_contract::Proposal {
            title: "Fixture scope".into(),
            objective: "Exercise saved routing".into(),
            deliverables: vec!["Fixture".into()],
            in_scope: vec!["Local fixture".into()],
            out_of_scope: vec!["Native execution".into()],
            constraints: vec![],
            acceptance_criteria: vec!["Preserve exact proposal".into()],
            unresolved_decisions: vec![],
        };
        let context = crate::task_scope::capture(&tx, &s.key, None).unwrap();
        crate::task_scope::save(
            &tx,
            &s.key,
            "fixture-task",
            &proposal,
            &context,
            json!({"kind":"fixture"}),
            "Routing fixture",
        )
        .unwrap();
        tx.execute(
            "UPDATE dialogue_subjects SET task_id='fixture-task' WHERE subject_id='discussion'",
            [],
        )
        .unwrap();
        crate::request_history::link(&tx, "discussion", "fixture-task").unwrap();
        let before = crate::task_scope::head(&tx, &s.key, "fixture-task").unwrap();
        tx.commit().unwrap();
        let first = reconcile(&s, &request, sent["message_id"].as_str().unwrap()).unwrap();
        assert_eq!(first["changed"], true);
        assert_eq!(
            reconcile(&s, &request, sent["message_id"].as_str().unwrap()).unwrap()["changed"],
            false
        );
        c.execute("UPDATE request_history_requests SET task_id='fixture-task' WHERE subject_id='discussion'",[]).unwrap();
        let mut reader = s.workflow.connection().unwrap();
        let tx = reader.transaction().unwrap();
        assert_eq!(
            crate::task_scope::head(&tx, &s.key, "fixture-task").unwrap(),
            before
        );
        tx.commit().unwrap();
        let policy = policy(&s, "fixture-task").unwrap();
        let (prefs, rev) = provider_selection::preferences(&s).unwrap();
        assert_eq!(prefs.role_default("review"), "codex");
        assert_eq!(
            policy["routing"]["roles"]["review"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let (_, selected) = provider_selection::execution(
            &s,
            "review",
            &Value::Null,
            None,
            &policy,
            "human selected review",
        )
        .unwrap();
        assert_eq!(selected["profile_id"], "claude");
        assert!(
            prefs
                .resolve(
                    "review",
                    &Value::Null,
                    Some("codex"),
                    &policy,
                    rev,
                    "model override"
                )
                .is_err()
        );
        assert!(
            prefs
                .resolve(
                    "review",
                    &json!({"mode":"pin","profiles":["codex"]}),
                    None,
                    &policy,
                    rev,
                    "task override"
                )
                .is_err()
        );
        assert!(
            provider_selection::fallback_choice(&s, "review", &selected, &Value::Null, &policy)
                .unwrap()
                .is_none()
        );
        let (execution, _) = provider_selection::execution(
            &s,
            "review",
            &Value::Null,
            None,
            &policy,
            "review reservation",
        )
        .unwrap();
        let mut packet = json!({"operation_kind":"development-review","provider":execution["provider"],"model":execution["model"],"execution":execution,"policy":policy});
        assert!(
            provider_selection::packet_authorized(&s, &json!({"policy":policy}), &packet).unwrap()
        );
        let mut unpinned = policy.clone();
        unpinned.as_object_mut().unwrap().remove("request_roles");
        let (wrong, _) = provider_selection::execution(
            &s,
            "review",
            &Value::Null,
            Some("codex"),
            &unpinned,
            "wrong review",
        )
        .unwrap();
        packet["provider"] = wrong["provider"].clone();
        packet["model"] = wrong["model"].clone();
        packet["execution"] = wrong;
        assert!(
            !provider_selection::packet_authorized(&s, &json!({"policy":policy}), &packet).unwrap(),
            "Launch permit cannot bypass the request pin"
        );
        let unit = json!({"role":"implementation","result_commit":"exact"});
        assert!(require_review(&policy, &unit, &[]).is_err());
        let mut review = json!({"role":"review","status":"reviewed","selection":selected,"review":{"reviewed_commits":["exact"],"requires_changes":true}});
        assert!(require_review(&policy, &unit, &[review.clone()]).is_err());
        review["review"]["requires_changes"] = json!(false);
        assert!(require_review(&policy, &unit, &[review.clone()]).is_ok());
        review["review"]["reviewed_commits"] = json!(["different"]);
        assert!(require_review(&policy, &unit, &[review]).is_err());
        assert!(s.development_records("workflow").unwrap().is_empty());
        s.conversation_submit(&json!({"command_id":"fresh-scoped-choice","action":"submit_message","payload":{"subject_id":"discussion-next","text":"Use Claude as reviewer for this work.","interaction":{"version":1,"subject_id":"discussion-next","provenance":{"kind":"typed","deliberate_submission":true}}}})).unwrap();
        assert_eq!(
            subject_roles(&s, "discussion-next").unwrap()["review"]["profile_id"],
            "claude"
        );
        // Isolated fixture approval verifies the saved routing reaches the existing authority record.
        // No service/worker is launched, and this fixture conveys no human authorization.
        c.execute("UPDATE conversation_turns SET status='completed'", [])
            .unwrap();
        let head = s.scope_detail("fixture-task").unwrap().unwrap();
        let result=s.scope_command(&json!({"command_id":"fixture-scope-only","action":"authorize_scope","payload":{"base":head["base"],"decision_revision":head["decision_revision"],"authorization_command_id":head["authorization_command_id"],"operating_policy_sha256":canonical_sha256(&head["operating_policy"]).unwrap(),"acknowledged_unresolved_sha256":canonical_sha256(&json!([])).unwrap()}})).unwrap();
        assert_eq!(result["status"], "accepted");
        let approved = s
            .development_record("workflow", "fixture-task")
            .unwrap()
            .unwrap();
        assert_eq!(
            approved["policy"]["request_roles"]["review"]["profile_id"],
            "claude"
        );
        assert_eq!(
            approved["authorization"]["operating_policy"]["request_roles"],
            approved["policy"]["request_roles"]
        );
        assert!(
            choices(&s, "What if we use Claude as reviewer for this work?")
                .unwrap()
                .is_null()
        );
    }
}

pub fn pin_allows(policy: &Value, role: &str, hash: &str, profile: &Value) -> bool {
    let pin = &policy["request_roles"][role];
    pin.is_null() || pin["profile_sha256"] == hash && pin["profile"] == *profile
}

pub fn subject_roles(s: &Session, subject: &str) -> Result<Value, String> {
    let raw: Option<String> = s
        .workflow
        .connection()?
        .query_row(
            "SELECT record_json FROM development_records WHERE kind='request_routing' AND id=?1",
            [subject],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(raw
        .map(|raw| serde_json::from_str::<Value>(&raw).map(|v| v["roles"].clone()))
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or(Value::Null))
}
