//! Inert coordinator suggestions. No provider output grants execution authority.
use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use std::fmt;
pub const RESULT_LIMIT: usize = 64 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub title: String,
    pub objective: String,
    pub deliverables: Vec<String>,
    pub in_scope: Vec<String>,
    pub out_of_scope: Vec<String>,
    pub constraints: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub unresolved_decisions: Vec<String>,
}
impl Proposal {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 120
            || self.objective.len() > 4096
        {
            return Err("Invalid proposal title/objective".into());
        }
        for list in [
            &self.deliverables,
            &self.in_scope,
            &self.out_of_scope,
            &self.constraints,
            &self.acceptance_criteria,
            &self.unresolved_decisions,
        ] {
            if list.len() > 16 || list.iter().any(|v| v.trim().is_empty() || v.len() > 1024) {
                return Err("Invalid proposal list".into());
            }
        }
        if self.unresolved_decisions.len() > 8
            || crate::codec::canonical(&serde_json::to_value(self).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
                .len()
                > 16384
        {
            return Err("Proposal exceeds bounds".into());
        }
        Ok(())
    }
    pub fn eligible(&self) -> bool {
        !self.objective.trim().is_empty()
            && !self.deliverables.is_empty()
            && !self.in_scope.is_empty()
            && !self.acceptance_criteria.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Base {
    pub task_id: String,
    pub revision: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Action {
    RetrieveEvidence { requests: Vec<crate::development_context::Request> },
    AskQuestion {
        prompt: String,
        rationale: String,
        options: Vec<String>,
    },
    UpsertProposal {
        base: Option<Base>,
        proposal: Proposal,
    },
    SuggestQuestionDisposition {
        question_id: String,
        question_revision: u64,
        disposition: String,
        answer: String,
        source_message_id: String,
        source_sha256: String,
        quote_start: usize,
        quote_end: usize,
    },
    ProposeSteering {
        source_message_id:String, source_sha256:String, work_id:String,
        requirement_generation:u64, scope_sha256:String, policy_sha256:String,
        text:String, in_scope:bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema_version: u64,
    pub assistant_text: String,
    pub evidence_refs: Vec<String>,
    pub action: Option<Action>,
}
// Reject duplicate keys at every level before converting into typed records.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("unambiguous JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = vec![];
                while let Some(Strict(item)) = a.next_element()? {
                    v.push(item)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(de::Error::custom("Duplicate JSON field"));
                    }
                    let Strict(item) = a.next_value()?;
                    v.insert(k, item);
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
/// Reject duplicate raw keys before serde_json::Value would discard them.
pub fn strict_json(text: &str) -> Result<Value,String> {
    if text.len()>1024*1024 {return Err("JSON frame exceeds 1 MiB".into())}
    let Strict(v)=serde_json::from_str(text).map_err(|e|e.to_string())?;
    Ok(v)
}
pub fn parse(text: &str) -> Result<Value, String> {
    if text.len() > RESULT_LIMIT {
        return Err("Coordinator envelope exceeds 64 KiB".into());
    }
    let v = strict_json(text)?;
    fn depth(v: &Value, n: usize) -> bool {
        n <= 16
            && match v {
                Value::Array(a) => a.iter().all(|i| depth(i, n + 1)),
                Value::Object(a) => a.values().all(|i| depth(i, n + 1)),
                _ => true,
            }
    }
    if !depth(&v, 0) {
        return Err("Coordinator envelope nesting exceeds limit".into());
    }
    validate(&v)?;
    Ok(v)
}
pub fn validate(v: &Value) -> Result<Response, String> {
    if crate::codec::canonical(v).map_err(|e| e.to_string())?.len() > RESULT_LIMIT {
        return Err("Coordinator envelope exceeds 64 KiB".into());
    }
    let r: Response = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    if !matches!(r.schema_version,2|3)
        || r.assistant_text.len() > 32768
        || r.evidence_refs.len() > 32
        || r.evidence_refs.iter().any(|v| v.len() > 256)
    {
        return Err("Invalid coordinator envelope version/text/references".into());
    }
    match &r.action {
        Some(Action::RetrieveEvidence{requests})=>{if requests.is_empty(){return Err("Empty evidence retrieval".into())}crate::development_context::validate_requests(&serde_json::to_value(requests).map_err(|e|e.to_string())?)?;},
        Some(Action::AskQuestion {
            prompt,
            rationale,
            options,
        }) => {
            if prompt.trim().is_empty()
                || prompt.len() > 4096
                || rationale.trim().is_empty()
                || rationale.len() > 4096
                || options.len() > 8
                || options
                    .iter()
                    .any(|v| v.trim().is_empty() || v.len() > 1024)
            {
                return Err("Invalid clarification".into());
            }
        }
        Some(Action::UpsertProposal { proposal, .. }) => proposal.validate()?,
        Some(Action::ProposeSteering{source_message_id,source_sha256,work_id,scope_sha256,policy_sha256,text,..})=>{
            if r.schema_version!=3||source_message_id.is_empty()||source_message_id.len()>128||work_id.is_empty()||work_id.len()>128||text.trim().is_empty()||text.len()>16000||[source_sha256,scope_sha256,policy_sha256].iter().any(|s|s.len()!=64||!s.bytes().all(|b|b.is_ascii_hexdigit())){return Err("Invalid source-bound steering suggestion".into())}
        },
        Some(Action::SuggestQuestionDisposition {question_id,question_revision,disposition,answer,source_message_id,source_sha256,quote_start,quote_end}) => {
            if r.schema_version!=3 || question_id.is_empty() || question_id.len()>128 || *question_revision==0
              || !matches!(disposition.as_str(),"answer"|"defer") || (disposition=="answer"&&answer.trim().is_empty())
              || answer.len()>16384 || source_message_id.is_empty() || source_message_id.len()>128
              || source_sha256.len()!=64 || !source_sha256.bytes().all(|b|b.is_ascii_hexdigit())
              || quote_start>=quote_end || *quote_end>32768 {return Err("Invalid formal question suggestion".into())}
        },
        None => {}
    }
    Ok(r)
}
/// Display only a top-level string; incomplete/malformed output is provisional.
/// Structural scanning skips other strings, including nested actions and escaped keys.
pub fn provisional(text: &str) -> String {
    if text.len() > RESULT_LIMIT {
        return String::new();
    }
    let b = text.as_bytes();
    let mut i = 0;
    let mut depth = 0;
    while i < b.len() {
        match b[i] {
            b'{' | b'[' => {
                depth += 1;
                i += 1
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1
            }
            b'"' => {
                let start = i;
                i += 1;
                while i < b.len() {
                    if b[i] == b'\\' {
                        i += 2;
                    } else if b[i] == b'"' {
                        break;
                    } else {
                        i += 1
                    }
                }
                if i >= b.len() {
                    break;
                }
                let key = serde_json::from_str::<String>(&text[start..=i]).ok();
                i += 1;
                if depth == 1 && key.as_deref() == Some("assistant_text") {
                    while i < b.len() && b[i].is_ascii_whitespace() {
                        i += 1
                    }
                    if b.get(i) != Some(&b':') {
                        continue;
                    }
                    i += 1;
                    while i < b.len() && b[i].is_ascii_whitespace() {
                        i += 1
                    }
                    if b.get(i) != Some(&b'"') {
                        return String::new();
                    }
                    let start = i;
                    i += 1;
                    while i < b.len() {
                        if b[i] == b'\\' {
                            i += 2
                        } else if b[i] == b'"' {
                            return serde_json::from_str::<String>(&text[start..=i])
                                .ok()
                                .filter(|v| v.len() <= 32768)
                                .unwrap_or_default();
                        } else {
                            i += 1
                        }
                    }
                    let mut end = text.len();
                    for _ in 0..13 {
                        if let Ok(v) =
                            serde_json::from_str::<String>(&format!("{}\"", &text[start..end]))
                        {
                            return if v.len() <= 32768 { v } else { String::new() };
                        }
                        if end <= start + 1 {
                            break;
                        }
                        end -= 1;
                        while !text.is_char_boundary(end) {
                            end -= 1
                        }
                    }
                    return String::new();
                }
            }
            _ => i += 1,
        }
    }
    String::new()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_inert_bounds() {
        let r =
            r#"{"schema_version":2,"assistant_text":"Discuss","evidence_refs":[],"action":null}"#;
        assert!(parse(r).is_ok());
        for bad in [
            r.replace("null", "{\"kind\":\"authorize\",\"payload\":{}}"),
            r.replace(
                "\"schema_version\":2",
                "\"schema_version\":2,\"schema_version\":2",
            ),
            format!("{r}{r}"),
            format!("```json\n{r}\n```"),
            r.replace("\"action\":null", "\"action\":null,\"permission\":true"),
        ] {
            assert!(parse(&bad).is_err(), "{bad}")
        }
        assert!(parse(&r.replace("Discuss", &"x".repeat(32769))).is_err());
    }
    #[test]
    fn streaming_only_text_and_split_escapes() {
        let text = r#"{"schema_version":2,"assistant_text":"Hi\n\u263a \ud83d\ude00","action":{"assistant_text":"secret"}}"#;
        for (i, _) in text.char_indices() {
            let out = provisional(&text[..i]);
            assert!(!out.contains("secret"));
        }
        assert_eq!(provisional(text), "Hi\n☺ 😀");
        assert_eq!(provisional(r#"{"action":{"assistant_text":"secret"}}"#), "");
    }
}

#[cfg(test)] mod continuous_tests {
 use super::*;
 #[test] fn version_three_is_inert_strict_and_does_not_expand_old_actions(){
 let r=json!({"schema_version":3,"assistant_text":"Discuss freely","evidence_refs":[],"action":null});
 assert!(parse(&r.to_string()).is_ok());
 assert!(strict_json("{\"result\":{\"action\":null,\"action\":null}}").is_err());
 let mut a=r.clone();a["action"]=json!({"kind":"suggest_question_disposition","payload":{"question_id":"q-one","question_revision":1,"disposition":"answer","answer":"C","source_message_id":"m-one","source_sha256":"f".repeat(64),"quote_start":0,"quote_end":1}});
 assert!(validate(&a).is_ok());a["schema_version"]=json!(2);assert!(validate(&a).is_err());
 a["schema_version"]=json!(3);a["action"]["payload"]["command"]=json!("do something");assert!(validate(&a).is_err());
 }
}
