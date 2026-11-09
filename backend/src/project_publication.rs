//! Frozen GitHub publication intent. All retries reconcile exact remote/PR identity.
use crate::session_store::Session;
use serde_json::{Value, json};
use std::process::Command;
fn run(s: &Session, program: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new(program)
        .args(args)
        .current_dir(&s.project)
        .env_remove("GH_DEBUG")
        .env_remove("GIT_TRACE")
        .env_remove("GIT_CURL_VERBOSE")
        .output()
        .map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!(
            "{program} operation refused: {}",
            String::from_utf8_lossy(&o.stderr)
                .chars()
                .take(1500)
                .collect::<String>()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().into())
}
pub fn tick(s: &Session, w: &mut Value, e: &mut Value) -> Result<bool, String> {
    tick_using(s, w, e, &|program, args| run(s, program, args))
}
fn tick_using(
    s: &Session,
    w: &mut Value,
    e: &mut Value,
    call: &dyn Fn(&str, &[&str]) -> Result<String, String>,
) -> Result<bool, String> {
    let gh = |args: &[&str]| -> Result<Value, String> {
        serde_json::from_str(&call("gh", args)?).map_err(|e| e.to_string())
    };
    let frozen_w = w.clone();
    let frozen_e = e.clone();
    let p = &frozen_w["policy"]["project_workflow"];
    let repo = p["repository"]
        .as_str()
        .ok_or("Missing approved GitHub repository")?;
    let remote = p["remote"].as_str().ok_or("Missing approved Git remote")?;
    let base = p["target"].as_str().ok_or("Missing approved PR base")?;
    let head = frozen_w["integration_branch"]
        .as_str()
        .ok_or("Missing governing branch")?;
    let sha = frozen_e["submitted_commit"]
        .as_str()
        .ok_or("Missing publication SHA")?;
    if w["head"] != sha || w["checks"]["commit"] != sha || w["checks"]["result"]["passed"] != true {
        return Err("Publication must use the exact checked candidate".into());
    }
    let url = call("git", &["remote", "get-url", remote])?;
    let accepted = [
        format!("https://github.com/{repo}"),
        format!("https://github.com/{repo}.git"),
        format!("git@github.com:{repo}.git"),
        format!("ssh://git@github.com/{repo}.git"),
    ];
    if !accepted.contains(&url) {
        return Err(
            "Approved repository does not match the selected remote; no publication".into(),
        );
    }
    if e["publication"].is_null() {
        e["publication"] = json!({"repository":repo,"remote":remote,"base":base,"branch":head,"head":sha,"merge_method":p["merge_method"],"marker":format!("Waywr request: {}",w["work_id"].as_str().unwrap_or("")),"phase":"prepared"});
        s.development_save(
            "effect",
            e["effect_id"].as_str().unwrap(),
            e,
            "development.publication_intent",
        )?;
    }
    let saved = &e["publication"];
    if saved["repository"] != repo || saved["branch"] != head || saved["head"] != sha {
        return Err("Publication identity changed; preserve and reconcile".into());
    }
    let prs = gh(&[
        "pr",
        "list",
        "--repo",
        repo,
        "--head",
        head,
        "--base",
        base,
        "--state",
        "all",
        "--json",
        "number,headRefOid,body,url,state,mergeCommit",
    ])?;
    let matches = prs
        .as_array()
        .ok_or("Invalid PR listing")?
        .iter()
        .filter(|pr| {
            pr["headRefOid"] == sha
                && pr["body"]
                    .as_str()
                    .is_some_and(|b| b.contains(e["publication"]["marker"].as_str().unwrap()))
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err("Multiple matching publication records; reconcile explicitly".into());
    }
    if let Some(pr) = matches.first().filter(|p| p["state"] == "MERGED") {
        return complete(s, w, e, pr, sha);
    }
    let refname = format!("refs/heads/{head}");
    let observed = call("git", &["ls-remote", "--heads", remote, &refname])?;
    if !observed.is_empty() && !observed.starts_with(&format!("{sha}\t")) {
        return Err("Remote governing branch differs from approved candidate; no overwrite".into());
    }
    if observed.is_empty() {
        if e["publication"]["push_started"] == true {
            return Err(
                "Prior push has no matching remote ref; reconcile before another publication"
                    .into(),
            );
        }
        e["publication"]["push_started"] = json!(true);
        s.development_save(
            "effect",
            e["effect_id"].as_str().unwrap(),
            e,
            "development.publication_intent",
        )?;
        call("git", &["push", remote, &format!("{sha}:{refname}")])?;
    }
    let pr = if matches.len() == 1 {
        matches[0].clone()
    } else if !matches.is_empty() {
        return Err("Multiple matching publication records; reconcile explicitly".into());
    } else {
        if e["publication"]["create_started"] == true {
            return Err("PR create response not reconciled; no duplicate creation".into());
        }
        let body = s
            .workflow
            .state
            .join("development")
            .join(w["work_id"].as_str().unwrap())
            .join("publication-body.md");
        std::fs::create_dir_all(body.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&body,format!("{}\n\nExact candidate: `{sha}`\n\n{}\n\nChecks and review receipts retained in project history.\n",e["publication"]["marker"].as_str().unwrap(),w["summary"]["text"].as_str().unwrap_or("Coordinator delivery"))).map_err(|e|e.to_string())?;
        e["publication"]["create_started"] = json!(true);
        s.development_save(
            "effect",
            e["effect_id"].as_str().unwrap(),
            e,
            "development.publication_intent",
        )?;
        call(
            "gh",
            &[
                "pr",
                "create",
                "--repo",
                repo,
                "--head",
                head,
                "--base",
                base,
                "--title",
                &format!("Waywr: {}", w["work_id"].as_str().unwrap()),
                "--body-file",
                body.to_str().ok_or("Invalid body path")?,
            ],
        )?;
        return Ok(false);
    };
    e["publication"]["pr"] = pr.clone();
    if pr["state"] != "OPEN" {
        return Err(
            "PR was closed without merge; publication requires a substantive decision".into(),
        );
    }
    let number = pr["number"].to_string();
    let status = gh(&[
        "pr",
        "view",
        &number,
        "--repo",
        repo,
        "--json",
        "headRefOid,mergeStateStatus,reviewDecision,statusCheckRollup",
    ])?;
    if status["headRefOid"] != sha {
        return Err("PR head changed; frozen candidate no longer matches".into());
    }
    let checks_ok = status["statusCheckRollup"].as_array().is_some_and(|a| {
        a.iter().all(|c| {
            matches!(
                c["conclusion"].as_str().or(c["state"].as_str()),
                Some("SUCCESS" | "NEUTRAL" | "SKIPPED")
            )
        })
    });
    if !checks_ok
        || !matches!(
            status["mergeStateStatus"].as_str(),
            Some("CLEAN" | "HAS_HOOKS")
        )
        || status["reviewDecision"] == "REVIEW_REQUIRED"
        || status["reviewDecision"] == "CHANGES_REQUESTED"
    {
        e["publication"]["phase"] = json!("waiting-checks-or-review");
        e["publication"]["status"] = status;
        e["retry_at"] = json!(crate::execution::now() + 30.0);
        s.development_save(
            "effect",
            e["effect_id"].as_str().unwrap(),
            e,
            "development.publication_wait",
        )?;
        return Ok(false);
    }
    if e["publication"]["merge_started"] == true {
        return Err(
            "Merge response remains unconfirmed; query the saved PR before replacement".into(),
        );
    }
    let method = format!(
        "--{}",
        p["merge_method"].as_str().ok_or("Missing merge method")?
    );
    e["publication"]["merge_started"] = json!(true);
    s.development_save(
        "effect",
        e["effect_id"].as_str().unwrap(),
        e,
        "development.publication_intent",
    )?;
    call(
        "gh",
        &[
            "pr",
            "merge",
            &number,
            "--repo",
            repo,
            &method,
            "--match-head-commit",
            sha,
        ],
    )?;
    e["retry_at"] = json!(crate::execution::now() + 5.0);
    Ok(false)
}
fn complete(
    s: &Session,
    w: &mut Value,
    e: &mut Value,
    pr: &Value,
    sha: &str,
) -> Result<bool, String> {
    if pr["mergeCommit"]["oid"]
        .as_str()
        .is_none_or(|v| v.len() != 40 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("Merged PR lacks an exact merge commit; retain publication evidence".into());
    }
    e["publication"]["pr"] = pr.clone();
    e["status"] = json!("recorded");
    e["result_commit"] = pr["mergeCommit"]["oid"].clone();
    w["status"] = json!("delivered");
    w["delivery"] = json!({"kind":"github-pr","candidate_commit":sha,"merge_commit":pr["mergeCommit"]["oid"],"url":pr["url"],"local_target":"Not advanced; remote delivery confirmed"});
    record_complete(s, w, e)?;
    Ok(true)
}
fn record_complete(s: &Session, w: &Value, e: &Value) -> Result<(), String> {
    if let Some(writer) = &s.workflow.writer {
        let (w, e) = (w.clone(), e.clone());
        return writer.call(move |s| record_complete(s, &w, &e));
    }
    let mut connection = s.workflow.connection()?;
    let tx = connection.transaction().map_err(|e| e.to_string())?;
    let raw: String = tx
        .query_row(
            "SELECT record_json FROM development_records WHERE kind='workflow' AND id=?1",
            [w["work_id"].as_str()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if current["policy"] != w["policy"]
        || current["requirement_generation"] != w["requirement_generation"]
        || matches!(current["status"].as_str(), Some("cancelled" | "cancelling"))
    {
        return Err("Publication completion authority changed; exact merged evidence retained for reconciliation".into());
    }
    for (kind, id, value, event_kind) in [
        (
            "workflow",
            w["work_id"].as_str(),
            w,
            "development.delivered",
        ),
        (
            "effect",
            e["effect_id"].as_str(),
            e,
            "development.publication_complete",
        ),
    ] {
        tx.execute(
            "UPDATE development_records SET record_json=?1 WHERE kind=?2 AND id=?3",
            rusqlite::params![crate::conversation_store::encode(value)?, kind, id],
        )
        .map_err(|e| e.to_string())?;
        let (seq, _) =
            crate::conversation_store::event(&tx, event_kind, w["work_id"].as_str(), value)?;
        if let Some(request) = w["request_id"].as_str() {
            crate::request_history::tag(&tx, seq, request, "service / exact publication receipt")?;
        }
    }
    tx.execute("UPDATE session_tasks SET status='approved',state_revision=state_revision+1 WHERE task_id=?1 AND status!='approved'",[w["work_id"].as_str()]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    #[test]
    fn ordinary_publication_waits_for_gates_and_records_exact_merge_without_calls() {
        let (_d, s) = crate::provider_selection::recovery_tests::fixture();
        s.development_schema().unwrap();
        let sha = "a".repeat(40);
        let merged = "b".repeat(40);
        let mut w = json!({"work_id":"t-publication","request_id":null,"requirement_generation":0,"status":"running","head":sha,"integration_branch":"waywr/general-request/integration","checks":{"commit":sha,"result":{"passed":true,"checks":[{"success":true}]}},"policy":{"project_workflow":{"completion":"github-pr","repository":"owner/repo","remote":"origin","target":"trunk","merge_method":"squash"}}});
        s.development_save("workflow", "t-publication", &w, "fixture")
            .unwrap();
        let mut e = json!({"effect_id":"publish-exact","work_id":w["work_id"],"submitted_commit":sha,"status":"intent"});
        let phase = Cell::new(0);
        let calls = RefCell::new(Vec::new());
        let fake = |program: &str, args: &[&str]| -> Result<String, String> {
            calls
                .borrow_mut()
                .push(format!("{program} {}", args.join(" ")));
            assert!(!args.contains(&"--admin") && !args.contains(&"--force"));
            match (program,args[0],args.get(1).copied().unwrap_or("")){
 ("git","remote",_)=>Ok("https://github.com/owner/repo.git".into()),("git","ls-remote",_)=>{assert_ne!(phase.get(),2,"Confirm merged PR before probing its auto-deleted branch");Ok(format!("{sha}\trefs/heads/waywr/general-request/integration"))},
 ("gh","pr","list")=>Ok(json!([{"number":7,"headRefOid":sha,"body":"Waywr request: t-publication","url":"https://github.com/owner/repo/pull/7","state":if phase.get()==2{"MERGED"}else{"OPEN"},"mergeCommit":{"oid":merged}}]).to_string()),
 ("gh","pr","view")=>Ok(json!({"headRefOid":sha,"mergeStateStatus":if phase.get()==0{"BLOCKED"}else{"CLEAN"},"reviewDecision":if phase.get()==0{"REVIEW_REQUIRED"}else{"APPROVED"},"statusCheckRollup":[{"conclusion":"SUCCESS"}]}).to_string()),
 ("gh","pr","merge")=>{assert!(args.contains(&"--match-head-commit"));assert!(args.contains(&sha.as_str()));Ok("".into())},_=>panic!("Unexpected external action")}
        };
        assert!(!tick_using(&s, &mut w, &mut e, &fake).unwrap());
        assert_eq!(e["publication"]["phase"], "waiting-checks-or-review");
        assert_eq!(w["status"], "running");
        assert!(!calls.borrow().iter().any(|v| v.contains("pr merge")));
        phase.set(1);
        assert!(!tick_using(&s, &mut w, &mut e, &fake).unwrap());
        assert_eq!(e["publication"]["merge_started"], true);
        phase.set(2);
        assert!(tick_using(&s, &mut w, &mut e, &fake).unwrap());
        assert_eq!(w["status"], "delivered");
        assert_eq!(w["delivery"]["candidate_commit"], sha);
        assert_eq!(w["delivery"]["merge_commit"], merged);
        assert_eq!(
            calls
                .borrow()
                .iter()
                .filter(|v| v.contains("pr merge"))
                .count(),
            1
        );
    }
}
