// Included in development::tests to reuse its isolated real-Git fixture.
#[test]
fn ollama_http_exact_evidence_continues_coordinator_and_reviewer_without_mutable_reads() {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
    };
    fn roundtrip(mut packet: Value, result: Value, dir: &Path) -> Value {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        packet["provider"] = json!("ollama");
        packet["model"] = json!("fixture:1");
        packet["execution"] = json!({"profile":"development-profiles-v1","provider":"ollama","model":"fixture:1","effort":"provider_default","transport":{"kind":"ollama-http-v1","endpoint":endpoint,"server_version":"fixture","digest":"sha256:fixture","capabilities":["completion"],"options":{"num_ctx":131072,"num_predict":4096}}});
        let expected = packet.clone();
        let reply = result.clone();
        let server = std::thread::spawn(move || {
            for route in ["/api/version", "/api/tags", "/api/chat"] {
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(socket);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert!(line.contains(route));
                let mut length = 0;
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = n.trim().parse::<usize>().unwrap();
                    }
                }
                let mut input = vec![0; length];
                reader.read_exact(&mut input).unwrap();
                let body = match route {
                    "/api/version" => json!({"version":"fixture"}).to_string(),
                    "/api/tags" => {
                        json!({"models":[{"name":"fixture:1","digest":"sha256:fixture"}]})
                            .to_string()
                    }
                    _ => {
                        let input: Value = serde_json::from_slice(&input).unwrap();
                        assert_eq!(input["format"], result_schema(&expected));
                        assert_eq!(
                            serde_json::from_str::<Value>(
                                input["messages"][1]["content"].as_str().unwrap()
                            )
                            .unwrap(),
                            expected
                        );
                        format!(
                            "{}\n",
                            json!({"model":"fixture:1","message":{"content":reply.to_string()},"done":true,"done_reason":"stop"})
                        )
                    }
                };
                write!(
                    reader.into_inner(),
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        write(&dir.join("packet.json"), &packet, true).unwrap();
        crate::ollama::prepare(dir, &packet, instruction(&packet)).unwrap();
        crate::ollama::child(dir).unwrap();
        server.join().unwrap();
        let decoded = read(&dir.join("native-final.json")).unwrap();
        assert_eq!(decoded, result);
        packet
    }
    for role in ["coordinator", "review"] {
        let (_d, service, mut w) = fixture();
        let s = &service.session;
        let tree = PathBuf::from(strv(&w, "integration_worktree").unwrap());
        let base = strv(&w, "head").unwrap().to_owned();
        let head = commit_files(
            &tree,
            &[("source.txt", "frozen source, never mutable replacement\n")],
        );
        w["head"] = json!(head);
        s.development_save("workflow", strv(&w, "work_id").unwrap(), &w, "fixture")
            .unwrap();
        let target = json!({"unit_id":"impl","work_id":w["work_id"],"title":"Exact change","status":"submitted","base_commit":base,"result_commit":head});
        s.development_save("unit", "impl", &target, "fixture")
            .unwrap();
        let mut packet = context_packet(s, &w);
        packet["role"] = json!(role);
        packet["operation_kind"] = json!(if role == "review" {
            "development-review"
        } else {
            "development-coordinator"
        });
        packet["review_targets"] = json!([target]);
        packet["assignment"] = json!({"unit_id":"review"});
        packet["integration_head"] = w["head"].clone();
        let dir = op_dir(s, strv(&w, "work_id").unwrap(), "http-read");
        crate::development_context::prepare(s, &dir, &mut packet).unwrap();
        let mut requests =
            vec![json!({"evidence_id":format!("source:{head}"),"path":"source.txt","cursor":""})];
        if role == "review" {
            let diff = packet["evidence_index"]["entries"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["kind"] == "diff")
                .unwrap();
            requests.push(json!({"evidence_id":diff["id"],"path":"","cursor":""}));
        }
        let mut result = if role == "review" {
            json!({"schema_version":1,"summary":"Need exact source","reviewed_commits":[],"findings":["Preserve unresolved issue"],"requires_changes":true})
        } else {
            json!({"schema_version":1,"assistant_text":"Need exact source","action":"retrieve","tasks":[],"units":[],"checks":[],"resolved_obligations":[],"reason":"Need exact source"})
        };
        result["evidence_requests"] = json!(requests);
        result["context_summary"] = json!("Awaiting exact source");
        result["release_evidence"] = json!([]);
        let packet = roundtrip(packet, result.clone(), &dir);
        commit_files(
            &tree,
            &[("source.txt", "mutable replacement must not be read\n")],
        );
        let requests =
            crate::development_context::validate_requests(&result["evidence_requests"]).unwrap();
        let exchange = crate::development_context::retrieve_with_memory(
            s,
            &dir,
            &packet,
            &requests,
            &Value::Null,
            "Read frozen source",
            &json!(["Preserve unresolved issue"]),
            &[],
        )
        .unwrap();
        let reference = crate::development_context::reference(&dir, &exchange).unwrap();
        w["evidence_context"] = reference.clone();
        w["evidence_pending"] = json!(true);
        s.development_save("workflow", strv(&w, "work_id").unwrap(), &w, "fixture")
            .unwrap();
        let mut next = context_packet(s, &w);
        next["role"] = json!(role);
        next["operation_kind"] = packet["operation_kind"].clone();
        next["assignment"] = json!({"unit_id":"review","evidence_context":reference});
        next["review_targets"] = packet["review_targets"].clone();
        next["integration_head"] = w["head"].clone();
        let dir2 = op_dir(s, strv(&w, "work_id").unwrap(), "http-continue");
        crate::development_context::prepare(s, &dir2, &mut next).unwrap();
        assert!(next["evidence_pages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["path"] == "source.txt"
                && p["data"].as_str().unwrap().contains("frozen source")));
        assert_eq!(
            exchange["unresolved_findings"],
            json!(["Preserve unresolved issue"])
        );
        result["evidence_requests"] = json!([]);
        result["context_summary"] = json!("Frozen source retrieved; unresolved issue retained");
        if role == "review" {
            result["reviewed_commits"] = json!([head]);
        } else {
            result["action"] = json!("decision");
            result["reason"] = json!("Preserve unresolved issue; inspect before acting");
        }
        roundtrip(next, result, &dir2);
        assert!(!dir.join("active-host-attempt.json").exists());
    }
}
