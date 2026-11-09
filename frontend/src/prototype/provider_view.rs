// Production F3 forms. No discovery or inference occurs merely by opening Settings.
impl App {
    fn provider_state(&self) -> Value {
        self.development_view()["providers"].clone()
    }
    fn provider_draft(&self) -> Value {
        let mut state = self.provider_state();
        if state["setup_preferences"].is_object() {
            state["preferences"] = state["setup_preferences"].clone();
        }
        state
    }
    fn provider_settings(&self) -> bool {
        self.modal == Modal::Settings && self.provider_state()["schema_version"] == 1
    }
    fn provider_units(&self) -> Vec<Value> {
        let Some(work) = self.development_workflow() else {
            return vec![];
        };
        self.development_view()["units"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|u| u["work_id"] == work["work_id"])
            .cloned()
            .collect()
    }
    fn provider_names(&self) -> Vec<String> {
        self.provider_edit
            .as_ref()
            .and_then(|v| v["preferences"]["profiles"].as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }
    fn provider_command(&mut self, action: &str, mut payload: Value) {
        payload["subject_id"] = serde_json::json!(self.conversation_subject);
        payload["expected_revision"] = self
            .provider_edit
            .as_ref()
            .map(|v| v["revision"].clone())
            .unwrap_or(Value::Null);
        self.intent(LiveIntent::Dialogue(
            serde_json::json!({"action":action,"payload":payload}),
        ));
        self.notice="Provider action pending durable acceptance; inspect the updated settings before continuing".into();
    }
    pub fn provider_command_started(&mut self, action: &str, id: &str) {
        if action == "providers.save" {
            if let Some(edit) = &mut self.provider_edit {
                edit["pending_save_command"] = serde_json::json!(id);
            }
        }
    }
    pub fn provider_command_result(&mut self, id: &str, outcome: &Value) {
        if let Some(edit) = &mut self.provider_edit {
            if edit["pending_save_command"].as_str() == Some(id) {
                edit.as_object_mut().unwrap().remove("pending_save_command");
                if outcome["status"] == "accepted" {
                    edit["await_revision"] = outcome["preferences_revision"].clone();
                }
            }
        }
        self.provider_refresh_saved();
    }
    fn provider_refresh_saved(&mut self) {
        if let Some(revision) = self
            .provider_edit
            .as_ref()
            .and_then(|e| e["await_revision"].as_u64())
        {
            let saved = self.provider_state();
            if saved["revision"].as_u64().is_some_and(|r| r >= revision) {
                self.provider_edit = Some(saved);
                self.provider_typing = false;
            }
        }
    }
    fn provider_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.code==KeyCode::Char('7')&&!self.provider_typing{self.provider_section=6;self.provider_row=0;self.mark();return}
        if self.provider_section==6&&(self.provider_typing||!matches!(key.code,KeyCode::Char('1'..='6'))){self.project_key(key);return}
        if self.provider_edit.is_none() {
            self.provider_edit = Some(self.provider_draft());
        }
        self.provider_refresh_saved();
        if self.provider_edit.as_ref().is_some_and(|e| {
            e["pending_save_command"].is_string() || e["await_revision"].is_number()
        }) && key.code != KeyCode::Char('l')
        {
            self.notice =
                "Preference save pending; the saved revision will refresh automatically".into();
            self.mark();
            return;
        }
        let names = self.provider_names();
        if names.is_empty() {
            return;
        }
        let name = names[self.provider_index % names.len()].clone();
        if self.provider_typing {
            if key.code == KeyCode::Esc {
                self.provider_typing = false;
                self.mark();
                return;
            }
            if key.code == KeyCode::Enter {
                if self.provider_field==6 {
                    let query=self.scope_editor.text().to_lowercase();
                    let choice=self.provider_state()["discovery"]["models"].as_array().into_iter().flatten()
                        .find(|m|format!("{}/{} {}",m["provider"].as_str().unwrap_or(""),m["model"].as_str().unwrap_or(""),m["name"].as_str().unwrap_or("")).to_lowercase().contains(&query)).cloned();
                    if let Some(m)=choice {let p=&mut self.provider_edit.as_mut().unwrap()["preferences"]["profiles"][&name];p["provider"]=m["provider"].clone();p["model"]=m["model"].clone();p["options"]=serde_json::json!({});self.provider_typing=false;self.provider_field=1;}
                    else {self.notice="No matching model; edit provider/model fields to enter an exact custom ID".into();}
                    self.mark();return;
                }
                let field = ["provider", "model", "options", "binary", "transport", "context_bytes"][self.provider_field % 6];
                let text = self.scope_editor.text();
                if let Some(edit) = &mut self.provider_edit {
                    edit["preferences"]["profiles"][&name][field] = if matches!(field,"transport"|"options"){match serde_json::from_str(&text){Ok(v)=>v,Err(_)=>{self.notice="Enter valid JSON before accepting this field".into();return;}}}else if field == "context_bytes" {
                        text.parse::<u64>().map(Value::from).unwrap_or(Value::Null)
                    } else {
                        Value::from(text)
                    };
                    if field == "binary" {
                        edit["preferences"]["profiles"][&name]["binary_sha256"] = Value::Null;
                    }
                }
                self.provider_typing = false;
                self.mark();
                return;
            }
            self.scope_editor.input(Event::Key(key));
            self.mark();
            return;
        }
        if let KeyCode::Char(c @ '1'..='6') = key.code {
            self.provider_section = (c as u8 - b'1') as usize;
            self.provider_row = 0;
            self.provider_grant = false;
            self.mark();
            return;
        }
        match key.code {
            KeyCode::Char('l') => {
                self.provider_edit = Some(self.provider_draft());
                self.notice = "Reloaded saved preferences; unsaved edits discarded".into();
            }
            KeyCode::Char('r') => self.provider_command("providers.refresh", serde_json::json!({})),
            KeyCode::Char('s') => {
                let p = self.provider_edit.as_ref().unwrap()["preferences"].clone();
                match waywr_backend::provider_selection::Preferences::parse(&p) {
                    Ok(_) => self
                        .provider_command("providers.save", serde_json::json!({"preferences":p})),
                    Err(e) => self.notice = e,
                }
            }
            KeyCode::Left | KeyCode::Right => {
                self.provider_index = if key.code == KeyCode::Right {
                    (self.provider_index + 1) % names.len()
                } else {
                    (self.provider_index + names.len() - 1) % names.len()
                };
                self.provider_grant = false;
            }
            KeyCode::Up | KeyCode::Down => {
                if self.provider_section == 0 || self.provider_section == 3 {
                    self.provider_index = if key.code == KeyCode::Down {
                        (self.provider_index + 1) % names.len()
                    } else {
                        (self.provider_index + names.len() - 1) % names.len()
                    };
                } else {
                    let count = if self.provider_section == 4 {
                        self.provider_units().len().max(1)
                    } else {
                        3
                    };
                    self.provider_row = if key.code == KeyCode::Down {
                        (self.provider_row + 1) % count
                    } else {
                        (self.provider_row + count - 1) % count
                    };
                }
                self.provider_grant = false;
            }
            KeyCode::Tab => {
                self.provider_field = (self.provider_field + 1) % 6;
            }
            KeyCode::Char('+') if self.provider_section == 0 => {
                let edit = self.provider_edit.as_mut().unwrap();
                let mut copy = edit["preferences"]["profiles"][&name].clone();
                copy["effort"] = serde_json::json!("provider_default");
                let id = (1..100)
                    .map(|n| format!("profile_{n}"))
                    .find(|id| edit["preferences"]["profiles"].get(id).is_none())
                    .unwrap();
                edit["preferences"]["profiles"][&id] = copy;
                self.provider_index = self
                    .provider_names()
                    .iter()
                    .position(|n| n == &id)
                    .unwrap_or(0);
            }
            KeyCode::Char('c') if self.provider_section==0=>{
                let current=&self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&name];let provider=current["provider"].clone();let catalog=self.provider_state()["discovery"]["models"].as_array().cloned().unwrap_or_default().into_iter().filter(|m|m["provider"]==provider).collect::<Vec<_>>();
                if catalog.is_empty(){self.notice="R refreshes runtime metadata; F searches all providers/models".into();return}let i=catalog.iter().position(|m|m["model"]==current["model"]).map_or(0,|i|(i+1)%catalog.len());let p=&mut self.provider_edit.as_mut().unwrap()["preferences"]["profiles"][&name];p["model"]=catalog[i]["model"].clone();p["effort"]=serde_json::json!("provider_default");self.notice="Model selected in draft; edit options JSON if needed, then S Save. Existing attempts stay frozen.".into();
            }
            KeyCode::Char('f') if self.provider_section==0=>{self.provider_field=6;self.provider_typing=true;self.scope_editor=Composer::default();self.notice="Search provider/model; Enter selects first match".into();}
            KeyCode::Char('v') if self.provider_section==0=>{
                let models=self.provider_state()["discovery"]["models"].as_array().cloned().unwrap_or_default();
                let mut providers=models.iter().filter_map(|m|m["provider"].as_str()).collect::<Vec<_>>();providers.sort();providers.dedup();
                let current=self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&name]["provider"].as_str().unwrap_or("").to_owned();
                if !providers.is_empty(){let i=providers.iter().position(|p|*p==current).map_or(0,|i|(i+1)%providers.len());
                    if let Some(m)=models.iter().find(|m|m["provider"]==providers[i]){let p=&mut self.provider_edit.as_mut().unwrap()["preferences"]["profiles"][&name];p["provider"]=m["provider"].clone();p["model"]=m["model"].clone();p["options"]=serde_json::json!({});}}
            }
            KeyCode::Char('i') if self.provider_section == 1 && self.provider_row > 0 => {
                let role = waywr_backend::provider_selection::ROLES[self.provider_row % 3];
                self.provider_edit.as_mut().unwrap()["preferences"]["roles"]
                    .as_object_mut()
                    .unwrap()
                    .remove(role);
            }
            KeyCode::Char('w') if self.provider_section == 1 => {
                self.provider_edit.as_mut().unwrap()["preferences"]["worker_default"] =
                    serde_json::json!(name);
            }
            KeyCode::Char(' ') if self.provider_section == 1 || self.provider_section == 2 => {
                let role = waywr_backend::provider_selection::ROLES[self.provider_row % 3];
                let edit = self.provider_edit.as_mut().unwrap();
                if self.provider_section == 1 && !edit["preferences"]["roles"][role].is_object() {
                    edit["preferences"]["roles"][role] = serde_json::json!({"default":edit["preferences"]["worker_default"],"allowed":[edit["preferences"]["worker_default"]]});
                }
                let values = if self.provider_section == 1 {
                    &mut edit["preferences"]["roles"][role]["allowed"]
                } else {
                    &mut edit["preferences"]["fallback"][role]
                };
                let mut list = values.as_array().cloned().unwrap_or_default();
                if let Some(i) = list.iter().position(|v| *v == name) {
                    list.remove(i);
                } else {
                    list.push(serde_json::json!(name));
                }
                *values = serde_json::json!(list);
            }
            KeyCode::Char('i') if self.provider_section == 3 => {
                self.provider_interrupt = !self.provider_interrupt;
            }
            KeyCode::Char('g') if self.provider_section == 3 => {
                self.provider_grant = !self.provider_grant;
            }
            KeyCode::Char('c') if self.provider_section == 3 => {
                self.provider_command("providers.cancel_switch", serde_json::json!({}))
            }
            KeyCode::Char(c @ ('i' | 'p' | 'a')) if self.provider_section == 4 => {
                let units = self.provider_units();
                if let Some(unit) = units.get(self.provider_row) {
                    let mut override_ = unit["profile_override"].clone();
                    if c == 'i' {
                        override_ = serde_json::json!({"mode":"inherit"});
                    }
                    if c == 'p' {
                        override_ = serde_json::json!({"mode":"pin","profiles":[name]});
                    }
                    if c == 'a' {
                        let mut profiles = if override_["mode"] == "allow" {
                            override_["profiles"]
                                .as_array()
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            vec![]
                        };
                        if let Some(i) = profiles.iter().position(|v| *v == name) {
                            profiles.remove(i);
                        } else {
                            profiles.push(serde_json::json!(name));
                        }
                        override_ = serde_json::json!({"mode":"allow","profiles":profiles});
                    }
                    self.provider_command("providers.task_override",serde_json::json!({"unit_id":unit["unit_id"],"attempt_revision":unit["attempt_revision"],"override":override_}));
                }
            }
            KeyCode::Enter => match self.provider_section {
                0 => {
                    let field =
                        ["provider", "model", "options", "binary", "transport", "context_bytes"][self.provider_field % 6];
                    let value = &self.provider_edit.as_ref().unwrap()["preferences"]["profiles"]
                        [&name][field];
                    self.scope_editor = Composer::default();
                    self.scope_editor.input(Event::Paste(
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    ));
                    self.provider_typing = true;
                }
                1 => {
                    let role = waywr_backend::provider_selection::ROLES[self.provider_row % 3];
                    let edit = self.provider_edit.as_mut().unwrap();
                    if !edit["preferences"]["roles"][role].is_object() {
                        edit["preferences"]["roles"][role] =
                            serde_json::json!({"default":name,"allowed":[name]});
                    } else {
                        edit["preferences"]["roles"][role]["default"] = serde_json::json!(name);
                    }
                    if role == "coordinator" {
                        edit["preferences"]["coordinator"] = serde_json::json!(name);
                    }
                }
                3 => {
                    let binding = self
                        .live
                        .as_ref()
                        .and_then(|l| l.conversation.as_ref())
                        .map(|c| c["binding"]["generation"].clone())
                        .unwrap_or(Value::Null);
                    self.provider_command("providers.switch",serde_json::json!({"profile_id":name,"expected_generation":binding,"mode":if self.provider_interrupt{"interrupt"}else{"after_current_turn"},"grant_context":self.provider_grant}));
                    self.provider_grant = false;
                }
                5 => {
                    if let Some(work) = self.development_workflow() {
                        match waywr_backend::provider_selection::Preferences::parse(&self.provider_state()["preferences"]).and_then(|p|p.allowed_policy()){
                            Ok(routing)=>self.provider_command("providers.approve_policy",serde_json::json!({"work_id":work["work_id"],"policy_sha256":waywr_backend::provider_selection::digest(&work["policy"]).unwrap_or_default(),"routing_sha256":waywr_backend::provider_selection::digest(&routing).unwrap_or_default()})),Err(e)=>self.notice=e
                        }
                    } else {
                        self.notice =
                            "Approve a new request's scope and operating policy through /plan"
                                .into();
                    }
                }
                _ => {}
            },
            _ => {}
        }
        self.mark();
    }
    fn provider_body(&self) -> (String, String) {
        if self.provider_section==6{return self.project_body()}
        let saved = self.provider_state();
        let edit = self.provider_edit.as_ref().unwrap_or(&saved);
        let p = &edit["preferences"];
        let names = self.provider_names();
        let name = names
            .get(self.provider_index % names.len().max(1))
            .map(String::as_str)
            .unwrap_or("default");
        let mut text = format!(
            "[1 Profiles] [2 Roles] [3 Fallback] [4 Switch] [5 Tasks] [6 Policy] [7 Project]\nRevision {} · saved current coordinator {}\nSelected profile: {} · ←/→ changes profile\nS saves preference draft · L reloads · R refreshes local metadata · Esc returns\nPreference saves affect future attempts; Switch changes both coordinator lanes.\n\n",
            edit["revision"], saved["coordinator"]["profile_id"], name
        );
        if saved["setup_required"] == true {
            text.push_str("Provider setup required. Retired profiles cannot execute.\nChoose provider/model, S save, then 4 Switch. G explicitly grants selected project context; Enter applies the switch. Existing request policies may need approval in 6 Policy.\nThis form is an unsaved draft; opening it changes no provider settings.\n\n");
        }
        match self.provider_section {
            0 => {
                for id in p["profiles"].as_object().into_iter().flat_map(|o| o.keys()) {
                    let v = &p["profiles"][id];
                    text.push_str(&format!(
                        "{} {}: {}/{} · {}\n",
                        if id == name { "›" } else { " " },
                        id,
                        v["provider"].as_str().unwrap_or("?"),
                        v["model"].as_str().unwrap_or("?"),
                        v["effort"].as_str().unwrap_or("?")
                    ));
                }
                text.push_str("\n↑/↓ profile · + duplicate · V provider · C model · F search catalog · Tab field · Enter edits/accepts\n");
                for (i, field) in ["provider", "model", "options", "binary", "transport", "context_bytes"]
                    .iter()
                    .enumerate()
                {
                    text.push_str(&format!(
                        "{} {}: {}\n",
                        if i == self.provider_field { "›" } else { " " },
                        if *field=="context_bytes"{"working target bytes (optional; soft)"}else{field},
                        if self.provider_typing && i == self.provider_field {
                            self.scope_editor.text()
                        } else {
                            p["profiles"][name][*field]
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| p["profiles"][name][*field].to_string())
                        }
                    ));
                }
                let query=if self.provider_typing&&self.provider_field==6{self.scope_editor.text()}else{String::new()};
                if self.provider_field==6{text.push_str(&format!("\nSearch: {query} (Enter selects first match)\n"));}
                let state=self.provider_state();let models=state["discovery"]["models"].as_array().cloned().unwrap_or_default();
                let mut providers=models.iter().filter_map(|m|m["provider"].as_str()).collect::<Vec<_>>();providers.sort();providers.dedup();
                text.push_str(&format!("\n{} providers · {} catalog models · authentication not checked\n",providers.len(),models.len()));
                if let Some(reason)=state["discovery"]["reason"].as_str(){text.push_str(&format!("Catalog unavailable: {reason}\n"));}
                let selected=p["profiles"][name]["provider"].as_str().unwrap_or("");
                for m in models.iter().filter(|m|if query.is_empty(){m["provider"]==selected}else{format!("{}/{} {}",m["provider"].as_str().unwrap_or(""),m["model"].as_str().unwrap_or(""),m["name"].as_str().unwrap_or("")).to_lowercase().contains(&query.to_lowercase())}).take(16){
                    text.push_str(&format!("  {}/{} · context {}\n",m["provider"].as_str().unwrap_or(""),m["model"].as_str().unwrap_or(""),m["context_window"]));
                }
                text.push_str("Exact provider/model IDs and runtime JSON options are editable. Catalogs are suggestions, not core allowlists. Configure endpoints/models in the runtime configuration; credentials stay outside project state.\n");

            }
            1 | 2 => {
                text.push_str(if self.provider_section==1{"↑/↓ role · Space toggle selected profile in allowed set · Enter set role default · I inherit project worker default · W set project worker default\n"}else{"↑/↓ role · Space append/remove selected alternative. Listed order is fallback order. Empty = wait, never substitute.\n"});
                for (i, role) in waywr_backend::provider_selection::ROLES.iter().enumerate() {
                    text.push_str(&format!(
                        "{} {}: {}\n",
                        if self.provider_row == i { "›" } else { " " },
                        role,
                        if self.provider_section == 1 {
                            p["roles"][*role].clone()
                        } else {
                            p["fallback"][*role].clone()
                        }
                    ));
                }
                text.push_str(&format!("\nProject worker default: {}\nSave preferences, then review /plan policy or a narrow amendment before a new destination executes. Fallback requires explicit settled capacity refusal; auth/config/policy/uncertain failures never substitute.",p["worker_default"]));
            }
            3 => {
                text.push_str(&format!("Target profile (saved value):\n{}\n\nMode: {} · I toggles explicit interruption\nG toggles selected-context grant for this exact NEW profile: {}\nEnter requests switch · C cancels a waiting after-turn switch\n\nPending handover:\n{}\nWorkers, questions, decisions and integration remain in Waywr. Interrupt fences old decisions and waits for positive settlement; uncertain work blocks replacement.\n",serde_json::to_string_pretty(&saved["preferences"]["profiles"][name]).unwrap_or_default(),if self.provider_interrupt{"INTERRUPT AND SWITCH"}else{"after current turn (default)"},self.provider_grant,serde_json::to_string_pretty(&saved["switch"]).unwrap_or_default()));
            }
            4 => {
                text.push_str("↑/↓ task · ←/→ profile · P pin · A toggle allowed set · I inherit\nOnly unreserved attempts change. Running attempt selections stay frozen.\n");
                let units = self.provider_units();
                {
                    for (i, u) in units.iter().enumerate() {
                        text.push_str(&format!(
                            "{} {} · {} · role {}\n  Override {}\n  Frozen selection {}\n",
                            if i == self.provider_row { "›" } else { " " },
                            u["title"].as_str().unwrap_or("Task"),
                            u["status"],
                            u["role"],
                            u["profile_override"],
                            u["selection"]
                        ));
                    }
                }
            }
            5 => {
                text.push_str("Enter explicitly approves this additive provider policy for the active request. Existing scope approval, attempts and work are retained. This grants the named exact profiles access/execution for the listed roles; no new project or expanded task scope.\n\n");
                if let Some(work) = self.development_workflow() {
                    text.push_str(&format!(
                        "Request: {}\nCurrent policy:\n{}\n",
                        work["work_id"],
                        serde_json::to_string_pretty(&work["policy"]).unwrap_or_default()
                    ));
                }
                text.push_str(&format!(
                    "Saved proposed routing (unsaved form edits are excluded):\n{}\nFallback: {}",
                    waywr_backend::provider_selection::Preferences::parse(&saved["preferences"])
                        .and_then(|p| p.allowed_policy())
                        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
                        .unwrap_or_default(),
                    saved["preferences"]["fallback"]
                ));
            }
            _ => {}
        }
        ("Project providers and roles".into(), text)
    }
}

#[cfg(test)]
mod provider_ui_tests {
    use super::*;
    #[test]
    fn retired_profiles_offer_an_unsaved_setup_form() {
        let mut app = App::new_live("project".into(), Draft::default());
        app.live.as_mut().unwrap().conversation = Some(serde_json::json!({"development":{"providers":{
            "schema_version":1,"revision":7,"setup_required":true,
            "preferences":{"schema_version":2,"profiles":{"retired":{"provider":"retired"}}},
            "setup_preferences":{"schema_version":3,"profile":"development-profiles-v1",
                "profiles":{"default":{"provider":"new-provider","model":"new-model","binary":"/missing/node",
                    "transport":{"entry":"bundled"},"effort":"provider_default","options":{},"context_bytes":98304}},
                "coordinator":"default","worker_default":"default","roles":{},"fallback":{},"concurrency":2,
                "dependency_network":true,"main_policy":"checked-fast-forward","quality_preference":""},
            "coordinator":{"profile_id":"retired"}}}}));
        app.open(Modal::Settings);
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["schema_version"], 3);
        assert_eq!(app.provider_state()["preferences"]["schema_version"], 2);
        assert!(app.provider_body().1.contains("Provider setup required"));
        assert!(app.take_intent().is_none(), "Opening settings does not save or authorize execution");
        app.handle_modal(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(matches!(app.take_intent(),Some(LiveIntent::Dialogue(v)) if v["action"]=="providers.save" && v["payload"]["preferences"]["schema_version"]==3 && v["payload"]["expected_revision"]==7));
    }
    #[test]
    fn provider_forms_edit_save_roles_and_switch_without_sending_a_request() {
        let mut app = App::new_live("project".into(), Draft::default());
        let prefs = serde_json::json!({"schema_version":3,"profile":"development-profiles-v1","profiles":{"codex":{"provider":"codex","binary":"/missing/node","options":{},"transport":{"entry":"/unused/runtime.mjs"},"model":"gpt-6.1-sol","effort":"medium","binary_sha256":null,"context_bytes":98304},"claude":{"provider":"claude","binary":"/missing/node","options":{},"transport":{"entry":"/unused/runtime.mjs"},"model":"claude-sonnet-5-5","effort":"medium","binary_sha256":null,"context_bytes":98304}},"coordinator":"codex","worker_default":"codex","roles":{"coordinator":{"default":"codex","allowed":["codex"]},"review":{"default":"codex","allowed":["codex"]},"implementation":{"default":"codex","allowed":["codex"]}},"fallback":{"coordinator":[],"implementation":[],"review":[]},"concurrency":2,"dependency_network":true,"main_policy":"checked-fast-forward","quality_preference":""});
        app.live.as_mut().unwrap().conversation = Some(
            serde_json::json!({"binding":{"generation":1},"development":{"providers":{"schema_version":1,"revision":1,"preferences":prefs,"coordinator":{"profile_id":"codex"},"switch":null},"units":[],"workflows":[],"requests":[]}}),
        );
        app.open(Modal::Settings);
        assert!(app.provider_settings());
        assert!(app.take_intent().is_none());
        app.live.as_mut().unwrap().conversation.as_mut().unwrap()["development"]["providers"]["discovery"]=serde_json::json!({"models":[
            {"provider":"brand-outside-core","model":"custom-model","name":"Searchable model","context_window":32768}]});
        app.handle_modal(KeyEvent::new(KeyCode::Char('f'),KeyModifiers::NONE));
        app.scope_editor.input(Event::Paste("Searchable".into()));
        app.handle_modal(KeyEvent::new(KeyCode::Enter,KeyModifiers::NONE));
        let selected=app.provider_names()[app.provider_index].clone();
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&selected]["provider"],"brand-outside-core");
        assert!(app.provider_body().1.contains("custom-model"));
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.handle_modal(key(KeyCode::Down));
        assert_eq!(app.provider_index, 1); // arrow reaches form, not generic scroll
        app.handle_modal(key(KeyCode::Left));
        app.handle_modal(key(KeyCode::Char('2')));
        app.handle_modal(key(KeyCode::Char(' ')));
        app.handle_modal(key(KeyCode::Char('s')));
        assert!(
            matches!(app.take_intent(),Some(LiveIntent::Dialogue(v))if v["action"]=="providers.save")
        );
        // The saved server revision refreshes the form before a subsequent switch.
        app.provider_command_started("providers.save", "save-form");
        app.provider_command_result(
            "save-form",
            &serde_json::json!({"status":"accepted","preferences_revision":2}),
        );
        let mut saved = app.provider_edit.as_ref().unwrap().clone();
        saved["revision"] = serde_json::json!(2);
        saved.as_object_mut().unwrap().remove("await_revision");
        let conversation = app.live.as_ref().unwrap().conversation.clone().unwrap();
        let mut snapshot = serde_json::json!({"tasks":[],"session":{"phase":"running"},"conversation":conversation,"recent_events":[]});
        snapshot["conversation"]["development"]["providers"] = saved;
        app.apply_snapshot(&snapshot);
        assert_eq!(app.provider_edit.as_ref().unwrap()["revision"], 2);
        assert!(app.provider_edit.as_ref().unwrap()["await_revision"].is_null());
        app.handle_modal(key(KeyCode::Char('4')));
        assert!(!app.provider_interrupt);
        app.handle_modal(key(KeyCode::Enter));
        assert!(
            matches!(app.take_intent(),Some(LiveIntent::Dialogue(v))if v["action"]=="providers.switch"&&v["payload"]["expected_revision"]==2&&v["payload"]["mode"]=="after_current_turn"&&v["payload"]["grant_context"]==false)
        );
        app.handle_modal(key(KeyCode::Char('i')));
        app.handle_modal(key(KeyCode::Char('g')));
        app.handle_modal(key(KeyCode::Enter));
        assert!(
            matches!(app.take_intent(),Some(LiveIntent::Dialogue(v))if v["payload"]["mode"]=="interrupt"&&v["payload"]["grant_context"]==true)
        );
        assert!(app.take_intent().is_none());
        assert!(app.provider_body().1.contains("positive settlement"));
    }
}
