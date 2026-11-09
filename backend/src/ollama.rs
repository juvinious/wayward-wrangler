//! Loopback-only HTTP transport. Discovery never generates; public content only is retained.
use crate::execution::{read, write, write_bytes};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    time::Duration,
};
pub fn endpoint(input: &str) -> Result<String, String> {
    let mut u = reqwest::Url::parse(input).map_err(|_| "Invalid Ollama endpoint")?;
    if u.scheme() != "http"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() != "/"
    {
        return Err(
            "Ollama endpoint must be plain loopback HTTP origin without credentials/path/query"
                .into(),
        );
    }
    if u.host_str() == Some("localhost") {
        u.set_host(Some("127.0.0.1"))
            .map_err(|_| "Invalid loopback host")?;
    }
    let host = u.host_str().unwrap_or("").trim_matches(['[', ']']);
    let ip = host
        .parse::<std::net::IpAddr>()
        .map_err(|_| "Ollama requires a literal loopback address")?;
    if !ip.is_loopback() {
        return Err("Remote/cloud Ollama is unsupported".into());
    }
    Ok(u.as_str().trim_end_matches('/').into())
}
fn client(metadata: bool) -> Result<Client, String> {
    let b = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3));
    let b = if metadata {
        b.timeout(Duration::from_secs(3))
    } else {
        b.timeout(None)
    };
    b.build().map_err(|e| e.to_string())
}
fn metadata(c: &Client, url: &str, body: Option<&Value>) -> Result<Value, String> {
    let r = if let Some(b) = body {
        c.post(url).json(b).send()
    } else {
        c.get(url).send()
    }
    .map_err(|_| "Ollama metadata unavailable (service is not started automatically)")?;
    if !r.status().is_success() {
        return Err(format!("Ollama metadata status {}", r.status()));
    }
    let mut bytes = Vec::new();
    r.take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Ollama metadata exceeds transport limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Ollama metadata JSON".into())
}
pub fn inventory(origin: &str) -> Result<Value, String> {
    let origin = endpoint(origin)?;
    let c = client(true)?;
    let version = metadata(&c, &format!("{origin}/api/version"), None)?;
    let tags = metadata(&c, &format!("{origin}/api/tags"), None)?;
    let mut models = Vec::new();
    for tag in tags["models"].as_array().into_iter().flatten().take(128) {
        let Some(name) = tag["name"].as_str() else {
            continue;
        };
        let detail = metadata(
            &c,
            &format!("{origin}/api/show"),
            Some(&json!({"model":name,"verbose":false})),
        );
        match detail {
            Ok(v) => {
                let limit = v["model_info"]
                    .as_object()
                    .into_iter()
                    .flat_map(|v| v.iter())
                    .filter(|(k, _)| k.ends_with(".context_length"))
                    .filter_map(|(_, v)| v.as_u64())
                    .min();
                let local = v["remote_host"].is_null()
                    && v["remote_model"].is_null()
                    && !name.contains(":cloud")
                    && !name.ends_with("-cloud");
                let completion = v["capabilities"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!("completion")));
                models.push(json!({"model":name,"digest":tag["digest"],"capabilities":v["capabilities"],"thinking":v["thinking"],"context_limit":limit,"local":local,"selectable":local&&completion&&tag["digest"].as_str().is_some(),"suitability":"Unverified for this model and task; adapter support is not quality proof","transport":{"kind":"ollama-http-v1","endpoint":origin,"server_version":version["version"],"digest":tag["digest"],"context_limit":limit,"capabilities":v["capabilities"],"thinking":v["thinking"],"options":{"num_ctx":limit.unwrap_or(8192).min(32768),"num_predict":4096}}}));
            }
            Err(reason) => models.push(json!({"model":name,"selectable":false,"reason":reason})),
        }
    }
    Ok(
        json!({"endpoint":origin,"server_version":version["version"],"models":models,"inference":false,"catalog_complete":tags["models"].as_array().is_none_or(|a|a.len()<=128),"source":"GET /api/version,/api/tags and POST /api/show (metadata only)"}),
    )
}
pub fn validate_profile(p: &Value) -> Result<(), String> {
    let t = &p["transport"];
    if t["kind"] != "ollama-http-v1"
        || endpoint(t["endpoint"].as_str().ok_or("Missing Ollama endpoint")?)?
            != t["endpoint"].as_str().unwrap()
    {
        return Err("Ollama profile needs a canonical loopback HTTP transport".into());
    }
    if p["effort"] != "provider_default"
        || p["model"].as_str().is_none_or(|m| m.contains("cloud"))
        || t["digest"].as_str().is_none_or(str::is_empty)
        || t["server_version"].as_str().is_none_or(str::is_empty)
        || !t["capabilities"]
            .as_array()
            .is_some_and(|a| a.contains(&json!("completion")))
    {
        return Err(
            "Ollama profile lacks frozen local model identity/capability; refresh discovery".into(),
        );
    }
    let ctx = t["options"]["num_ctx"]
        .as_u64()
        .ok_or("Choose native num_ctx")?;
    let output = t["options"]["num_predict"]
        .as_u64()
        .ok_or("Choose response headroom")?;
    if output == 0 || ctx <= output || t["context_limit"].as_u64().is_some_and(|max| ctx > max) {
        return Err("Invalid Ollama context/headroom options".into());
    }
    for key in t["options"]
        .as_object()
        .ok_or("Native options must be an object")?
        .keys()
    {
        if !matches!(
            key.as_str(),
            "num_ctx" | "num_predict" | "temperature" | "seed" | "top_p" | "top_k"
        ) {
            return Err("Unsupported native Ollama option; no silent mapping".into());
        }
    }
    Ok(())
}
pub fn preflight(p: &Value) -> Result<(), String> {
    validate_profile(p)?;
    let t = &p["transport"];
    let c = client(true)?;
    let origin = t["endpoint"].as_str().unwrap();
    let version = metadata(&c, &format!("{origin}/api/version"), None)?;
    let tags = metadata(&c, &format!("{origin}/api/tags"), None)?;
    if version["version"] != t["server_version"]
        || !tags["models"].as_array().is_some_and(|a| {
            a.iter().any(|m| {
                (m["name"] == p["model"] || m["model"] == p["model"]) && m["digest"] == t["digest"]
            })
        })
    {
        return Err(
            "Ollama server/model identity changed before send; refresh and explicitly select it"
                .into(),
        );
    }
    Ok(())
}
pub fn prepare(
    dir: &Path,
    p: &Value,
    instruction: &str,
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    if p["role"] == "implementation" || p["operation_kind"] == "development-worker" {
        return Err("Ollama coding workers are unsupported".into());
    }
    let schema = crate::development::result_schema(p);
    let input = json!({"model":p["model"],"messages":[{"role":"system","content":instruction},{"role":"user","content":serde_json::to_string(p).map_err(|e|e.to_string())?}],"format":schema,"stream":true,"options":p["execution"]["transport"]["options"]});
    let bytes = serde_json::to_vec(&input).map_err(|e| e.to_string())?.len();
    let ctx = p["execution"]["transport"]["options"]["num_ctx"]
        .as_u64()
        .unwrap_or(0);
    let output = p["execution"]["transport"]["options"]["num_predict"]
        .as_u64()
        .unwrap_or(0);
    let estimate = (bytes as u64).div_ceil(3) + 256;
    write(
        &dir.join("ollama-context.json"),
        &json!({"input_bytes":bytes,"estimated_input_tokens":estimate,"estimate":"UTF-8 bytes / 3 plus framing; not a tokenizer guarantee","context_tokens":ctx,"response_headroom":output,"includes":"instructions, schema, packet and supplied tool results","required_input_truncated":false}),
        true,
    )?;
    if estimate + output > ctx {
        return Err(format!(
            "Ollama estimated input {estimate} plus response headroom {output} exceeds selected num_ctx {ctx}; required input retained"
        ));
    }
    write(&dir.join("ollama-request.json"), &input, true)?;
    let runner = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner = if runner.file_name().and_then(|n| n.to_str()) == Some("waywr-runner") {
        runner
    } else {
        runner.with_file_name("waywr-runner")
    };
    Ok((
        vec![
            runner.to_string_lossy().into(),
            "ollama-child".into(),
            dir.to_string_lossy().into(),
        ],
        BTreeMap::new(),
    ))
}
fn emit(v: &Value) -> Result<(), String> {
    println!("{v}");
    std::io::stdout().flush().map_err(|e| e.to_string())
}
pub fn child(dir: &Path) -> Result<(), String> {
    let p = read(&dir.join("packet.json"))?;
    if dir.join("cancel.json").exists(){return Err("Cancelled before Ollama send; no HTTP generation requested".into())}
    preflight(&p["execution"])?;
    if dir.join("cancel.json").exists(){return Err("Cancelled during preflight; no HTTP generation requested".into())}
    let request = read(&dir.join("ollama-request.json"))?;
    let endpoint = p["execution"]["transport"]["endpoint"].as_str().unwrap();
    write(
        &dir.join("ollama-send-intent.json"),
        &json!({"endpoint":endpoint,"model":p["model"],"digest":p["execution"]["transport"]["digest"],"server_completion":"unknown until terminal response"}),
        true,
    )?;
    let response = client(false)?
        .post(format!("{endpoint}/api/chat"))
        .json(&request)
        .send()
        .map_err(|_| "Ollama response lost after send intent; no blind replay")?;
    if !response.status().is_success() {
        emit(&json!({"type":"ollama.refused","http_status":response.status().as_u16()}))?;
        return Err("Ollama request refused; inspect HTTP status (body omitted)".into());
    }
    decode_stream(BufReader::new(response), &p, dir)
}
pub fn decode_stream(mut stream: impl BufRead, p: &Value, dir: &Path) -> Result<(), String> {
    let mut content = String::new();
    let mut terminal = None;
    loop {
        let mut bytes = Vec::new();
        let n = (&mut stream)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| "Ollama stream interrupted; remote completion unknown")?;
        if n == 0 {
            break;
        }
        if n > 1024 * 1024 {
            return Err("Ollama stream chunk exceeds transport memory limit".into());
        }
        let v: Value = serde_json::from_slice(&bytes)
            .map_err(|_| "Invalid Ollama stream JSON (raw chunk not retained)")?;
        if terminal.is_some() {
            return Err("Ollama emitted data after terminal completion".into());
        }
        if v["model"] != p["model"] || !v["error"].is_null() {
            return Err("Ollama model identity mismatch or provider error".into());
        }
        if v["message"]["tool_calls"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
        {
            return Err("Ollama coordinator/reviewer has no native tool authority".into());
        }
        if let Some(public) = v["message"]["content"].as_str() {
            content.push_str(public);
            if content.len() > 65536 {
                return Err("Ollama structured public result exceeds output memory limit".into());
            }
        } // Thinking/hidden fields are discarded before persistence.
        if v["done"] == true {
            if v["done_reason"] != "stop" {
                return Err("Ollama result ended without a complete stop".into());
            }
            terminal = Some(
                json!({"type":"ollama.completed","model":v["model"],"prompt_eval_count":v["prompt_eval_count"],"eval_count":v["eval_count"],"done_reason":v["done_reason"]}),
            );
        }
    }
    let terminal = terminal
        .ok_or("Ollama stream ended without terminal completion; remote completion unknown")?;
    emit(&terminal)?;
    let result = crate::scope_contract::strict_json(content.trim())?;
    crate::output::validate(&result, p)?;
    write_bytes(
        &dir.join("native-final.json"),
        content.trim().as_bytes(),
        true,
    )?;
    emit(
        &json!({"type":"ollama.public_result","result_sha256":crate::codec::sha256(content.trim().as_bytes())}),
    )?;
    Ok(())
}
pub fn finish(
    events: &[Value],
    dir: &Path,
    p: &Value,
    code: Option<i32>,
) -> Result<(Value, Value), String> {
    let completed = events
        .iter()
        .filter(|e| e["type"] == "ollama.completed")
        .collect::<Vec<_>>();
    if code != Some(0) || completed.len() != 1 || completed[0]["model"] != p["model"] {
        return Err("Ollama lacks one validated successful terminal response".into());
    }
    let result = crate::scope_contract::strict_json(
        &std::fs::read_to_string(dir.join("native-final.json")).map_err(|e| e.to_string())?,
    )?;
    crate::output::validate(&result, p)?;
    Ok((
        result,
        json!({"provider":"ollama","transport":"ollama-http-v1","model":p["model"],"identity":p["execution"]["transport"],"usage":completed[0],"cost":"local/unknown","complete":true}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ollama_loopback_and_model_roles_are_explicit() {
        assert!(endpoint("http://example.com:11434").is_err());
        assert!(endpoint("http://127.0.0.1:11434/redirect").is_err());
        assert!(endpoint("http://token@127.0.0.1:11434").is_err());
        assert_eq!(
            endpoint("http://localhost:11434").unwrap(),
            "http://127.0.0.1:11434"
        );
        let profile = json!({"provider":"ollama","model":"qwen3:8b","effort":"provider_default","transport":{"kind":"ollama-http-v1","endpoint":"http://127.0.0.1:11434","server_version":"fixture","digest":"sha256:fixture","capabilities":["completion"],"options":{"num_ctx":8192,"num_predict":2048}}});
        assert!(validate_profile(&profile).is_ok());
        let mut bad = profile;
        bad["effort"] = json!("medium");
        assert!(validate_profile(&bad).is_err());
    }
    #[test]
    fn ollama_role_capabilities_and_profile_encoding_remain_explicit() {
        let native = json!({"provider":"codex","binary":"/bin/true","model":"gpt-6.1-sol","effort":"medium","binary_sha256":null,"context_bytes":98304});
        let local = json!({"provider":"ollama","binary":"","model":"qwen3:8b","effort":"provider_default","context_bytes":98304,"transport":{"kind":"ollama-http-v1","endpoint":"http://127.0.0.1:11434","server_version":"fixture","digest":"sha256:fixture","capabilities":["completion"],"options":{"num_ctx":8192,"num_predict":2048}}});
        let mut prefs = json!({"schema_version":2,"profile":"development-profiles-v1","profiles":{"codex":native,"local":local},"coordinator":"codex","worker_default":"codex","roles":{"coordinator":{"default":"codex","allowed":["codex","local"]},"review":{"default":"local","allowed":["local"]},"implementation":{"default":"codex","allowed":["codex"]}},"fallback":{},"concurrency":2,"dependency_network":false,"main_policy":"checked-fast-forward","quality_preference":""});
        let parsed = crate::provider_selection::Preferences::parse(&prefs).unwrap();
        assert_eq!(parsed.value()["profiles"]["codex"], native);
        assert!(
            parsed.value()["profiles"]["codex"]
                .get("transport")
                .is_none()
        );
        prefs["roles"]["implementation"]["allowed"] = json!(["codex", "local"]);
        assert!(
            crate::provider_selection::Preferences::parse(&prefs)
                .unwrap_err()
                .contains("coding workers")
        );
    }
    #[test]
    fn ollama_stream_public_only_exact_terminal_and_unknown_loss() {
        let d = tempfile::tempdir().unwrap();
        let packet = json!({"model":"qwen3:8b","provider":"ollama","development":true,"operation_kind":"development-review","role":"review","review_targets":[{"result_commit":"abc"}]});
        let result = json!({"schema_version":1,"summary":"No issues in supplied diff","reviewed_commits":["abc"],"requires_changes":false,"findings":[]});
        let data = format!(
            "{}\n{}\n",
            json!({"model":"qwen3:8b","message":{"content":result.to_string(),"thinking":"SECRET HIDDEN REASONING"},"done":false}),
            json!({"model":"qwen3:8b","done":true,"done_reason":"stop","eval_count":22})
        );
        decode_stream(std::io::Cursor::new(data), &packet, d.path()).unwrap();
        assert!(
            !std::fs::read_to_string(d.path().join("native-final.json"))
                .unwrap()
                .contains("SECRET")
        );
        let empty = tempfile::tempdir().unwrap();
        assert!(
            decode_stream(std::io::Cursor::new(""), &packet, empty.path())
                .unwrap_err()
                .contains("unknown")
        );
        assert!(!empty.path().join("native-final.json").exists());
    }
    #[test]
    fn ollama_metadata_http_does_not_generate_or_redirect() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            for expected in ["/api/version", "/api/tags"] {
                let (mut socket, _) = listener.accept().unwrap();
                let mut bytes = [0; 8192];
                let n = socket.read(&mut bytes).unwrap();
                let req = String::from_utf8_lossy(&bytes[..n]);
                assert!(req.starts_with(&format!("GET {expected} ")));
                let body = if expected.ends_with("version") {
                    json!({"version":"fixture"})
                } else {
                    json!({"models":[]})
                }
                .to_string();
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let observed = inventory(&origin).unwrap();
        assert_eq!(observed["inference"], false);
        assert_eq!(observed["server_version"], "fixture");
        thread.join().unwrap();
    }
}
