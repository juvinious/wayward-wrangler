impl App {
    fn candidate_text(c:&Value)->String{
        let mut text=format!("Candidate ready · {}\nBranch: {} → target {} · {}\n{}\n",c["commit"].as_str().unwrap_or("?"),c["governing_branch"].as_str().unwrap_or("?"),c["target"].as_str().unwrap_or("?"),c["completion"].as_str().unwrap_or("?"),c["delivery"]["environment"].as_str().unwrap_or("See project documentation"));
        for k in ["prerequisites","build","run","inspect","expected","shutdown","notes"]{for v in c["delivery"][k].as_array().into_iter().flatten().take(3){text.push_str(&format!("• {k}: {}\n",v.as_str().unwrap_or("")));}}
        text.push_str("Instructions are coordinator suggestions; exact checks/reviews are in Inspect. Candidate readiness is not delivery.\nSay “check out candidate” for ordinary checkout (clean registered checkout required). Say “accept candidate” after validation, or “candidate feedback: …”. Committed edits: “include committed changes”. Ending validation leaves your checkout detached at its exact commit, without restoring another branch.");text
    }
    fn restore_candidate_cards(&mut self){let cards=self.development_view()["candidates"].as_array().cloned().unwrap_or_default();for c in cards{let Some(seq)=c["event_seq"].as_u64()else{continue};if !self.journal.messages.iter().any(|m|m.seq==seq){self.journal.messages.push(super::scenario::Message{at:c["at"].as_f64(),seq,speaker:"Waywr".into(),text:Self::candidate_text(&c),task_id:c["work_id"].as_str().unwrap_or("").into(),task_version:0,simulated:false,source_sha256:String::new(),activity:None});}}self.journal.messages.sort_by_key(|m|m.seq);}
    fn present_visible_candidate(&mut self,top:usize,height:usize,width:u16){
        if height==0||self.modal!=Modal::None||self.journal.connection!="connected"{return}
        let Some(w)=self.development_workflow().filter(|w|w["candidate"].is_object()&&w["candidate"]["presented"]!=true)else{return};
        let cards=self.development_view()["candidates"].as_array().cloned().unwrap_or_default();let Some(c)=cards.iter().find(|c|c["candidate_id"]==w["candidate"]["candidate_id"])else{return};let Some(id)=c["candidate_id"].as_str()else{return};if self.card_presenting.as_deref()==Some(id){return}
        let mut line=0;for m in self.journal.messages.iter().filter(|m|Self::visible_message(m)){let count=Paragraph::new(self.message_text(m,width)).wrap(Wrap{trim:false}).line_count(width.max(1));if m.seq==c["event_seq"].as_u64().unwrap_or(0)&&line<top+height&&line+count>top{self.card_presenting=Some(id.into());self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"candidate.present","payload":{"work_id":w["work_id"],"candidate_id":id,"commit":c["commit"]}})));break}line+=count;}
    }
}

#[cfg(test)]mod ordinary_candidate_ui_tests{
 use super::*;
 #[test]fn ordinary_candidate_context_is_frozen_in_drafts_and_rendered_without_execution(){
 let mut a=coordination_ui_tests::app();a.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["workflows"]=serde_json::json!([{"work_id":"t-scope","candidate":{"candidate_id":"candidate-one","commit":"abc","presented":true}}]);
 let captured=a.coordination_context("typed");assert_eq!(captured["candidate"]["commit"],"abc");a.draft_reply=Some(captured);a.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["workflows"][0]["candidate"]["commit"]=serde_json::json!("new");assert_eq!(a.message_payload("accept candidate")["interaction"]["candidate"]["commit"],"abc");
 let rendered=App::candidate_text(&serde_json::json!({"commit":"abc","governing_branch":"waywr/example/integration","target":"trunk","completion":"checked-local","delivery":{"environment":"Node project","run":["npm run dev"]}}));assert!(rendered.contains("npm run dev"));assert!(rendered.contains("Suggestions")||rendered.contains("suggestions"));assert!(a.take_intent().is_none());
 }
}
