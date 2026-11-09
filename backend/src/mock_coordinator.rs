//! Deterministic, explicitly simulated coordinator for installed schema-4 checks.
//! It does not execute a provider CLI or interpret arbitrary natural language.
use serde_json::{json,Value};

pub fn response(role:&str,intent:&str,text:&str,has_answer:bool,task_id:Option<&str>)->Value{
    let lowered=text.to_ascii_lowercase();
    if role=="consultation_request"{
        return json!({"schema_version":1,"assistant_text":"Mock second-agent opinion: keep the sample data local, label the selected city and unit clearly, and check keyboard access and narrow layouts. This is scripted fixture advice, not an independent model review.","evidence_refs":[],"action":null});
    }
    if role=="internal"{
        return json!({"schema_version":1,"assistant_text":"Mock coordinator synthesis: the bounded second-agent opinion is recorded. Here is a scope for your review; no worker is authorized yet.","evidence_refs":[],
            "action":{"kind":"propose_task","payload":{"brief":"Build a static local weather page with fixed sample cities, accessible city and unit controls, and no external requests. Publish only the approved file bundle for review."}}});
    }
    if intent=="steer"{
        return json!({"schema_version":1,"assistant_text":"Mock coordinator prepared an in-scope steering suggestion. It remains pending until you accept it for an undispatched boundary.","evidence_refs":[],
            "action":{"kind":"propose_steering","payload":{"task_id":task_id.unwrap_or(""),"text":text}}});
    }
    if lowered.starts_with("answer to "){
        return json!({"schema_version":1,"assistant_text":format!("Mock coordinator recorded your answer: {}. You can request a second-agent opinion or ask for a task proposal.",text.split_once(':').map_or(text,|(_,value)|value.trim())),"evidence_refs":[],"action":null});
    }
    if lowered.contains("consult") || lowered.contains("second opinion"){
        return json!({"schema_version":1,"assistant_text":"Mock coordinator prepared one bounded consultation. Review and authorize that separate call; it does not authorize task execution.","evidence_refs":[],
            "action":{"kind":"request_consultation","payload":{"question":"What should the static sample weather page emphasize for clarity and accessibility?","evidence_refs":[]}}});
    }
    if lowered.contains("weather") && !has_answer{
        return json!({"schema_version":1,"assistant_text":"Mock coordinator can help scope a fixed-data weather page. One product choice affects its behavior.","evidence_refs":[],
            "action":{"kind":"ask_question","payload":{"question":"Should the sample page show Celsius only or include a Celsius/Fahrenheit toggle?","rationale":"The selected unit changes the display and acceptance checks.","options":["Celsius only","Celsius/Fahrenheit toggle"]}}});
    }
    if intent=="task_request" || lowered.contains("propose") || lowered.contains("build") || lowered.contains("create"){
        return json!({"schema_version":1,"assistant_text":"Mock coordinator prepared a bounded task scope. Open Review scope before authorizing any worker call.","evidence_refs":[],
            "action":{"kind":"propose_task","payload":{"brief":"Build a static local weather page with fixed sample cities and no external requests, using the saved unit decision."}}});
    }
    json!({"schema_version":1,"assistant_text":"Mock coordinator response: this is scripted discussion. Ask about the fixed-data weather page, request a consultation, or ask for a proposal to exercise the workflow.","evidence_refs":[],"action":null})
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::native_contract::CoordinatorTurn;
    #[test]
    fn ordinary_discussion_has_no_task_action(){
        let result=response("user","discussion","What do you think?",false,None);
        assert!(CoordinatorTurn::validate(&result).unwrap().action.is_none());
    }
    #[test]
    fn weather_question_consultation_and_synthesis_are_distinct(){
        assert_eq!(response("user","task_request","Build a weather page",false,None)["action"]["kind"],"ask_question");
        assert_eq!(response("user","discussion","Consult a second agent",true,None)["action"]["kind"],"request_consultation");
        assert!(response("consultation_request","discussion","What matters?",true,None)["action"].is_null());
        assert_eq!(response("internal","discussion","Synthesize",true,None)["action"]["kind"],"propose_task");
    }
}
