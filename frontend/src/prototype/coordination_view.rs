// Application-owned conversational context. Inspection never changes the composer owner.
impl App {
    pub fn coordination_mode(&self) -> bool {
        self.live
            .as_ref()
            .and_then(|l| l.conversation.as_ref())
            .is_some_and(|c| c["coordination"]["version"] == 1)
    }
    fn coordination_request(&self) -> Option<&Value> {
        self.live.as_ref()?.conversation.as_ref()?["coordination"]["requests"]
            .as_array()?
            .iter()
            .find(|r| r["subject_id"] == self.conversation_subject)
    }
    fn coordination_card(&self) -> Option<&Value> {
        self.coordination_request()?
            .get("card")
            .filter(|c| c["ready"] == true)
    }
    fn coordination_status(&self) -> String {
        if self.live.as_ref().is_some_and(|l| l.phase != "running") {
            return "Paused".into();
        }
        let rows = self.queue_rows();
        if rows
            .iter()
            .any(|r| r["subject_id"] == self.conversation_subject && r["status"] == "reserved")
        {
            return "Preparing plan / replying".into();
        }
        if rows
            .iter()
            .any(|r| r["subject_id"] == self.conversation_subject && r["status"] == "queued")
        {
            return "Waiting for coordinator".into();
        }
        self.coordination_request()
            .and_then(|r| r["status"].as_str())
            .unwrap_or("Discussing")
            .into()
    }
    fn coordination_context(&self, source: &str) -> Value {
        serde_json::json!({"version":1,"subject_id":self.conversation_subject,"request_id":self.coordination_request().map(|r|r["request_id"].clone()),"card_id":self.coordination_card().map(|c|c["card_id"].clone()),"reply_to_question":self.pending_conversation_question().map(|q|serde_json::json!({"question_id":q["question_id"],"revision":q["scope"]["question_revision"]})),"candidate":self.development_workflow().filter(|w|w["candidate"]["presented"]==true).map(|w|serde_json::json!({"work_id":w["work_id"],"candidate_id":w["candidate"]["candidate_id"],"commit":w["candidate"]["commit"]})),"development_question":self.development_workflow().filter(|w|!w["question"].is_null()).map(|w|serde_json::json!({"work_id":w["work_id"],"revision":w["revision"],"question_sha256":waywr_backend::codec::canonical_sha256(&w["question"]).unwrap_or_default()})),"provider_card_id":self.coordination_request().map(|r|r["provider_grant"]["card_id"].clone()),"provenance":{"kind":source,"deliberate_submission":true}})
    }
    fn role_summary(policy: &Value, role: &str) -> String {
        let pin = &policy["request_roles"][role];
        let selection = if pin.is_null() {
            &policy["routing"]["defaults"][role]
        } else {
            pin
        };
        let profile = if selection["profile"].is_object() {
            &selection["profile"]
        } else if role == "coordinator" {
            &policy["coordinator"]
        } else {
            &policy["workers"]
        };
        format!(
            "{}: {} · {} / {}{}",
            match role {
                "coordinator" => "Coordinator",
                "implementation" => "Implementation",
                _ => "Reviewer",
            },
            match profile["provider"].as_str() {
                Some("claude") => "Claude",
                Some("codex") => "Codex",
                Some(v) => v,
                None => "unconfigured",
            },
            profile["model"].as_str().unwrap_or("?"),
            profile["effort"].as_str().unwrap_or("?"),
            if pin.is_null() {
                " (default)"
            } else {
                " (fixed for this request)"
            }
        )
    }
    fn proposal_card_text(card: &Value) -> String {
        let p = &card["proposal"];
        let policy = &card["policy"];
        let short = |value: &Value, n: usize| {
            let text = value.as_str().unwrap_or("");
            let mut s = text.chars().take(n).collect::<String>();
            if text.chars().count() > n {
                s.push('…');
            }
            s
        };
        let mut text = format!(
            "{} · proposal v{}\n{}\n",
            p["title"].as_str().unwrap_or("Request"),
            card["identity"]["base"]["revision"],
            short(&p["objective"], 240)
        );
        for deliverable in p["deliverables"].as_array().into_iter().flatten().take(3) {
            text.push_str(&format!("• {}\n", short(deliverable, 110)));
        }
        text.push_str(&format!(
            "{} deliverables · {} acceptance checks · Details shows the complete scope.\n",
            p["deliverables"].as_array().map_or(0, Vec::len),
            p["acceptance_criteria"].as_array().map_or(0, Vec::len)
        ));
        for role in ["coordinator", "implementation", "review"] {
            text.push_str(&Self::role_summary(policy, role));
            text.push('\n');
        }
        if let Some(hashes) = policy["routing"]["roles"]["review"]
            .as_array()
            .filter(|a| a.len() > 1)
        {
            let names = hashes
                .iter()
                .filter_map(|h| policy["routing"]["profiles"][h.as_str()?]["name"].as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!(
                "Review permission: {names}. The effective reviewer above is enforced.\n"
            ));
        }
        if policy["project_workflow"]["generated_outputs"].as_array().is_some_and(|a|!a.is_empty()){text.push_str(&format!("Generated output roots eligible for inventoried retirement: {}\n",policy["project_workflow"]["generated_outputs"]));}
        if policy["project_workflow"].is_object(){let p=&policy["project_workflow"];text.push_str(&format!("Baseline: {} ({}) → target {} · {}\nHuman validation before integration: {} · retention {}\n{}\n",p["baseline"],p["baseline_input"],p["target"],p["completion"],p["human_validation"],p["retention"],if p["ready"]==true{"Setup ready; approval includes these exact choices".into()}else{format!("Setup needs attention: {} · F3 → 7 Project",p["blockers"])}));}
        let fallback = policy["fallback"].as_object().map_or(
            policy["fallback"].as_array().is_some_and(|a| !a.is_empty()),
            |roles| {
                roles
                    .values()
                    .any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
            },
        );
        text.push_str(&format!("Policy: dependency downloads {}; selected integration policy above (legacy: checked local main); Concurrent workers: {}; fallback {}.\n",if policy["dependency_network"]==true{"allowed"}else{"disabled"},policy["workers"]["concurrency"],if fallback{"explicit configured alternatives"}else{"disabled; wait for capacity"}));
        for unresolved in p["unresolved_decisions"].as_array().into_iter().flatten() {
            text.push_str(&format!(
                "Unresolved — approval acknowledges: {}\n",
                unresolved.as_str().unwrap_or("")
            ));
        }
        if policy["project_workflow"]["completion"]=="github-pr"{text.push_str(&format!("Publication authority: repository {} · remote {} · base {} · {} merge, normal GitHub gates.\n",policy["project_workflow"]["repository"],policy["project_workflow"]["remote"],policy["project_workflow"]["target"],policy["project_workflow"]["merge_method"]));}else{text.push_str("Scoped edits, tests, local commits and reviewed integration; no remote publication or unrelated projects.\n");}text.push_str("Reply approve, request a change, or expand Details below.");
        text
    }
    fn present_visible_card(&mut self, top: usize, height: usize, width: u16) {
        if !self.coordination_mode() || height == 0 || self.journal.connection != "connected" {
            return;
        }
        if self.modal != Modal::None && !(self.modal == Modal::Scope && self.scope_step == 3) {
            return;
        }
        let Some(card) = self.coordination_card().cloned() else {
            return;
        };
        let Some(id) = card["card_id"].as_str() else {
            return;
        };
        if card["presented"] == true || self.card_presenting.as_deref() == Some(id) {
            return;
        }
        if self.modal == Modal::Scope && self.scope_step == 3 {
            self.card_presenting = Some(id.into());
            self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"coordination.present","payload":{"subject_id":self.conversation_subject,"card_id":id}})));
            return;
        }
        let mut line = 0;
        for message in self
            .journal
            .messages
            .iter()
            .filter(|m| Self::visible_message(m))
        {
            let count = Paragraph::new(self.message_text(message, width))
                .wrap(Wrap { trim: false })
                .line_count(width.max(1));
            if self.card_seqs.get(&message.seq).is_some_and(|v| v == id)
                && line < top + height
                && line + count > top
            {
                self.card_presenting = Some(id.into());
                self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"coordination.present","payload":{"subject_id":self.conversation_subject,"card_id":id}})));
                break;
            }
            line += count;
        }
    }
    fn inline_card_action(&mut self) {
        match self.card_action {
            1 => self.submit_inline_approval("inline_approve"),
            2 => {
                self.focus = Focus::Composer;
                self.notice = "Describe the change to this proposal; approval stays pending".into();
            }
            3 | 4 => {
                let Some(card) = self.coordination_card() else {
                    return;
                };
                let value = serde_json::json!({"action":"coordination.disposition","payload":{"subject_id":self.conversation_subject,"card_id":card["card_id"],"disposition":if self.card_action==3{"defer"}else{"reject"}}});
                self.intent(LiveIntent::Dialogue(value));
            }
            _ => self.open(Modal::Scope),
        }
        self.mark();
    }
    fn coordination_event(&mut self, kind: &str, body: &Value) {
        if kind == "conversation.turn_completed" || kind == "conversation.turn_stopped" {
            if let Some(id) = body["turn_id"].as_str() {
                self.finished_turns.push_back(id.into());
                while self.finished_turns.len() > 64 {
                    self.finished_turns.pop_front();
                }
                if self.delivery_turn.as_deref() == Some(id) {
                    self.notice = if kind == "conversation.turn_completed" {
                        "Reply complete"
                    } else {
                        "Reply stopped; inspect this request"
                    }
                    .into();
                    self.delivery_turn = None;
                }
            }
        }
        if kind == "conversation.answer_continuation" {
            if body["subject_id"] != self.conversation_subject {
                return;
            }
            self.delivery_turn = body["turn_id"].as_str().map(str::to_owned);
            self.notice = "Answer saved; waiting for coordinator".into();
        }
        if kind == "conversation.turn_reserved"
            && self.delivery_turn.as_deref() == body["turn_id"].as_str()
        {
            self.notice = "Coordinator replying…".into();
        }
        if kind == "conversation.input_result" {
            if let Some(id) = body["input_id"].as_str() {
                self.finished_inputs.push_back(id.into());
                while self.finished_inputs.len() > 64 {
                    self.finished_inputs.pop_front();
                }
            }
            if body["submitted_subject"] != self.conversation_subject
                && body["subject_id"] != self.conversation_subject
            {
                return;
            }
            if body["submitted_subject"] == self.conversation_subject
                && body["subject_id"]
                    .as_str()
                    .is_some_and(|s| s != self.conversation_subject)
            {
                self.conversation_subject = body["subject_id"].as_str().unwrap().into();
                self.card_action = 0;
            }
            if let Some(turn) = body["outcome"]["turn_id"].as_str() {
                if self.finished_turns.iter().any(|v| v == turn) {
                    self.delivery_turn = None;
                    self.notice = "Reply complete".into();
                    return;
                }
                self.delivery_turn = Some(turn.into());
            }
            self.notice = if body["outcome"]["status"] == "rejected" {
                format!(
                    "Input refused: {}",
                    body["outcome"]["message"]
                        .as_str()
                        .unwrap_or("Inspect retained input")
                )
            } else if body["route"] == "authorize_scope" {
                "Proposal approved".into()
            } else if body["route"] == "answer_conversation" {
                if body["outcome"]["question"]["disposition"] == "defer" {
                    "Question deferred"
                } else {
                    "Answer saved; preparing proposal"
                }
                .into()
            } else if body["route"] == "clarify_provider" {
                "Name one exact configured profile or use F3".into()
            } else if body["route"] == "clarify_approval" {
                "Choose Approve for the displayed proposal, or describe a change".into()
            } else {
                "Message accepted".into()
            };
        }
    }
    pub fn message_acknowledgement(&mut self, id: &str, out: &Value) -> bool {
        if out["input_id"].is_string()&&out["delivery"]=="pending"{
            self.notice="Steering pending · applies at the next agent boundary".into();return true;
        }
        if out["input_id"].is_string() {
            if !self.finished_inputs.iter().any(|v| v == id) {
                self.notice = "Message received; resolving its saved request context".into();
            }
            return true;
        }
        if let Some(turn) = out["turn_id"].as_str() {
            let status = self
                .live
                .as_ref()
                .and_then(|l| l.conversation.as_ref())
                .and_then(|c| c["turns"].as_array())
                .and_then(|a| a.iter().find(|t| t["turn_id"] == turn))
                .and_then(|t| t["status"].as_str());
            self.notice =
                if self.finished_turns.iter().any(|v| v == turn) || status == Some("completed") {
                    self.delivery_turn = None;
                    "Reply complete"
                } else if status == Some("reserved") {
                    self.delivery_turn = Some(turn.into());
                    "Coordinator replying…"
                } else {
                    self.delivery_turn = Some(turn.into());
                    "Message accepted; waiting for coordinator"
                }
                .into();
            return true;
        }
        false
    }
    fn discuss_selected_request(&mut self) {
        let subject = self
            .live
            .as_ref()
            .and_then(|l| l.selected(self.journal.selected_task))
            .and_then(|t| t["task_id"].as_str())
            .and_then(|task| {
                self.live.as_ref()?.conversation.as_ref()?["continuous"]["subjects"]
                    .as_array()?
                    .iter()
                    .find(|s| s["task_id"] == task)?["subject_id"]
                    .as_str()
            })
            .map(str::to_owned);
        if let Some(subject) = subject {
            if let Some(original) = self.input_history.take_original() {
                self.composer = Composer::from_draft(original);
                self.draft_reply = self.history_reply.take().filter(|v| !v.is_null());
            }
            let old = format!(
                "subject:{}:{}",
                self.conversation_subject,
                Self::request_draft_slot(self.journal.target)
            );
            if let Some(context) = self.draft_reply.take() {
                self.draft_replies.insert(old.clone(), context);
            }
            self.journal.drafts.insert(old, self.composer.snapshot());
            self.conversation_subject = subject;
            self.journal.target = Target::Coordinator;
            let key = format!(
                "subject:{}:{}",
                self.conversation_subject,
                Self::request_draft_slot(self.journal.target)
            );
            self.composer =
                Composer::from_draft(self.journal.drafts.get(&key).cloned().unwrap_or_default());
            self.draft_reply = self.draft_replies.get(&key).cloned();
            self.freeze_unbound_draft();
            self.card_action = 0;
            self.focus = Focus::Composer;
            self.mark();
        } else {
            self.notice = "The inspected task has no linked request".into();
        }
    }
}

#[cfg(test)]
mod coordination_ui_tests {
    use super::*;
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    pub(super) fn app() -> App {
        let mut a = tests::continuous_app();
        let scope = a.live.as_ref().unwrap().detail.as_ref().unwrap()["scope"].clone();
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"] = serde_json::json!({"version":1,"requests":[{"request_id":"old","subject_id":"discussion","task_id":"t-scope","scope":scope,"status":"Awaiting your approval","card":null},{"request_id":"new","subject_id":"discussion-new","task_id":null,"scope":null,"status":"Discussing","card":null}]});
        a
    }
    #[test]
    fn coordination_unbound_owner_survives_inspection_targets_and_restart() {
        let mut a = app();
        a.conversation_subject = "discussion-new".into();
        a.input(Event::Paste("My new draft".into()));
        a.set_target(Target::Coordinator);
        assert_eq!(a.conversation_subject, "discussion-new");
        assert_eq!(a.composer.text(), "My new draft");
        assert!(a.current_scope().is_none());
        a.select_task(0);
        a.open(Modal::Settings);
        a.input(key(KeyCode::Esc));
        assert_eq!(a.message_payload("answer")["subject_id"], "discussion-new");
        let state = a.conversation_ui_state();
        let mut b = app();
        b.restore_conversation_ui(&state).unwrap();
        assert_eq!(b.conversation_subject, "discussion-new");
        assert_eq!(b.composer.text(), "My new draft");
        assert!(b.current_scope().is_none());
        b.discuss_selected_request();
        assert_eq!(b.conversation_subject, "discussion");
        assert!(b.current_scope().is_some());
    }
    #[test]
    fn coordination_completed_turn_wins_a_late_delivery_acknowledgement() {
        let mut a = app();
        a.coordination_event(
            "conversation.turn_completed",
            &serde_json::json!({"turn_id":"c-one"}),
        );
        a.message_acknowledgement(
            "command",
            &serde_json::json!({"turn_id":"c-one","status":"accepted"}),
        );
        assert_eq!(a.notice, "Reply complete");
        a.coordination_event("conversation.input_result",&serde_json::json!({"input_id":"late-input","subject_id":"discussion","submitted_subject":"discussion","route":"submit_message","outcome":{"status":"accepted","turn_id":"c-one"}}));
        assert_eq!(a.notice, "Reply complete");
        assert!(a.delivery_turn.is_none());

        a.coordination_event("conversation.input_result",&serde_json::json!({"input_id":"input","submitted_subject":"discussion","subject_id":"discussion","route":"authorize_scope","outcome":{"status":"accepted"}}));
        a.message_acknowledgement(
            "input",
            &serde_json::json!({"input_id":"input","status":"accepted"}),
        );
        assert_eq!(a.notice, "Proposal approved");
    }
}

impl App {
    fn submit_inline_approval(&mut self, source: &str) {
        let context = self.coordination_context(source);
        self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"coordination.submit","payload":{"subject_id":self.conversation_subject,"text":"approve","interaction":context}})));
    }
    fn provider_grant_text(card: &Value) -> String {
        format!(
            "**Allow provider for {}?**\nExact destination: {}/{} · effort {} · role {} · profile {}.\n{}\nPreferences are saved; this grant is still pending. Reply **allow provider** to approve this exact destination/role, or use F3 for detail.",
            card["role"].as_str().unwrap_or("this role"),
            card["profile"]["provider"].as_str().unwrap_or("?"),
            card["profile"]["model"].as_str().unwrap_or("?"),
            card["profile"]["effort"].as_str().unwrap_or("?"),
            card["role"].as_str().unwrap_or("?"),
            card["profile_id"].as_str().unwrap_or("?"),
            card["explanation"]
                .as_str()
                .unwrap_or("Only the displayed role and project context.")
        )
    }
    fn restore_proposal_cards(&mut self) {
        let cards = self
            .live
            .as_ref()
            .and_then(|l| l.conversation.as_ref())
            .and_then(|c| c["coordination"]["requests"].as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|r| {
                        r.get("card")
                            .filter(|c| c["ready"] == true && c["event_seq"].is_number())
                            .cloned()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for card in cards {
            let seq = card["event_seq"].as_u64().unwrap();
            let id = card["card_id"].as_str().unwrap_or("").to_owned();
            self.card_seqs.insert(seq, id);
            if !self.journal.messages.iter().any(|m| m.seq == seq) {
                self.journal.messages.push(super::scenario::Message {
                    at: card["at"].as_f64(),
                    seq,
                    speaker: "Waywr".into(),
                    text: Self::proposal_card_text(&card),
                    task_id: card["identity"]["base"]["task_id"]
                        .as_str()
                        .unwrap_or("")
                        .into(),
                    task_version: card["identity"]["base"]["revision"].as_u64().unwrap_or(0) as u32,
                    simulated: false,
                    source_sha256: String::new(),
                    activity: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod proposal_presentation_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    #[test]
    fn coordination_card_presentation_and_explicit_inline_approval_share_exact_identity() {
        let mut a = tests::continuous_app();
        let head = a.current_scope().unwrap().clone();
        let card = serde_json::json!({"card_id":"exact-card","ready":true,"presented":false,"event_seq":500,"identity":{"base":head["base"],"request_id":"request","subject_id":"discussion"},"proposal":head["record"]["proposal"],"policy":{"provider":"codex","model":"gpt-6.1-sol","dependency_network":false,"main_policy":"checked-fast-forward","workers":{"concurrency":2},"fallback":[]}});
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"] = serde_json::json!({"version":1,"requests":[{"subject_id":"discussion","request_id":"request","scope":head,"status":"Awaiting your approval","card":card.clone()}]});
        assert!(App::proposal_card_text(&card).contains("Concurrent workers: 2"));
        let prose =
            "A failed check, rejected input, and blocked source must be handled before delivery.";
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]["requests"][0]["card"]
            ["proposal"]["constraints"] = serde_json::json!([prose]);
        a.restore_proposal_cards();
        a.open(Modal::Help);
        let mut t = Terminal::new(TestBackend::new(120, 50)).unwrap();
        t.draw(|f| a.render(f, Duration::ZERO)).unwrap();
        assert!(
            a.take_intent().is_none(),
            "A covered card was not presented"
        );
        a.input(key(KeyCode::Esc));
        t.draw(|f| a.render(f, Duration::ZERO)).unwrap();
        let Some(LiveIntent::Dialogue(shown)) = a.take_intent() else {
            panic!("Visible card needs presentation receipt")
        };
        assert_eq!(shown["action"], "coordination.present");
        assert_eq!(shown["payload"]["card_id"], "exact-card");
        let message = a.journal.messages.iter().find(|m| m.seq == 500).unwrap();
        let rendered = a.message_text(message, 100).to_string();
        assert!(rendered.contains("Awaiting your approval"));
        assert!(!rendered.contains("**"));
        assert!(!rendered.contains("• unavailable · Failed"));
        assert!(
            !rendered.contains(prose),
            "Detailed constraints belong in Details"
        );
        a.card_action = 1;
        a.focus = Focus::Composer;
        a.input(key(KeyCode::Enter));
        assert_eq!(a.modal, Modal::Scope);
        assert!(a.take_intent().is_none(), "Empty Enter cannot approve");
        a.input(key(KeyCode::Esc));
        a.focus = Focus::Activity;
        a.input(key(KeyCode::Enter));
        let Some(LiveIntent::Dialogue(approval)) = a.take_intent() else {
            panic!()
        };
        assert_eq!(approval["action"], "coordination.submit");
        assert_eq!(approval["payload"]["text"], "approve");
        assert_eq!(approval["payload"]["interaction"]["card_id"], "exact-card");
        assert_eq!(a.modal, Modal::None);
        assert!(a.take_intent().is_none(), "No second confirmation");
    }
    #[test]
    fn coordination_draft_targets_survive_refresh_recall_and_restart() {
        let mut a = tests::continuous_app();
        let head = a.current_scope().unwrap().clone();
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"] = serde_json::json!({"version":1,"requests":[{"subject_id":"discussion","scope":head,"card":{"card_id":"first","ready":true}}]});
        a.input_history.remember(Draft {
            text: "older input".into(),
            ..Draft::default()
        });
        a.input(Event::Paste("approve".into()));
        assert_eq!(
            a.message_payload("approve")["interaction"]["card_id"],
            "first"
        );
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]["requests"][0]["card"]
            ["card_id"] = serde_json::json!("second");
        assert_eq!(
            a.message_payload("approve")["interaction"]["card_id"],
            "first"
        );
        a.input(key(KeyCode::Up));
        assert_eq!(
            a.message_payload("older input")["interaction"]["card_id"],
            "second"
        );
        let saved = a.conversation_ui_state();
        assert_eq!(
            saved["draft_contexts"]["subject:discussion:chat"]["card_id"],
            "first"
        );
        a.input(key(KeyCode::Down));
        assert_eq!(a.composer.text(), "approve");
        assert_eq!(
            a.message_payload("approve")["interaction"]["card_id"],
            "first"
        );
        let mut restored = tests::continuous_app();
        restored.live.as_mut().unwrap().conversation =
            a.live.as_ref().unwrap().conversation.clone();
        restored.restore_conversation_ui(&saved).unwrap();
        assert_eq!(restored.composer.text(), "approve");
        assert_eq!(
            restored.message_payload("approve")["interaction"]["card_id"],
            "first"
        );
    }
    #[test]
    fn coordination_fresh_approval_after_command_and_restart_uses_the_current_card() {
        let mut a = tests::continuous_app();
        let head = a.current_scope().unwrap().clone();
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"] = serde_json::json!({"version":1,"requests":[{"subject_id":"discussion","scope":head,"card":{"card_id":"old-policy","ready":true}}]});
        for c in "/detach".chars() {
            a.input(key(KeyCode::Char(c)));
        }
        assert_eq!(a.draft_reply.as_ref().unwrap()["card_id"], "old-policy");
        a.input(key(KeyCode::Enter));
        assert!(a.detached);
        assert_eq!(a.modal, Modal::None);
        assert!(a.composer.text().is_empty());
        assert!(a.draft_reply.is_none());
        a.input(key(KeyCode::Esc));
        // Reproduce an empty draft/context written by the previous release.
        a.draft_reply = Some(a.coordination_context("typed"));
        let mut saved = a.conversation_ui_state();
        assert!(saved["draft_contexts"]["subject:discussion:chat"].is_null());
        saved["draft_contexts"]["subject:discussion:chat"] = a.draft_reply.clone().unwrap();
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]["requests"][0]["card"]
            ["card_id"] = serde_json::json!("current-policy");
        let mut b = tests::continuous_app();
        b.live.as_mut().unwrap().conversation = a.live.as_ref().unwrap().conversation.clone();
        b.restore_conversation_ui(&saved).unwrap();
        assert!(b.draft_reply.is_none());
        for c in "I approve".chars() {
            b.input(key(KeyCode::Char(c)));
        }
        b.input(key(KeyCode::Enter));
        assert!(matches!(b.take_intent(),Some(LiveIntent::SubmitTask(text)) if text=="I approve"));
        assert_eq!(
            b.modal,
            Modal::None,
            "A fresh chat approval must not open a mandatory menu"
        );
        assert_eq!(
            b.message_payload("I approve")["interaction"]["card_id"],
            "current-policy"
        );
    }
    #[test]
    fn coordination_repaired_legacy_draft_never_binds_to_a_future_proposal() {
        let mut a = coordination_ui_tests::app();
        a.composer = Composer::from_draft(Draft {
            text: "approve".into(),
            ..Draft::default()
        });
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]["resume_subject"] =
            serde_json::json!("discussion-new");
        a.restore_conversation_ui(
            &serde_json::json!({"schema_version":1,"subject_id":"discussion"}),
        )
        .unwrap();
        assert_eq!(a.conversation_subject, "discussion-new");
        assert!(a.composer.text().is_empty());
        a.discuss_selected_request();
        assert_eq!(a.composer.text(), "approve");
        a.live.as_mut().unwrap().conversation.as_mut().unwrap()["coordination"]["requests"][0]["card"] =
            serde_json::json!({"card_id":"future","ready":true});
        assert!(a.message_payload("approve")["interaction"]["card_id"].is_null());
    }
}

impl App {
    fn freeze_unbound_draft(&mut self) {
        if self.coordination_mode()
            && self.draft_reply.is_none()
            && !self.composer.text().is_empty()
        {
            let mut c = self.coordination_context("legacy unsent draft");
            for key in [
                "card_id",
                "candidate",
                "reply_to_question",
                "development_question",
                "provider_card_id",
            ] {
                c[key] = Value::Null;
            }
            self.draft_reply = Some(c);
        }
    }
    fn request_draft_slot(target: Target) -> &'static str {
        if matches!(target, Target::Coordinator | Target::NewTask) {
            "chat"
        } else {
            target.key()
        }
    }
}

impl App {
    fn confirm_inline_answer(&mut self) {
        if let Some(a) = self.question_suggestion().cloned() {
            let sha = waywr_backend::codec::canonical_sha256(&a["body"]).unwrap_or_default();
            self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"confirm_question_disposition","payload":{"action_id":a["action_id"],"suggestion_sha256":sha,"answer":a["body"]["action"]["payload"]["answer"],"coordination_version":1}})));
            self.notice = "Saving the displayed source-bound answer".into();
        }
    }
}

impl App {
    pub fn presentation_delivery_failed(&mut self) {
        self.card_presenting = None;
    }
}

impl App {
    pub fn retain_submission_context(&mut self, submitted: &Value) {
        if submitted["interaction"]["version"] == 1
            && submitted["interaction"]["subject_id"] == self.conversation_subject
        {
            self.draft_reply = Some(submitted["interaction"].clone());
        }
    }
    pub fn message_submitted(&mut self) {
        self.discard_reply_context();
    }
    fn discard_reply_context(&mut self) {
        self.draft_reply = None;
        self.history_reply = None;
        let key = format!(
            "subject:{}:{}",
            self.conversation_subject,
            Self::request_draft_slot(self.journal.target)
        );
        self.draft_replies.remove(&key);
    }
}
