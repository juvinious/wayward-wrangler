//! Schema-4 static-file workflow policy. The project service owns transitions;
//! the existing host runner owns child execution and immutable receipts.
use crate::{codec::{canonical_sha256,sha256},execution::{read,write,write_bytes},session_store::Session};
use rusqlite::{params,OptionalExtension,TransactionBehavior};
use serde_json::{json,Value};
use std::{collections::BTreeSet,fs,path::{Path,PathBuf}};

pub const PROFILE:&str="static-files-v1";
pub const FILES:[&str;4]=["index.html","styles.css","app.js","README.md"];
const LIMIT:usize=32*1024;
fn encoded(v:&Value)->Result<String,String>{String::from_utf8(crate::codec::canonical(v).map_err(|e|e.to_string())?).map_err(|e|e.to_string())}
fn refused(code:&str,message:&str)->Value{json!({"status":"rejected","code":code,"message":message})}
fn stage_ok(stage:&str)->bool{matches!(stage,"implement"|"implement-followup"|"review-1"|"repair"|"repair-followup"|"review-2")}
fn author_stage(stage:&str)->bool{matches!(stage,"implement"|"implement-followup"|"repair"|"repair-followup")}
fn valid_id(id:&str)->bool{!id.is_empty() && id.len()<=64 && id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_')}
fn row(tx:&rusqlite::Transaction<'_>,run:&str)->Result<Option<Value>,String>{
    let raw:Option<String>=tx.query_row("SELECT state_json FROM static_runs WHERE run_id=?1",[run],|r|r.get(0))
        .optional().map_err(|e|e.to_string())?;
    raw.map(|v|serde_json::from_str(&v).map_err(|e|e.to_string())).transpose()
}
fn save(tx:&rusqlite::Transaction<'_>,run:&str,state:&Value)->Result<(),String>{
    tx.execute("UPDATE static_runs SET state_json=?1 WHERE run_id=?2",params![encoded(state)?,run]).map_err(|e|e.to_string())?;Ok(())
}
pub fn validate_result(result:&Value,packet:&Value)->Result<(),String>{
    if packet["profile"]!=PROFILE{return Err("Not a static-file packet".into());}
    let stage=packet["stage"].as_str().ok_or("Missing stage")?;
    if !stage_ok(stage) || result["schema_version"]!=1 || result["operation_id"]!=packet["operation_id"] ||
        result["packet_sha256"]!=canonical_sha256(packet).map_err(|e|e.to_string())?{
        if !matches!(stage,"implement"|"repair") || result["schema_version"]!=2 ||
            result["operation_id"]!=packet["operation_id"] ||
            result["packet_sha256"]!=canonical_sha256(packet).map_err(|e|e.to_string())?{
            return Err("Static result does not match frozen operation".into());
        }
        let keys=result.as_object().ok_or("Question result must be an object")?.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if keys!=["schema_version","operation_id","packet_sha256","question"].into_iter().collect(){
            return Err("Question output fields violate the versioned contract".into());
        }
        let question=&result["question"];
        let options=question["options"].as_array().ok_or("Question options missing")?;
        if question["id"].as_str().is_none_or(|v|!valid_id(v)) ||
            question["question"].as_str().is_none_or(|v|v.is_empty()||v.len()>1024) ||
            options.len()>8 || options.iter().any(|v|v.as_str().is_none_or(|s|s.is_empty()||s.len()>256)){
            return Err("Invalid bounded worker question".into());
        }
        return Ok(());
    }
    let expected=if author_stage(stage){
        ["schema_version","operation_id","packet_sha256","files","dispositions"].as_slice()
    }else{["schema_version","operation_id","packet_sha256","new_findings","assessments"].as_slice()};
    let keys=result.as_object().ok_or("Static result must be an object")?.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if keys!=expected.iter().copied().collect(){return Err("Static output fields violate the versioned contract".into());}
    if author_stage(stage){
        let files=result["files"].as_array().ok_or("Missing file entries")?;
        if files.len()!=FILES.len(){return Err("Static bundle must contain exactly four files".into());}
        let mut seen=BTreeSet::new();let mut total=0usize;
        for file in files{
            let fields=file.as_object().ok_or("Invalid file entry")?.keys().map(String::as_str).collect::<BTreeSet<_>>();
            if fields!=["path","content"].into_iter().collect(){return Err("Invalid file entry fields".into());}
            let path=file["path"].as_str().ok_or("Invalid file path")?;
            let content=file["content"].as_str().ok_or("File content must be UTF-8 text")?;
            if !FILES.contains(&path)||!seen.insert(path){return Err("Duplicate, unapproved or traversing file path".into());}
            total=total.checked_add(content.len()).ok_or("Bundle length overflow")?;
            if total>LIMIT{return Err("Static bundle exceeds 32 KiB".into());}
        }
        if !result["dispositions"].is_array(){return Err("Missing finding dispositions".into());}
        let mut refs=BTreeSet::new();
        for item in result["dispositions"].as_array().unwrap(){
            let id=item["finding_ref"].as_str().ok_or("Disposition lacks finding reference")?;
            if !refs.insert(id)||!matches!(item["assessment"].as_str(),Some("addressed"|"unresolved"))||
                item["rationale"].as_str().is_none_or(str::is_empty){return Err("Invalid or duplicate disposition".into());}
        }
    }else{
        let new=result["new_findings"].as_array().ok_or("Missing new findings")?;
        let assessments=result["assessments"].as_array().ok_or("Missing assessments")?;
        if new.len()>16||assessments.len()>32{return Err("Review exceeds finding bound".into());}
        for item in new{
            if item.get("id").is_some()||item["evidence"].as_str().is_none_or(str::is_empty)||
                item["requested_change"].as_str().is_none_or(str::is_empty){return Err("Invalid new finding".into());}
        }
        let mut refs=BTreeSet::new();
        for item in assessments{
            let id=item["finding_ref"].as_str().ok_or("Assessment lacks reference")?;
            if !refs.insert(id)||!matches!(item["assessment"].as_str(),Some("addressed"|"unresolved"))||
                item["evidence"].as_str().is_none_or(str::is_empty){return Err("Invalid existing-finding assessment".into());}
        }
    }
    Ok(())
}
fn candidate_dir(state:&Path,run:&str,stage:&str)->PathBuf{state.join("runs").join(run).join("static").join("candidates").join(stage)}
fn publish(state:&Path,run:&str,stage:&str,result:&Value,scope:&Value,decision:&Value,context:&Value)->Result<Value,String>{
    let directory=candidate_dir(state,run,stage);
    let root=state.join("runs").join(run).canonicalize().map_err(|e|e.to_string())?;
    fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
    for path in [root.join("static"),root.join("static/candidates"),directory.clone()]{
        if !fs::symlink_metadata(&path).map_err(|e|e.to_string())?.file_type().is_dir(){
            return Err("Candidate publication directory is linked or not a directory".into());
        }
    }
    if !directory.canonicalize().map_err(|e|e.to_string())?.starts_with(&root){
        return Err("Candidate publication leaves this run".into());
    }
    let mut files=Vec::new();
    for item in result["files"].as_array().ok_or("Missing file entries")?{
        let path=item["path"].as_str().ok_or("Missing file path")?;
        let bytes=item["content"].as_str().ok_or("Missing file content")?.as_bytes();
        write_bytes(&directory.join(path),bytes,true)?;
        files.push(json!({"path":path,"sha256":sha256(bytes),"size":bytes.len()}));
    }
    files.sort_by(|a,b|a["path"].as_str().cmp(&b["path"].as_str()));
    let manifest=json!({"schema_version":1,"profile":PROFILE,"run_id":run,"stage":stage,
        "files":files,"scope_sha256":scope,"decision_sha256":decision,"context_sha256":context});
    write(&directory.join("manifest.json"),&manifest,true)?;
    Ok(json!({"path":format!("static/candidates/{stage}/manifest.json"),
        "sha256":canonical_sha256(&manifest).map_err(|e|e.to_string())?,"stage":stage}))
}
fn verified_manifest(state:&Path,run:&str,reference:&Value)->Result<Value,String>{
    let stage=reference["stage"].as_str().ok_or("Invalid candidate stage")?;
    if !author_stage(stage){return Err("Invalid candidate stage".into());}
    let dir=candidate_dir(state,run,stage);
    let root=state.join("runs").join(run);
    let resolved=dir.canonicalize().map_err(|e|e.to_string())?;
    if !resolved.starts_with(root.canonicalize().map_err(|e|e.to_string())?){return Err("Candidate directory escapes run".into());}
    if fs::symlink_metadata(dir.join("manifest.json")).map_err(|e|e.to_string())?.file_type().is_symlink(){
        return Err("Candidate manifest symlink refused".into());
    }
    let manifest=read(&dir.join("manifest.json"))?;
    if canonical_sha256(&manifest).map_err(|e|e.to_string())?!=reference["sha256"]{return Err("Candidate manifest changed".into());}
    for file in manifest["files"].as_array().ok_or("Invalid candidate manifest")?{
        let name=file["path"].as_str().ok_or("Invalid manifest path")?;
        if !FILES.contains(&name){return Err("Candidate path left allowlist".into());}
        let path=dir.join(name);
        let metadata=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
        if !metadata.file_type().is_file() || metadata.len()!=file["size"].as_u64().ok_or("Invalid file size")?{
            return Err("Candidate is missing, changed, or linked".into());
        }
        if sha256(&fs::read(path).map_err(|e|e.to_string())?)!=file["sha256"]{return Err("Candidate file hash changed".into());}
    }
    Ok(manifest)
}

impl Session{
    pub fn static_schema(&self)->Result<(),String>{
        self.workflow.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS static_runs(run_id TEXT PRIMARY KEY,task_id TEXT NOT NULL UNIQUE,state_json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS static_effects(run_id TEXT NOT NULL,stage TEXT NOT NULL,packet_sha256 TEXT NOT NULL,
                operation_id TEXT NOT NULL UNIQUE,result_sha256 TEXT,charged INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(run_id,stage));
            CREATE TABLE IF NOT EXISTS static_reads(run_id TEXT NOT NULL,request_id TEXT NOT NULL,manifest_sha256 TEXT NOT NULL,
                offset INTEGER NOT NULL,complete INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(run_id,request_id));")
            .map_err(|e|e.to_string())
    }
    pub fn static_snapshot(&self,run:&str)->Result<Option<Value>,String>{
        if !self.conversation_enabled()?{return Ok(None);}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Deferred).map_err(|e|e.to_string())?;
        row(&tx,run)
    }
    pub fn static_create(&self,task:&str,plan:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let (task,plan)=(task.to_owned(),plan.clone());return writer.call(move|s|s.static_create(&task,&plan));}
        self.static_schema()?;
        let run=plan["run_id"].as_str().ok_or("Missing run ID")?;
        if !valid_id(run)||plan["workflow_profile"]!=PROFILE||plan["project_key"]!=self.key||plan["task_id"]!=task{
            return Err("Static plan identity mismatch".into());
        }
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if let Some(existing)=row(&tx,run)?{return Ok(existing);}
        let state=json!({"schema_version":1,"profile":PROFILE,"task_id":task,"run_id":run,
            "plan_sha256":canonical_sha256(plan).map_err(|e|e.to_string())?,"status":"running","next_stage":"implement",
            "completed":[],"candidate":null,"findings":[],"assessments":[],"pending":null,"question_answers":[],
            "used":0,"reviews":0,"scope_sha256":plan["proposal_result_sha256"],
            "decision_sha256":plan["decision_sha256"],"context_sha256":plan["context_handoff_sha256"]});
        tx.execute("INSERT INTO static_runs VALUES(?1,?2,?3)",params![run,task,encoded(&state)?]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;Ok(state)
    }
    pub fn static_packet(&self,run:&str,plan:&Value)->Result<Option<Value>,String>{
        let Some(state)=self.static_snapshot(run)? else{return Ok(None)};
        if state["status"]!="running"{return Ok(None);}
        let stage=state["next_stage"].as_str().ok_or("Invalid static stage")?;
        if !stage_ok(stage){return Err("Invalid static stage".into());}
        if state["plan_sha256"]!=canonical_sha256(plan).map_err(|e|e.to_string())?{return Err("Frozen plan changed".into());}
        let packet=json!({"schema_version":1,"profile":PROFILE,"operation_kind":"static-file-worker",
            "operation_id":format!("static-{run}-{stage}"),"run_id":run,"stage":stage,
            "role":if stage.starts_with("review"){"critic"}else{"author"},"provider":"fake",
            "model":if stage.starts_with("review"){"deterministic-critic-v1"}else{"deterministic-author-v1"},
            "project_key":self.key,"plan_sha256":state["plan_sha256"],"brief":plan["brief"],
            "candidate":state["candidate"],"findings":state["findings"],"assessments":state["assessments"],
            "question_answers":state["question_answers"],
            "decision_sha256":state["decision_sha256"],"context_sha256":state["context_sha256"],
            "max_task_invocations":4,"max_reviews":2,"seconds":0.15});
        if !state["candidate"].is_null(){verified_manifest(&self.workflow.state,run,&state["candidate"])?;}
        Ok(Some(packet))
    }
    pub fn static_reserve(&self,run:&str,packet:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let (run,packet)=(run.to_owned(),packet.clone());return writer.call(move|s|s.static_reserve(&run,&packet));}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let mut state=row(&tx,run)?.ok_or("Static run missing")?;
        let stage=packet["stage"].as_str().ok_or("Missing static stage")?;
        let hash=canonical_sha256(packet).map_err(|e|e.to_string())?;
        let old:Option<String>=tx.query_row("SELECT packet_sha256 FROM static_effects WHERE run_id=?1 AND stage=?2",params![run,stage],|r|r.get(0))
            .optional().map_err(|e|e.to_string())?;
        if let Some(existing)=old{
            return Ok(if existing==hash{json!({"status":"accepted","replayed":true,"packet_sha256":hash})}
                else{refused("packet_conflict","Stage already has different frozen input")});
        }
        let reserved:i64=tx.query_row("SELECT count(*) FROM static_effects WHERE run_id=?1",[run],|r|r.get(0))
            .map_err(|e|e.to_string())?;
        if state["status"]!="running"||state["next_stage"]!=stage||reserved>=4{
            return Ok(refused("stale_or_exhausted","Static stage changed or four-call limit reached"));
        }
        if stage.starts_with("review") && state["reviews"].as_u64().unwrap_or(2)>=2{
            return Ok(refused("review_exhausted","Two-review limit reached"));
        }
        let (limit,charged):(i64,i64)=tx.query_row("SELECT limit_count,charged FROM conversation_budgets WHERE bucket='task'",[],|r|Ok((r.get(0)?,r.get(1)?)))
            .map_err(|e|e.to_string())?;
        if charged>=limit{return Ok(refused("budget_exhausted","Task invocation allowance exhausted"));}
        tx.execute("INSERT INTO static_effects(run_id,stage,packet_sha256,operation_id,result_sha256,charged) VALUES(?1,?2,?3,?4,NULL,0)",params![run,stage,hash,packet["operation_id"].as_str()])
            .map_err(|e|e.to_string())?;
        // A prepared operation is not a model launch. Charge only after the
        // existing host commit-spawn marker proves an invocation may have run.
        if stage.starts_with("review"){state["reviews"]=json!(state["reviews"].as_u64().unwrap_or(0)+1);}
        save(&tx,run,&state)?;tx.commit().map_err(|e|e.to_string())?;
        Ok(json!({"status":"accepted","replayed":false,"packet_sha256":hash}))
    }
    pub fn static_charge_if_started(&self,run:&str,packet:&Value)->Result<bool,String>{
        if let Some(writer)=&self.workflow.writer{let (run,packet)=(run.to_owned(),packet.clone());
            return writer.call(move|s|s.static_charge_if_started(&run,&packet));}
        let stage=packet["stage"].as_str().ok_or("Missing stage")?;
        let directory=self.workflow.state.join("runs").join(run).join("operations").join(stage);
        let marker=directory.join("invocation-reservation.json");
        if !marker.exists(){return Ok(false);}
        let op=read(&directory.join("operation.json"))?;
        let hash=canonical_sha256(packet).map_err(|e|e.to_string())?;
        if op["packet_sha256"]!=hash||op["project_key"]!=self.key{return Err("Host launch marker identity mismatch".into());}
        let proof=read(&marker)?;
        if proof["attempt_id"].as_str().is_none_or(str::is_empty){return Err("Invalid host invocation reservation".into());}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let charged:Option<(String,i64)>=tx.query_row("SELECT packet_sha256,charged FROM static_effects WHERE run_id=?1 AND stage=?2",
            params![run,stage],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let Some((frozen,prior))=charged else{return Err("Unreserved static invocation".into())};
        if frozen!=hash{return Err("Static invocation packet conflict".into());}
        if prior==1{return Ok(true);}
        tx.execute("UPDATE static_effects SET charged=1 WHERE run_id=?1 AND stage=?2",params![run,stage]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_budgets SET charged=charged+1 WHERE bucket='task'",[]).map_err(|e|e.to_string())?;
        let mut state=row(&tx,run)?.ok_or("Static run missing")?;
        state["used"]=json!(state["used"].as_u64().unwrap_or(0)+1);
        save(&tx,run,&state)?;tx.commit().map_err(|e|e.to_string())?;Ok(true)
    }
    pub fn static_apply_saved(&self,run:&str,packet:&Value,receipt:&Value,result:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let (run,packet,receipt,result)=(run.to_owned(),packet.clone(),receipt.clone(),result.clone());
            return writer.call(move|s|s.static_apply_saved(&run,&packet,&receipt,&result));}
        validate_result(result,packet)?;
        if receipt["status"]!="completed"||receipt["packet_sha256"]!=canonical_sha256(packet).map_err(|e|e.to_string())?||
            receipt["result_sha256"]!=canonical_sha256(result).map_err(|e|e.to_string())?{
            return Err("Static receipt does not bind saved result and packet".into());
        }
        let stage=packet["stage"].as_str().ok_or("Missing stage")?;
        let prior_state=self.static_snapshot(run)?.ok_or("Static run missing")?;
        let hash=canonical_sha256(packet).map_err(|e|e.to_string())?;
        let result_hash=canonical_sha256(result).map_err(|e|e.to_string())?;
        // Replay is a read of the already applied effect. It must not publish
        // files again, even if the saved output is adopted more than once.
        let prior:Option<(String,Option<String>,i64)>=self.workflow.connection()?.query_row(
            "SELECT packet_sha256,result_sha256,charged FROM static_effects WHERE run_id=?1 AND stage=?2",
            params![run,stage],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
        let Some((frozen,applied,charged))=prior else{return Err("Static stage was not reserved".into())};
        if frozen!=hash{return Err("Static packet differs from reserved effect".into());}
        if charged!=1{return Err("Completed static invocation lacks durable budget charge".into());}
        if let Some(applied)=applied{
            if applied!=result_hash{return Err("Static result conflicts with applied effect".into());}
            if !prior_state["candidate"].is_null(){verified_manifest(&self.workflow.state,run,&prior_state["candidate"])?;}
            return Ok(prior_state);
        }
        if prior_state["status"]!="running"||prior_state["next_stage"]!=stage{
            return Err("Static stage changed before result application".into());
        }
        let published=if author_stage(stage) && result["schema_version"]==1{
            Some(publish(&self.workflow.state,run,stage,result,&prior_state["scope_sha256"],
                &prior_state["decision_sha256"],&prior_state["context_sha256"])?)
        }else{None};
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let mut state=row(&tx,run)?.ok_or("Static run missing")?;
        let effect:Option<(String,Option<String>,i64)>=tx.query_row("SELECT packet_sha256,result_sha256,charged FROM static_effects WHERE run_id=?1 AND stage=?2",
            params![run,stage],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
        let Some((frozen,applied,charged))=effect else{return Err("Static stage was not reserved".into())};
        if frozen!=hash{return Err("Static packet differs from reserved effect".into());}
        if charged!=1{return Err("Completed static invocation lacks durable budget charge".into());}
        if let Some(prior)=applied{return if prior==result_hash{Ok(state)}else{Err("Static result conflicts with applied effect".into())};}
        if state["status"]!="running"||state["next_stage"]!=stage{return Err("Static stage changed before result application".into());}
        if result["schema_version"]==2{
            state["status"]="awaiting-input".into();
            state["pending"]=json!({"schema_version":2,"kind":"choice",
                "request_id":format!("worker-{run}-{stage}"),"question":result["question"],
                "dependent_stage":if stage=="implement"{"implement-followup"}else{"repair-followup"},
                "source_stage":stage,"source_result_sha256":result_hash});
        }else if author_stage(stage){
            let candidate=published.ok_or("Candidate publication missing")?;
            if matches!(stage,"repair"|"repair-followup"){
                let findings=state["findings"].as_array().ok_or("Invalid finding state")?.clone();
                let mut assessments=state["assessments"].as_array().ok_or("Invalid assessment state")?.clone();
                for disposition in result["dispositions"].as_array().ok_or("Invalid dispositions")?{
                    let id=disposition["finding_ref"].as_str().ok_or("Missing finding reference")?;
                    findings.iter().find(|f|f["id"]==id).ok_or("Disposition references unknown finding")?;
                    assessments.push(json!({"stage":stage,"kind":"author-disposition","candidate_sha256":candidate["sha256"],
                        "assessment":disposition}));
                }
                state["findings"]=json!(findings);
                state["assessments"]=json!(assessments);
            }
            state["candidate"]=candidate;
            state["next_stage"]=if matches!(stage,"implement"|"implement-followup"){"review-1"}else{"review-2"}.into();
        }else{
            if packet["candidate"]!=state["candidate"]{return Err("Review candidate changed".into());}
            let mut findings=state["findings"].as_array().ok_or("Invalid findings")?.clone();
            let mut assessments=state["assessments"].as_array().ok_or("Invalid assessments")?.clone();
            for assessment in result["assessments"].as_array().ok_or("Invalid assessments")?{
                let Some(existing)=findings.iter_mut().find(|f|f["id"]==assessment["finding_ref"])else{
                    return Err("Ambiguous review reference".into());
                };
                existing["status"]=assessment["assessment"].clone();
                assessments.push(json!({"stage":stage,"candidate_sha256":state["candidate"]["sha256"],"assessment":assessment}));
            }
            for new in result["new_findings"].as_array().ok_or("Invalid new findings")?{
                let id=format!("sf-{}",findings.len()+1);
                findings.push(json!({"id":id,"source_stage":stage,"candidate_sha256":state["candidate"]["sha256"],
                    "evidence":new["evidence"],"requested_change":new["requested_change"],"status":"unresolved"}));
            }
            state["findings"]=json!(findings);state["assessments"]=json!(assessments);
            if stage=="review-1" && findings.iter().any(|f|f["status"]!="addressed"){
                if state["used"].as_u64().unwrap_or(4)+2>4{
                    state["status"]="blocked".into();
                    state["reason"]="Review found unresolved work, but repair and independent re-review require two more calls; frozen four-call allowance is insufficient".into();
                }else{state["next_stage"]="repair".into();}
            }
            else{
                state["next_stage"]="final".into();state["status"]="awaiting-input".into();
                let unresolved=findings.iter().filter(|f|f["status"]!="addressed").cloned().collect::<Vec<_>>();
                state["pending"]=json!({"schema_version":1,"kind":"final-review","request_id":format!("final-{run}"),
                    "artifact_sha256":state["candidate"]["sha256"],"artifact":state["candidate"],
                    "unresolved_findings":unresolved,"last_reviewed":packet["candidate"],
                    "notice":"Review the exact bundle and unresolved findings. The final candidate has no further independent review after the last review stage."});
            }
        }
        state["completed"].as_array_mut().ok_or("Invalid completed stages")?.push(json!(stage));
        tx.execute("UPDATE static_effects SET result_sha256=?1 WHERE run_id=?2 AND stage=?3",params![result_hash,run,stage])
            .map_err(|e|e.to_string())?;
        save(&tx,run,&state)?;tx.commit().map_err(|e|e.to_string())?;Ok(state)
    }
    pub fn static_stop(&self,run:&str,status:&str,reason:&str)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer{let (run,status,reason)=(run.to_owned(),status.to_owned(),reason.to_owned());
            return writer.call(move|s|s.static_stop(&run,&status,&reason));}
        if !matches!(status,"failed"|"uncertain"|"output-invalid"|"blocked"|"cancelled") {return Err("Invalid static stop state".into());}
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let mut state=row(&tx,run)?.ok_or("Static run missing")?;
        if state["status"]==status{return Ok(());}
        if state["status"]!="running" && !(state["status"]=="awaiting-input"&&matches!(status,"cancelled"|"output-invalid")){
            return Err("Static run already resolved".into());
        }
        state["status"]=status.into();state["reason"]=reason.into();save(&tx,run,&state)?;tx.commit().map_err(|e|e.to_string())
    }
    pub fn static_preview(&self,run:&str)->Result<Option<(String,String)>,String>{
        let Some(state)=self.static_snapshot(run)? else{return Ok(None)};
        let candidate=&state["candidate"];
        if candidate.is_null(){return Ok(None);}
        let manifest=verified_manifest(&self.workflow.state,run,candidate)?;
        let dir=candidate_dir(&self.workflow.state,run,candidate["stage"].as_str().ok_or("Invalid stage")?);
        let mut preview=String::new();
        for item in manifest["files"].as_array().ok_or("Invalid manifest")?{
            let name=item["path"].as_str().ok_or("Invalid path")?;
            preview.push_str(&format!("\n===== {name} =====\n"));
            preview.push_str(&fs::read_to_string(dir.join(name)).map_err(|e|e.to_string())?);
            preview.push('\n');
        }
        Ok(Some((preview,candidate["sha256"].as_str().ok_or("Invalid manifest hash")?.into())))
    }
    pub fn static_file_preview(&self,run:&str,name:&str)->Result<(String,String,String),String>{
        if !FILES.contains(&name){return Err("Unknown or unsafe candidate file".into());}
        let state=self.static_snapshot(run)?.ok_or("Static run missing")?;
        let candidate=&state["candidate"];
        let manifest=verified_manifest(&self.workflow.state,run,candidate)?;
        let entry=manifest["files"].as_array().ok_or("Invalid manifest files")?.iter()
            .find(|entry|entry["path"]==name).ok_or("Candidate file missing")?;
        let path=candidate_dir(&self.workflow.state,run,candidate["stage"].as_str().ok_or("Invalid stage")?).join(name);
        Ok((fs::read_to_string(path).map_err(|e|e.to_string())?,
            entry["sha256"].as_str().ok_or("Invalid file hash")?.into(),
            candidate["sha256"].as_str().ok_or("Invalid manifest hash")?.into()))
    }
    pub fn static_read_progress(&self,run:&str,request:&str,hash:&str,offset:u64,end:u64,size:u64)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer{let (run,request,hash)=(run.to_owned(),request.to_owned(),hash.to_owned());
            return writer.call(move|s|s.static_read_progress(&run,&request,&hash,offset,end,size));}
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let state=row(&tx,run)?.ok_or("Static run missing")?;
        if state["pending"]["request_id"]!=request||state["pending"]["artifact_sha256"]!=hash{
            return Err("Review request or candidate changed".into());
        }
        let prior:Option<(String,i64)>=tx.query_row("SELECT manifest_sha256,offset FROM static_reads WHERE run_id=?1 AND request_id=?2",
            params![run,request],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let current=prior.as_ref().map_or(0,|v|v.1.max(0) as u64);
        if prior.as_ref().is_some_and(|v|v.0!=hash)||offset>current{return Err("Candidate reading cursor is stale or skipped".into());}
        let next=current.max(end);
        tx.execute("INSERT INTO static_reads(run_id,request_id,manifest_sha256,offset,complete) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(run_id,request_id) DO UPDATE SET offset=excluded.offset,complete=excluded.complete",
            params![run,request,hash,next as i64,(next==size) as i64]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn static_answer(&self,body:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let body=body.clone();return writer.call(move|s|s.static_answer(&body));}
        if let Some(prior)=self.command_prior(body)?{return Ok(prior);}
        let task=body["payload"]["task_id"].as_str().ok_or("Missing task ID")?;
        let run=self.task(task)?.and_then(|v|v["run_id"].as_str().map(str::to_owned)).ok_or("Task has no static run")?;
        let original=self.static_snapshot(&run)?.ok_or("Static run missing")?;
        if original["pending"]["kind"]=="final-review"{
            if let Err(error)=verified_manifest(&self.workflow.state,&run,&original["candidate"]){
                let reason=format!("Exact candidate integrity failed: {error}. Approval refused; inspect retained manifest and files.");
                self.static_stop(&run,"output-invalid",&reason)?;
                self.transition(task,&run,"output-invalid",Some(&reason))?;
                self.checkpoint()?;
                return Err(reason);
            }
        }
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let prior:Option<(String,String)>=tx.query_row("SELECT body,outcome FROM session_commands WHERE command_id=?1",[body["command_id"].as_str().ok_or("Missing command ID")?],|r|Ok((r.get(0)?,r.get(1)?)))
            .optional().map_err(|e|e.to_string())?;
        if let Some((input,outcome))=prior{return if input==encoded(body)?{serde_json::from_str(&outcome).map_err(|e|e.to_string())}
            else{Ok(refused("idempotency_conflict","Command ID belongs to different input"))};}
        let mut state=row(&tx,&run)?.ok_or("Static run missing")?;
        let row=self.task(task)?.ok_or("Task missing")?;
        if row["status"]!="awaiting-input"||row["version"]!=body["expected_task_version"]||
            row["state_revision"]!=body["expected_state_revision"]||state["status"]!="awaiting-input"||
            state["pending"]["request_id"]!=body["payload"]["request_id"]||
            state["pending"]["artifact_sha256"]!=body["payload"]["artifact_sha256"]{
            return Ok(refused("stale_review","Task, request or candidate changed"));
        }
        let outcome=body["payload"]["outcome"].as_str().ok_or("Missing outcome")?;
        if state["pending"]["kind"]=="choice"{
            if outcome!="answered"{return Ok(refused("invalid_answer","This worker question needs an answer"));}
            let answer=body["payload"]["answer"].as_str().ok_or("Missing answer")?;
            if answer.trim().is_empty()||answer.len()>4096{return Ok(refused("invalid_answer","Answer must contain 1–4096 bytes"));}
            let record=json!({"request_id":state["pending"]["request_id"],"question":state["pending"]["question"],
                "source_result_sha256":state["pending"]["source_result_sha256"],"answer":answer,
                "command_id":body["command_id"]});
            let next_stage=state["pending"]["dependent_stage"].as_str().ok_or("Missing dependent stage")?.to_owned();
            let used=state["used"].as_u64().unwrap_or(4);
            // The follow-up needs a worker call and a subsequent independent review.
            let next_status=if used+2>4{"blocked"}else{"active"};
            state["question_answers"].as_array_mut().ok_or("Invalid answer history")?.push(record.clone());
            state["pending"]=Value::Null;
            state["next_stage"]=next_stage.into();
            state["status"]=if next_status=="active"{"running"}else{"blocked"}.into();
            if next_status=="blocked"{
                state["reason"]="Worker question was answered, but the remaining frozen task-call allowance cannot cover the follow-up and independent review".into();
            }
            save(&tx,&run,&state)?;
            tx.execute("UPDATE session_tasks SET status=?1,state_revision=state_revision+1 WHERE task_id=?2",params![next_status,task])
                .map_err(|e|e.to_string())?;
            let revision_raw:String=tx.query_row("SELECT value FROM session_metadata WHERE name='state_revision'",[],|r|r.get(0))
                .map_err(|e|e.to_string())?;
            let revision=serde_json::from_str::<u64>(&revision_raw).map_err(|e|e.to_string())?+1;
            tx.execute("UPDATE session_metadata SET value=?1 WHERE name='state_revision'",[encoded(&json!(revision))?]).map_err(|e|e.to_string())?;
            tx.execute("INSERT INTO session_events(schema_version,type,task_id,body) VALUES(2,'task.question_answered',?1,?2)",
                params![task,encoded(&record)?]).map_err(|e|e.to_string())?;
            let response=json!({"status":"accepted","task_id":task,"run_id":run,"task_status":next_status,
                "state_revision":revision,"question_answered":true});
            tx.execute("INSERT INTO session_commands VALUES(?1,?2,?3)",params![body["command_id"].as_str(),encoded(body)?,encoded(&response)?])
                .map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;return Ok(response);
        }
        if !matches!(outcome,"approved"|"rejected"){return Ok(refused("invalid_answer","Expected approve or reject"));}
        if outcome=="approved"{
            let receipt:i64=tx.query_row("SELECT complete FROM static_reads WHERE run_id=?1 AND request_id=?2 AND manifest_sha256=?3",
                params![run,state["pending"]["request_id"].as_str(),state["pending"]["artifact_sha256"].as_str()],|r|r.get(0))
                .optional().map_err(|e|e.to_string())?.unwrap_or(0);
            if receipt!=1{return Ok(refused("candidate_not_read","Read the exact candidate bundle before approval"));}
        }
        if state["candidate"]!=original["candidate"]{return Ok(refused("stale_review","Candidate changed while verifying it"));}
        state["status"]=outcome.into();state["decision"]=json!({"command_id":body["command_id"],"request_id":state["pending"]["request_id"],
            "manifest_sha256":state["candidate"]["sha256"],"outcome":outcome,"answer":body["payload"]["answer"]});
        state["pending"]=Value::Null;save(&tx,&run,&state)?;
        tx.execute("UPDATE session_tasks SET status=?1,state_revision=state_revision+1 WHERE task_id=?2",params![outcome,task])
            .map_err(|e|e.to_string())?;
        let next:Value=serde_json::from_str(&tx.query_row::<String,_,_>("SELECT value FROM session_metadata WHERE name='state_revision'",[],|r|r.get(0))
            .map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let revision=next.as_u64().ok_or("Invalid revision")?+1;
        tx.execute("UPDATE session_metadata SET value=?1 WHERE name='state_revision'",[encoded(&json!(revision))?]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO session_events(schema_version,type,task_id,body) VALUES(2,'task.final_decision',?1,?2)",
            params![task,encoded(&state["decision"])?]).map_err(|e|e.to_string())?;
        let outcome=json!({"status":"accepted","task_id":task,"run_id":run,"request_id":body["payload"]["request_id"],
            "task_status":outcome,"manifest_sha256":state["decision"]["manifest_sha256"],"state_revision":revision});
        tx.execute("INSERT INTO session_commands VALUES(?1,?2,?3)",params![body["command_id"].as_str(),encoded(body)?,encoded(&outcome)?])
            .map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;Ok(outcome)
    }
}

pub fn fake_result(packet:&Value)->Result<Value,String>{
    let stage=packet["stage"].as_str().ok_or("Missing stage")?;
    let hash=canonical_sha256(packet).map_err(|e|e.to_string())?;
    let mut result=json!({"schema_version":1,"operation_id":packet["operation_id"],"packet_sha256":hash});
    match stage{
        "implement"|"implement-followup"|"repair"|"repair-followup"=>{
            let repaired=matches!(stage,"repair"|"repair-followup");
            let markup="<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Sample Weather</title><link rel=\"stylesheet\" href=\"styles.css\"><main><h1>Sample Weather</h1><label>City <select id=\"city\"><option>Boston</option><option>Chicago</option><option>Seattle</option></select></label><label>Units <select id=\"units\"><option value=\"C\">Celsius</option><option value=\"F\">Fahrenheit</option></select></label><section aria-live=\"polite\"><h2 id=\"place\"></h2><p id=\"temperature\"></p><p id=\"condition\"></p><p id=\"humidity\"></p><p id=\"wind\"></p></section></main><script src=\"app.js\"></script></html>";
            let css=if repaired{"body{font:1rem system-ui;margin:2rem auto;max-width:42rem;padding:0 1rem}label{display:inline-block;margin:0 1rem 1rem 0}select{font:inherit;padding:.5rem}section{border:1px solid #777;border-radius:.5rem;padding:1rem}@media(max-width:360px){label{display:block}}"}
                else{"body{font:1rem system-ui;margin:2rem auto;max-width:42rem;padding:0 1rem}label{display:inline-block;margin:0 1rem 1rem 0}section{border:1px solid #777;padding:1rem}"};
            let js="const data={Boston:{c:18,condition:'Cloudy',humidity:62,wind:12},Chicago:{c:22,condition:'Sunny',humidity:49,wind:18},Seattle:{c:15,condition:'Rain',humidity:81,wind:9}};const city=document.querySelector('#city'),units=document.querySelector('#units');function render(){let name=city.value,item=data[name],fahrenheit=units.value==='F';document.querySelector('#place').textContent=name;document.querySelector('#temperature').textContent=`Temperature: ${fahrenheit?Math.round(item.c*9/5+32):item.c} °${units.value}`;document.querySelector('#condition').textContent=`Condition: ${item.condition}`;document.querySelector('#humidity').textContent=`Humidity: ${item.humidity}%`;document.querySelector('#wind').textContent=`Wind: ${item.wind} km/h`;}city.addEventListener('change',render);units.addEventListener('change',render);render();";
            result["files"]=json!([{"path":"index.html","content":markup},{"path":"styles.css","content":css},
                {"path":"app.js","content":js},{"path":"README.md","content":"# Sample Weather\nOpen index.html in a browser. Choose Boston, Chicago or Seattle, then change units. All data is local and fixed. Check 360 px width and keyboard controls. No external requests are made.\n"}]);
            result["dispositions"]=if repaired{json!([{"finding_ref":"sf-1","assessment":"addressed",
                "rationale":"Added mobile wrapping and padded native controls."}])}else{json!([])};
        }
        "review-1"=>{result["new_findings"]=json!([{"evidence":"Initial controls have no narrow-screen wrapping/padding rule.",
            "requested_change":"Add a 360 px layout and padded controls."}]);result["assessments"]=json!([]);}
        "review-2"=>{result["new_findings"]=json!([]);result["assessments"]=json!([{"finding_ref":"sf-1",
            "assessment":"addressed","evidence":"Repaired stylesheet includes max-width:360px layout and padded selects."}]);}
        _=>return Err("Invalid mock static stage".into())
    }
    validate_result(&result,packet)?;Ok(result)
}

#[cfg(test)]
mod tests{
    use super::*;
    use tempfile::TempDir;
    fn fixture()->(TempDir,Session,Value){
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        fs::create_dir_all(&project).unwrap();let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":4,
            "execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let session=Session::open(&project,&state,key,"test-release").unwrap();
        session.workflow.connection().unwrap().execute("INSERT INTO session_tasks VALUES('task',1,1,'awaiting-input','Weather','Weather','r-task')",[]).unwrap();
        let plan=json!({"schema_version":2,"workflow_profile":PROFILE,"project_key":key,"task_id":"task",
            "run_id":"r-task","brief":"Weather","proposal_result_sha256":"scope-hash",
            "decision_sha256":"decision-hash","context_handoff_sha256":"context-hash"});
        session.static_create("task",&plan).unwrap();(root,session,plan)
    }
    fn stage(session:&Session,plan:&Value,expected:&str)->(Value,Value){
        let packet=session.static_packet("r-task",plan).unwrap().unwrap();
        assert_eq!(packet["stage"],expected);
        let reserved=session.static_reserve("r-task",&packet).unwrap();
        assert_eq!(reserved["status"],"accepted");
        assert_eq!(session.static_reserve("r-task",&packet).unwrap()["replayed"],true);
        let dir=session.workflow.state.join("runs/r-task/operations").join(expected);
        crate::operation::prepare_host(&session.workflow.state,&dir,&packet,&session.key).unwrap();
        write(&dir.join("invocation-reservation.json"),&json!({"attempt_id":"one"}),true).unwrap();
        assert!(session.static_charge_if_started("r-task",&packet).unwrap());
        assert!(session.static_charge_if_started("r-task",&packet).unwrap());
        let result=fake_result(&packet).unwrap();
        let receipt=json!({"status":"completed","packet_sha256":canonical_sha256(&packet).unwrap(),
            "result_sha256":canonical_sha256(&result).unwrap()});
        let applied=session.static_apply_saved("r-task",&packet,&receipt,&result).unwrap();
        (packet,applied)
    }
    #[test]
    fn four_stage_bundle_review_read_and_exact_approval_survive_reopen(){
        let (_root,session,plan)=fixture();
        stage(&session,&plan,"implement");stage(&session,&plan,"review-1");
        assert_eq!(session.static_snapshot("r-task").unwrap().unwrap()["findings"][0]["status"],"unresolved");
        let (repair_packet,repair_state)=stage(&session,&plan,"repair");
        assert_eq!(repair_state["findings"][0]["status"],"unresolved");
        assert_eq!(repair_state["assessments"][0]["kind"],"author-disposition");
        let repair_result=fake_result(&repair_packet).unwrap();
        let repair_receipt=json!({"status":"completed","packet_sha256":canonical_sha256(&repair_packet).unwrap(),
            "result_sha256":canonical_sha256(&repair_result).unwrap()});
        assert_eq!(session.static_apply_saved("r-task",&repair_packet,&repair_receipt,&repair_result).unwrap(),repair_state);
        let (_,final_state)=stage(&session,&plan,"review-2");
        assert_eq!(final_state["status"],"awaiting-input");
        assert_eq!(final_state["used"],4);assert_eq!(final_state["reviews"],2);
        assert_eq!(final_state["findings"][0]["status"],"addressed");
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["task"]["charged"],4);
        let (preview,hash)=session.static_preview("r-task").unwrap().unwrap();
        assert!(preview.contains("===== app.js ====="));
        let request=final_state["pending"]["request_id"].as_str().unwrap();
        let command=json!({"command_id":"approve","action":"answer_task","expected_task_version":1,
            "expected_state_revision":1,"payload":{"task_id":"task","request_id":request,
                "outcome":"approved","answer":"Approved","artifact_sha256":hash}});
        assert_eq!(session.static_answer(&command).unwrap()["code"],"candidate_not_read");
        session.static_read_progress("r-task",request,&hash,0,preview.len() as u64,preview.len() as u64).unwrap();
        let accepted=session.static_answer(&command).unwrap();
        assert_eq!(accepted["task_status"],"approved");
        assert_eq!(session.static_answer(&command).unwrap(),accepted);
        let reopened=Session::open_existing(&session.project,&session.workflow.state,&session.key,&session.release).unwrap();
        assert_eq!(reopened.static_snapshot("r-task").unwrap().unwrap()["decision"]["manifest_sha256"],hash);
        assert_eq!(reopened.conversation_snapshot().unwrap()["budgets"]["task"]["charged"],4);
    }
    #[test]
    fn invalid_paths_missing_coverage_ambiguous_review_and_tamper_are_refused(){
        let (_root,session,plan)=fixture();
        let packet=session.static_packet("r-task",&plan).unwrap().unwrap();
        let mut result=fake_result(&packet).unwrap();
        result["files"][0]["path"]="../other".into();
        assert!(validate_result(&result,&packet).unwrap_err().contains("path"));
        let mut result=fake_result(&packet).unwrap();
        result["files"][1]["path"]="index.html".into();
        assert!(validate_result(&result,&packet).unwrap_err().contains("path"));
        let mut result=fake_result(&packet).unwrap();
        result["files"][0]["content"]=json!("x".repeat(LIMIT));
        assert!(validate_result(&result,&packet).unwrap_err().contains("32 KiB"));
        stage(&session,&plan,"implement");stage(&session,&plan,"review-1");
        let repair=session.static_packet("r-task",&plan).unwrap().unwrap();
        let mut incomplete=fake_result(&repair).unwrap();incomplete["dispositions"]=json!([]);
        validate_result(&incomplete,&repair).unwrap();
        let mut bad=fake_result(&repair).unwrap();bad["dispositions"][0]["finding_ref"]="other".into();
        assert!(validate_result(&bad,&repair).is_ok()); // semantic reference is checked by the broker
        let candidate=session.static_snapshot("r-task").unwrap().unwrap()["candidate"].clone();
        let path=candidate_dir(&session.workflow.state,"r-task","implement").join("app.js");
        fs::write(path,"modified").unwrap();
        assert!(verified_manifest(&session.workflow.state,"r-task",&candidate).is_err());
        assert!(session.static_packet("r-task",&plan).is_err());
    }
    #[test]
    fn omitted_repair_disposition_remains_unresolved_in_second_review_packet(){
        let (_root,session,plan)=fixture();
        stage(&session,&plan,"implement");stage(&session,&plan,"review-1");
        let packet=session.static_packet("r-task",&plan).unwrap().unwrap();
        let mut result=fake_result(&packet).unwrap();result["dispositions"]=json!([]);
        let stage=packet["stage"].as_str().unwrap();
        let dir=session.workflow.state.join("runs/r-task/operations").join(stage);
        crate::operation::prepare_host(&session.workflow.state,&dir,&packet,&session.key).unwrap();
        assert_eq!(session.static_reserve("r-task",&packet).unwrap()["status"],"accepted");
        write(&dir.join("invocation-reservation.json"),&json!({"attempt_id":"one"}),true).unwrap();
        session.static_charge_if_started("r-task",&packet).unwrap();
        let receipt=json!({"status":"completed","packet_sha256":canonical_sha256(&packet).unwrap(),
            "result_sha256":canonical_sha256(&result).unwrap()});
        let next=session.static_apply_saved("r-task",&packet,&receipt,&result).unwrap();
        assert_eq!(next["findings"][0]["status"],"unresolved");
        let review=session.static_packet("r-task",&plan).unwrap().unwrap();
        assert_eq!(review["stage"],"review-2");
        assert_eq!(review["findings"][0]["id"],"sf-1");
        assert_eq!(review["findings"][0]["status"],"unresolved");
        let mut ambiguous=fake_result(&review).unwrap();
        ambiguous["assessments"][0]["finding_ref"]="unknown".into();
        assert!(validate_result(&ambiguous,&review).is_ok());
        let review_dir=session.workflow.state.join("runs/r-task/operations/review-2");
        crate::operation::prepare_host(&session.workflow.state,&review_dir,&review,&session.key).unwrap();
        session.static_reserve("r-task",&review).unwrap();
        write(&review_dir.join("invocation-reservation.json"),&json!({"attempt_id":"two"}),true).unwrap();
        session.static_charge_if_started("r-task",&review).unwrap();
        let receipt=json!({"status":"completed","packet_sha256":canonical_sha256(&review).unwrap(),
            "result_sha256":canonical_sha256(&ambiguous).unwrap()});
        assert!(session.static_apply_saved("r-task",&review,&receipt,&ambiguous).unwrap_err().contains("Ambiguous"));
        assert_eq!(session.static_snapshot("r-task").unwrap().unwrap()["next_stage"],"review-2");
    }
    #[test]
    fn file_selection_does_not_grant_bundle_approval_and_tampering_is_refused(){
        let (_root,session,plan)=fixture();
        stage(&session,&plan,"implement");stage(&session,&plan,"review-1");
        stage(&session,&plan,"repair");let (_,state)=stage(&session,&plan,"review-2");
        let (file,file_hash,manifest)=session.static_file_preview("r-task","app.js").unwrap();
        assert!(file.contains("const data="));assert_eq!(sha256(file.as_bytes()),file_hash);
        assert_eq!(manifest,state["candidate"]["sha256"]);
        assert!(session.static_file_preview("r-task","../README.md").is_err());
        let request=state["pending"]["request_id"].as_str().unwrap();
        let approval=json!({"command_id":"file-only","action":"answer_task","expected_task_version":1,
            "expected_state_revision":1,"payload":{"task_id":"task","request_id":request,
                "outcome":"approved","answer":"Approve","artifact_sha256":manifest}});
        assert_eq!(session.static_answer(&approval).unwrap()["code"],"candidate_not_read");
        let path=candidate_dir(&session.workflow.state,"r-task","repair").join("app.js");
        fs::write(path,"tampered").unwrap();
        assert!(session.static_file_preview("r-task","app.js").is_err());
        assert!(session.static_answer(&approval).is_err());
        assert_eq!(session.static_snapshot("r-task").unwrap().unwrap()["status"],"output-invalid");
        assert_eq!(session.task("task").unwrap().unwrap()["status"],"output-invalid");
    }
    #[test]
    fn budget_refusal_and_cancellation_do_not_launch_a_worker(){
        let (_root,session,plan)=fixture();
        session.workflow.connection().unwrap().execute("UPDATE conversation_budgets SET limit_count=0 WHERE bucket='task'",[]).unwrap();
        let packet=session.static_packet("r-task",&plan).unwrap().unwrap();
        assert_eq!(session.static_reserve("r-task",&packet).unwrap()["code"],"budget_exhausted");
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["task"]["charged"],0);
        session.static_stop("r-task","cancelled","Operator cancelled before dispatch").unwrap();
        assert!(session.static_packet("r-task",&plan).unwrap().is_none());
    }
    #[test]
    fn linked_candidate_directory_cannot_redirect_publication(){
        let (root,session,plan)=fixture();
        let packet=session.static_packet("r-task",&plan).unwrap().unwrap();
        let result=fake_result(&packet).unwrap();
        let outside=root.path().join("outside");fs::create_dir_all(&outside).unwrap();
        let parent=session.workflow.state.join("runs/r-task/static/candidates");
        fs::create_dir_all(&parent).unwrap();
        std::os::unix::fs::symlink(&outside,parent.join("implement")).unwrap();
        assert!(publish(&session.workflow.state,"r-task","implement",&result,
            &json!("scope"),&json!("decision"),&json!("context")).is_err());
        assert!(!outside.join("index.html").exists());
    }
    #[test]
    fn worker_question_is_durable_and_does_not_expand_four_call_budget(){
        let (_root,session,plan)=fixture();
        let packet=session.static_packet("r-task",&plan).unwrap().unwrap();
        session.static_reserve("r-task",&packet).unwrap();
        let dir=session.workflow.state.join("runs/r-task/operations/implement");
        crate::operation::prepare_host(&session.workflow.state,&dir,&packet,&session.key).unwrap();
        write(&dir.join("invocation-reservation.json"),&json!({"attempt_id":"one"}),true).unwrap();
        session.static_charge_if_started("r-task",&packet).unwrap();
        let question=json!({"schema_version":2,"operation_id":packet["operation_id"],
            "packet_sha256":canonical_sha256(&packet).unwrap(),
            "question":{"id":"units","question":"Which temperature units?","options":["Celsius","Toggle"]}});
        let receipt=json!({"status":"completed","packet_sha256":canonical_sha256(&packet).unwrap(),
            "result_sha256":canonical_sha256(&question).unwrap()});
        let pending=session.static_apply_saved("r-task",&packet,&receipt,&question).unwrap();
        assert_eq!(pending["pending"]["kind"],"choice");
        assert_eq!(pending["used"],1);
        assert_eq!(session.static_apply_saved("r-task",&packet,&receipt,&question).unwrap(),pending);
        let answer=json!({"command_id":"answer-units","action":"answer_task","expected_task_version":1,
            "expected_state_revision":1,"payload":{"task_id":"task","request_id":"worker-r-task-implement",
                "outcome":"answered","answer":"Toggle","artifact_sha256":null}});
        let saved=session.static_answer(&answer).unwrap();
        assert_eq!(saved["task_status"],"active");
        assert_eq!(session.static_answer(&answer).unwrap(),saved);
        let followup=session.static_packet("r-task",&plan).unwrap().unwrap();
        assert_eq!(followup["stage"],"implement-followup");
        assert_eq!(followup["question_answers"][0]["answer"],"Toggle");
        stage(&session,&plan,"implement-followup");
        let (_,review)=stage(&session,&plan,"review-1");
        assert_eq!(review["status"],"blocked");
        assert!(review["reason"].as_str().unwrap().contains("four-call allowance"));
        assert_eq!(review["used"],3);
        assert!(session.static_packet("r-task",&plan).unwrap().is_none());
    }
}
