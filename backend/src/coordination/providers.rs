//! Chat choices reuse project preferences and the existing narrow grant/switch commands.
use super::*;
use crate::provider_selection as routing;

pub(super) fn prepare(
    tx: &Transaction<'_>,
    s: &Session,
    subject: &str,
    text: &str,
    context: &Value,
) -> Result<Option<Value>, String> {
    let t = normalized(text);
    let (prefs, revision) = routing::preferences(s)?;
    if matches!(
        t.as_str(),
        "allow provider" | "allow this provider" | "approve provider change"
    ) {
        let pending:Option<String>=tx.query_row("SELECT record_json FROM coordination_inputs WHERE subject_id=?1 AND json_extract(record_json,'$.status')='awaiting_grant' ORDER BY rowid DESC LIMIT 1",[subject],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let Some(raw) = pending else { return Ok(None) };
        let old = decode(raw)?;
        let card = &old["provider_card"];
        if context["provider_card_id"] != card["card_id"] {
            return Err("The provider grant card changed; review the current destination".into());
        }
        let change = &old["provider_change"];
        let mut payload = card["grant_payload"].clone();
        payload["subject_id"] = json!(subject);
        return Ok(Some(
            json!({"route":if card["grant_kind"]=="context"{"providers.switch"}else{"providers.approve_policy"},"payload":payload,"provider_change":change,"provider_grant_for":old["command_id"]}),
        ));
    }
    let words = t.split_whitespace().collect::<Vec<_>>();
    let (mode, role, wanted) = match words.as_slice() {
        ["use", name, "for", role] if matches!(*role, "review" | "implementation") => {
            ("after_current_turn", *role, *name)
        }
        ["use", name, "as", "coordinator"]
        | ["use", name, "for", "coordination"]
        | ["switch", "coordinator", "to", name] => ("after_current_turn", "coordinator", *name),
        ["interrupt", "and", "switch", "coordinator", "to", name] => {
            ("interrupt", "coordinator", *name)
        }
        _ => return Ok(None),
    };
    let names = prefs
        .profiles
        .iter()
        .filter(|(name, p)| name.as_str() == wanted || p.provider == wanted)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if names.len() != 1 {
        return Err(
            "Choose one exact configured profile for this role in F3, or name it in chat".into(),
        );
    }
    let name = &names[0];
    let profile = prefs.profile_value(name)?;
    let mut next = prefs.value();
    let allowed = next["roles"][role]["allowed"]
        .as_array_mut()
        .ok_or("Role preferences are missing")?;
    if !allowed.contains(&json!(name)) {
        allowed.push(json!(name));
    }
    next["roles"][role]["default"] = json!(name);
    if role == "coordinator" {
        next["coordinator"] = json!(name)
    }
    let binding = json!(
        s.workflow
            .connection()?
            .query_row("SELECT generation FROM conversation_bindings", [], |r| r
                .get::<_, i64>(0))
            .map_err(|e| e.to_string())?
    );
    Ok(Some(
        json!({"route":"providers.save","payload":{"expected_revision":revision,"preferences":next,"subject_id":subject},"provider_change":{"role":role,"profile_id":name,"profile":profile,"profile_sha256":hash(&profile)?,"mode":mode,"expected_generation":binding,"subject_id":subject}}),
    ))
}
fn switch_payload(
    s: &Session,
    change: &Value,
    revision: u64,
    grant: bool,
) -> Result<Value, String> {
    let generation = s
        .workflow
        .connection()?
        .query_row("SELECT generation FROM conversation_bindings", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())?;
    Ok(
        json!({"expected_revision":revision,"expected_generation":change["expected_generation"].as_i64().unwrap_or(generation),"profile_id":change["profile_id"],"mode":change["mode"],"grant_context":grant,"subject_id":change["subject_id"]}),
    )
}
pub(super) fn after(
    tx: &Transaction<'_>,
    s: &Session,
    input: &mut Value,
    out: &Value,
) -> Result<(), String> {
    if out["status"] != "accepted" || input["provider_change"].is_null() {
        return Ok(());
    }
    let change = input["provider_change"].clone();
    let subject = input["subject_id"].as_str().unwrap().to_owned();
    let (prefs, revision) = routing::preferences(s)?;
    let role = change["role"].as_str().ok_or("Missing provider role")?;
    let name = change["profile_id"]
        .as_str()
        .ok_or("Missing provider profile")?;
    if prefs.profile_value(name)? != change["profile"]
        || prefs.roles.get(role).map(|r| r.default.as_str()) != Some(name)
    {
        return Err(
            "Preferences changed during the provider choice; inspect F3 before another choice"
                .into(),
        );
    }
    let input_id = input["command_id"].as_str().unwrap().to_owned();
    let next_id = |label: &str| format!("choice-{label}-{}", &sha256(input_id.as_bytes())[..24]);
    if input["route"] == "providers.save" {
        let active:Option<String>=tx.query_row("SELECT w.record_json FROM dialogue_subjects d JOIN development_records w ON w.kind='workflow' AND w.id=d.task_id WHERE d.subject_id=?1 AND json_extract(w.record_json,'$.status') NOT IN ('delivered','cancelled')",[&subject],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let work = active.map(decode).transpose()?;
        let hash_ = change["profile_sha256"].as_str().unwrap();
        let approved = work.as_ref().is_some_and(|w| {
            routing::normalized_policy(s, &w["policy"])
                .is_ok_and(|p| routing::policy_allows(&p, role, hash_, &change["profile"]))
        });
        let grants: Option<String> = tx
            .query_row(
                "SELECT value FROM session_metadata WHERE name='provider_context_grants'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let context_allowed = approved
            || grants.map(decode).transpose()?.is_some_and(|g| {
                g.as_array()
                    .is_some_and(|a| a.contains(&change["profile_sha256"]))
            });
        let need_policy = work.is_some() && !approved;
        let need_context = role == "coordinator" && !context_allowed;
        if need_policy || need_context {
            let grant = if let Some(w) = work.as_ref().filter(|_| need_policy) {
                let routing = routing::narrow_routing(&prefs, role, name)?;
                json!({"kind":"policy","payload":{"expected_revision":revision,"work_id":w["work_id"],"policy_sha256":hash(&w["policy"])?,"routing_sha256":hash(&routing)?,"grant_role":role,"grant_profile":name}})
            } else {
                json!({"kind":"context","payload":switch_payload(s,&change,revision,true)?})
            };
            let mut card = json!({"subject_id":subject,"role":role,"profile_id":name,"profile":change["profile"],"preferences_revision":revision,"grant_kind":grant["kind"],"grant_payload":grant["payload"],"scope_unchanged":true,"explanation":"Allow this exact destination/role in the current project context. Existing work and approvals remain; no other provider/role or fallback is added."});
            card["card_id"] = json!(hash(&card)?);
            input["provider_card"] = card.clone();
            input["status"] = json!("awaiting_grant");
            tagged(tx, "conversation.provider_grant", &subject, &card)?;
            return Ok(());
        }
        if role == "coordinator" {
            input["route"] = json!("providers.switch");
            input["effect"] = json!({"command_id":next_id("switch"),"action":"providers.switch","payload":switch_payload(s,&change,revision,false)?});
            input["status"] = json!("pending");
        }
    } else if input["route"] == "providers.approve_policy" && role == "coordinator" {
        input["route"] = json!("providers.switch");
        input["effect"] = json!({"command_id":next_id("switch"),"action":"providers.switch","payload":switch_payload(s,&change,revision,false)?});
        input["status"] = json!("pending");
    }
    if let Some(old) = input["provider_grant_for"].as_str() {
        if let Some(mut original) = record(tx, "coordination_inputs", old)? {
            original["status"] = json!("granted");
            tx.execute(
                "UPDATE coordination_inputs SET record_json=?1 WHERE command_id=?2",
                params![encode(&original)?, old],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tagged(
        tx,
        "conversation.provider_choice",
        &subject,
        &json!({"role":role,"profile_id":name,"profile":change["profile"],"status":input["status"],"future_attempts_only":true,"switch_mode":change["mode"]}),
    )
}

/// A later exact scope approval can already contain the pending destination. Reuse it once;
/// do not ask again or treat preference files/model prose as authority.
pub(super) fn reconcile_grants(s: &Session) -> Result<(), String> {
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let rows = {
        let mut q=tx.prepare("SELECT record_json FROM coordination_inputs WHERE json_extract(record_json,'$.status')='awaiting_grant' ORDER BY rowid LIMIT 16").map_err(|e|e.to_string())?;
        q.query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    for raw in rows {
        let mut v = decode(raw)?;
        let change = v["provider_change"].clone();
        let subject = v["subject_id"].as_str().unwrap().to_owned();
        let work:Option<String>=tx.query_row("SELECT w.record_json FROM dialogue_subjects d JOIN development_records w ON w.kind='workflow' AND w.id=d.task_id WHERE d.subject_id=?1 AND json_extract(w.record_json,'$.status') NOT IN ('delivered','cancelled')",[&subject],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let Some(work) = work else { continue };
        let work = decode(work)?;
        let role = change["role"].as_str().unwrap();
        if !routing::normalized_policy(s, &work["policy"]).is_ok_and(|p| {
            routing::policy_allows(
                &p,
                role,
                change["profile_sha256"].as_str().unwrap(),
                &change["profile"],
            )
        }) {
            continue;
        }
        let (prefs, revision) = routing::preferences(s)?;
        let name = change["profile_id"].as_str().unwrap();
        if prefs.profile_value(name)? != change["profile"]
            || prefs.roles.get(role).map(|r| r.default.as_str()) != Some(name)
        {
            continue;
        }
        if role == "coordinator" {
            v["route"] = json!("providers.switch");
            v["effect"] = json!({"command_id":format!("scope-choice-{}",&sha256(v["command_id"].as_str().unwrap().as_bytes())[..24]),"action":"providers.switch","payload":switch_payload(s,&change,revision,false)?});
            v["status"] = json!("pending");
        } else {
            v["status"] = json!("granted");
        }
        tagged(
            &tx,
            "conversation.provider_grant_satisfied",
            &subject,
            &json!({"profile_id":name,"role":role,"source":"existing exact approved request policy","scope_base":work["scope_base"],"no_repeat_approval":true}),
        )?;
        tx.execute(
            "UPDATE coordination_inputs SET record_json=?1 WHERE command_id=?2",
            params![encode(&v)?, v["command_id"].as_str()],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}
