//! Read-only, bounded presentation of current native tool activity. Never output.
use serde_json::{Value, json};
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};

const TAIL_BYTES: u64 = 256 * 1024;

fn unavailable() -> Value { json!({"status":"unavailable","label":"Activity details unavailable"}) }

fn short(value: &str) -> String {
    let first = value.lines().next().unwrap_or("").trim();
    let mut text: String = first.chars().take(200)
        .map(|c| if c.is_control() { ' ' } else { c }).collect();
    if first.chars().count() > 200 || value.lines().count() > 1 { text.push('…'); }
    text
}

fn item_label(item: &Value) -> String {
    match item["type"].as_str().unwrap_or("") {
        "command_execution" => item["command"].as_str().map(short).filter(|s| !s.is_empty())
            .map(|command| format!("Running: {command}")).unwrap_or_else(|| "Running command".into()),
        "file_change" => "Editing files".into(),
        "mcp_tool_call" => {
            let tool = [item["server"].as_str(), item["tool"].as_str()].into_iter()
                .flatten().map(short).collect::<Vec<_>>().join("/");
            if tool.is_empty() { "Calling tool".into() } else { format!("Calling tool: {tool}") }
        }
        "web_search" => "Searching web".into(),
        _ => "Working".into(),
    }
}

fn parse_tail(bytes: &[u8], truncated: bool) -> Value {
    let mut active: Vec<(String, String)> = Vec::new();
    let mut reported=truncated; // Older omitted frames cannot establish initial preparation.
    let mut lines = bytes.split_inclusive(|b| *b == b'\n');
    // A tail can begin inside a frame. It cannot establish activity from that frame.
    if truncated { lines.next(); }
    for line in lines {
        if !line.ends_with(b"\n") { break; } // concurrent writer's incomplete frame
        if line.iter().all(u8::is_ascii_whitespace) { continue; }
        let Ok(event) = serde_json::from_slice::<Value>(line) else { return unavailable(); };
        match event["type"].as_str().unwrap_or("") {
            "turn.started" => active.clear(),
            "turn.completed" | "turn.failed" => {reported=true;active.clear();},
            "assistant" => {
                for tool in event["message"]["content"].as_array().into_iter().flatten().filter(|v|v["type"]=="tool_use"){
                    let Some(id)=tool["id"].as_str()else{continue};active.retain(|(other,_)|other!=id);
                    let label=match tool["name"].as_str(){
                        Some("Bash")=>tool["input"]["command"].as_str().map(short).map(|c|format!("Running: {c}")).unwrap_or_else(||"Running command".into()),
                        Some("Read")=>"Reading file".into(),Some("Edit"|"Write")=>"Editing files".into(),Some("Glob"|"Grep")=>"Searching files".into(),_=>continue};
                    reported=true;active.push((id.into(),label));
                }
            },
            "user"=>{for result in event["message"]["content"].as_array().into_iter().flatten().filter(|v|v["type"]=="tool_result"){if let Some(id)=result["tool_use_id"].as_str(){active.retain(|(other,_)|other!=id);}}},
            "result"=>{reported=true;active.clear();},
            "item.started" | "item.updated" | "item.completed" => {
                let item = &event["item"];
                if matches!(item["type"].as_str(),Some("command_execution"|"file_change"|"mcp_tool_call"|"web_search")){reported=true;}
                let Some(id) = item["id"].as_str() else { continue; };
                active.retain(|(other, _)| other != id);
                if event["type"] != "item.completed" &&
                    matches!(item["type"].as_str(), Some("command_execution" | "file_change" | "mcp_tool_call" | "web_search")) &&
                    !matches!(item["status"].as_str(), Some("completed" | "failed" | "declined")) {
                    active.push((id.into(), item_label(item)));
                }
            }
            _ => {}
        }
    }
    if let Some((_, label)) = active.last() {
        let suffix = if active.len() > 1 { format!(" (+{} operations)", active.len() - 1) } else { String::new() };
        json!({"status":"running","label":format!("{label}{suffix}")})
    } else if !reported {
        json!({"status":"preparing","label":"Preparing · waiting for a reported operation"})
    } else {
        // A start older than the bounded tail is deliberately not reconstructed.
        json!({"status":"working","label":"Working · no current operation reported"})
    }
}

pub(crate) fn current(directory: &Path) -> Value {
    if let Ok(v)=crate::execution::read(&directory.join("runtime-progress.json")) {
        let activity=&v["activity"];
        return json!({"status":activity["status"].as_str().unwrap_or("working"),
            "label":activity["label"].as_str().unwrap_or("Working"),"usage":v["usage"]});
    }
    if directory.join("receipt.json").exists() {
        return json!({"status":"finishing","label":"Finishing · result ready"});
    }
    let mut file = match File::open(directory.join("native-events.jsonl")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
            return json!({"status":"starting","label":"Preparing · waiting for a reported operation"}),
        Err(_) => return unavailable(),
    };
    let Ok(size) = file.metadata().map(|m| m.len()) else { return unavailable(); };
    let start = size.saturating_sub(TAIL_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() { return unavailable(); }
    let mut bytes = Vec::new();
    if file.take(size - start).read_to_end(&mut bytes).is_err() { return unavailable(); }
    parse_tail(&bytes, start > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(kind: &str, id: &str, command: &str) -> String {
        format!("{}\n", json!({"type":kind,"item":{"id":id,"type":"command_execution",
            "command":command,"aggregated_output":"OUTPUT MUST NOT APPEAR","status":"in_progress"}}))
    }
    #[test]fn claude_activity_reports_operation_without_output_or_reasoning(){
        let start=json!({"type":"assistant","message":{"content":[{"type":"thinking","thinking":"PRIVATE"},{"type":"tool_use","id":"a","name":"Bash","input":{"command":"cargo test"}}]}}).to_string()+"\n";
        let view=parse_tail(start.as_bytes(),false);assert_eq!(view["label"],"Running: cargo test");assert!(!view.to_string().contains("PRIVATE"));
        let result=json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"a","content":"PRIVATE OUTPUT"}]}}).to_string()+"\n";
        assert_eq!(parse_tail((start+&result).as_bytes(),false)["status"],"working");
    }
    #[test]
    fn worker_activity_preparing_until_first_public_operation() {
        let waiting=b"{\"type\":\"turn.started\"}\n{\"type\":\"item.completed\",\"item\":{\"id\":\"message\",\"type\":\"agent_message\",\"text\":\"PRIVATE\"}}\n";
        let preparing=parse_tail(waiting,false);assert_eq!(preparing["status"],"preparing");assert_eq!(preparing["label"],"Preparing · waiting for a reported operation");assert!(!preparing.to_string().contains("PRIVATE"));
        let events=format!("{}{}",String::from_utf8_lossy(waiting),frame("item.started","cmd","cargo test"));assert_eq!(parse_tail(events.as_bytes(),false)["status"],"running");
        let events=events+&frame("item.completed","cmd","cargo test");assert_eq!(parse_tail(events.as_bytes(),false)["status"],"working");
        assert_eq!(parse_tail(b"partial\n",true)["status"],"working","A truncated tail cannot prove this is initial preparation");
    }
    #[test]
    fn worker_activity_tracks_parallel_commands_and_completion_without_output() {
        let mut events = frame("item.started", "a", "cargo test");
        events += &frame("item.started", "b", "git status");
        let view = parse_tail(events.as_bytes(), false);
        assert_eq!(view["label"], "Running: git status (+1 operations)");
        assert!(!view.to_string().contains("OUTPUT"));
        events += &frame("item.completed", "a", "cargo test");
        assert_eq!(parse_tail(events.as_bytes(), false)["label"], "Running: git status");
        events += &frame("item.completed", "b", "git status");
        assert_eq!(parse_tail(events.as_bytes(), false)["status"], "working");
    }
    #[test]
    fn worker_activity_bounds_and_partial_frames_do_not_resurrect_commands() {
        let events = format!("partial frame\n{}{}", frame("item.completed", "a", "old"),
            frame("item.started", "b", "new").trim_end());
        assert_eq!(parse_tail(events.as_bytes(), true)["status"], "working");
        assert_eq!(parse_tail(b"invalid\n", false)["status"], "unavailable");
        let events = frame("item.started", "a", "python3 - <<'PY'\nsecret implementation\nPY");
        assert_eq!(parse_tail(events.as_bytes(), false)["label"], "Running: python3 - <<'PY'…");
        let events = format!("{events}{{\"type\":\"turn.completed\"}}\n");
        assert_eq!(parse_tail(events.as_bytes(), false)["status"], "working");
    }
    #[test]
    fn worker_activity_uses_only_selected_attempt_and_terminal_receipt() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(current(root.path())["status"], "starting");
        let mut events = vec![b'x'; TAIL_BYTES as usize + 100]; events.push(b'\n');
        events.extend(frame("item.started", "a", "python3 -m unittest").as_bytes());
        std::fs::write(root.path().join("native-events.jsonl"), events).unwrap();
        assert_eq!(current(root.path())["label"], "Running: python3 -m unittest");
        std::fs::write(root.path().join("receipt.json"), "{}").unwrap();
        assert_eq!(current(root.path())["status"], "finishing");
    }
    #[test]
    fn worker_activity_never_exposes_messages_reasoning_or_tool_arguments() {
        for kind in ["agent_message", "reasoning", "mcp_tool_call", "file_change", "web_search"] {
            let event = json!({"type":"item.started","item":{"id":"a","type":kind,
                "text":"PRIVATE TEXT","arguments":"PRIVATE ARGS","changes":"PRIVATE DIFF",
                "result":"PRIVATE RESULT","server":"local","tool":"inspect"}});
            let value = parse_tail(format!("{event}\n").as_bytes(), false);
            assert!(!value.to_string().contains("PRIVATE"));
        }
    }
}
