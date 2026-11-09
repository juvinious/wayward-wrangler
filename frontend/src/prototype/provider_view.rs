// Production F3 forms. No discovery or inference occurs merely by opening Settings.
const PROVIDER_FIELDS:[&str;7]=["provider","model","thinking","options","binary","transport","context_bytes"];
impl App {
    fn model_metadata(&self,profile:&Value)->Value {
        self.live.as_ref().and_then(|live|live.conversation.as_ref()).and_then(|c|c["development"]["providers"]["discovery"]["models"].as_array()).into_iter().flatten()
            .find(|m|m["provider"]==profile["provider"]&&m["model"]==profile["model"]).cloned().unwrap_or(Value::Null)
    }
    fn thinking_label(&self,profile:&Value)->String {
        if let Some(level)=profile["options"]["reasoning"].as_str(){return if self.model_metadata(profile)["thinking_levels"].is_array(){level.into()}else{format!("{level} (unverified)")}}
        if let Some(level)=self.model_metadata(profile)["thinking_default"].as_str(){return format!("default ({level})")}
        if let Some(effort)=profile["effort"].as_str().filter(|v|*v!="provider_default"){return format!("legacy {effort}")}
        "default (unreported)".into()
    }
    fn active_model_label(&self)->Option<String> {
        let profile=self.chat().map(|c|c["profile"].clone()).unwrap_or_else(||self.provider_state()["coordinator"]["profile"].clone());
        profile["model"].as_str().map(|model|format!("Model: {model} · thinking {} · {}",self.thinking_label(&profile),profile["provider"].as_str().unwrap_or("unconfigured")))
    }
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
        if key.kind == KeyEventKind::Release {
            return;
        }
        if self.provider_picker.is_some(){self.catalog_key(key);return;}
        if key.kind==KeyEventKind::Repeat&&(key.code==KeyCode::Enter||!self.provider_typing&&!matches!(key.code,KeyCode::Up|KeyCode::Down|KeyCode::Left|KeyCode::Right|KeyCode::Tab|KeyCode::BackTab|KeyCode::PageUp|KeyCode::PageDown|KeyCode::Home|KeyCode::End)){return;}
        if !self.provider_typing {
            if key.code==KeyCode::Esc {self.modal=Modal::None;self.mark();return;}
            if matches!(key.code,KeyCode::PageUp|KeyCode::PageDown|KeyCode::Home|KeyCode::End) {
                let step=if key.modifiers.contains(KeyModifiers::ALT){1}else{8};
                self.modal_scroll=match key.code{KeyCode::PageUp=>self.modal_scroll.saturating_sub(step),KeyCode::PageDown=>self.modal_scroll.saturating_add(step).min(self.modal_scroll_limit),KeyCode::Home=>0,_=>self.modal_scroll_limit};
                self.provider_follow=false;self.mark();return;
            }
            if let KeyCode::Char(c @ '1'..='7')=key.code {
                self.provider_section=(c as u8-b'1') as usize;self.provider_row=0;self.provider_grant=false;
                self.modal_scroll=0;self.provider_follow=true;self.mark();return;
            }
        }
        if self.provider_section==6{self.provider_follow=true;self.project_key(key);return}
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
            if matches!(key.code,KeyCode::Enter|KeyCode::Tab|KeyCode::BackTab) {
                let field = PROVIDER_FIELDS[self.provider_field % PROVIDER_FIELDS.len()];
                let text = self.scope_editor.text();
                if field=="thinking" {
                    let profile=&self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&name];
                    let levels=self.model_metadata(profile)["thinking_levels"].clone();
                    if levels.as_array().is_some_and(|levels|!levels.contains(&Value::String(text.clone()))){self.notice="Choose a runtime-advertised thinking level for this model".into();self.mark();return;}
                }
                if let Some(edit) = &mut self.provider_edit {
                    let value = if matches!(field,"transport"|"options"){match serde_json::from_str(&text){Ok(v)=>v,Err(_)=>{self.notice="Enter valid JSON before accepting this field".into();return;}}}else if field == "context_bytes" {
                        text.parse::<u64>().map(Value::from).unwrap_or(Value::Null)
                    } else {
                        Value::from(text)
                    };
                    if field=="options"&&!value.is_object(){self.notice="Model options must be a JSON object".into();self.mark();return;}
                    if field=="thinking"{edit["preferences"]["profiles"][&name]["options"]["reasoning"]=value;}else{edit["preferences"]["profiles"][&name][field]=value;}
                    if field == "binary" {
                        edit["preferences"]["profiles"][&name]["binary_sha256"] = Value::Null;
                    }
                }
                self.provider_typing = false;
                if key.code!=KeyCode::Enter{self.provider_field=(self.provider_field+if key.code==KeyCode::Tab{1}else{PROVIDER_FIELDS.len()-1})%PROVIDER_FIELDS.len();}
                self.provider_follow=true;
                self.mark();
                return;
            }
            self.scope_editor.input(Event::Key(key));
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
                for profile in p["profiles"].as_object().into_iter().flat_map(|profiles|profiles.values()){
                    let requested=&profile["options"]["reasoning"];
                    if !requested.is_null()&&(!requested.is_string()||self.model_metadata(profile)["thinking_levels"].as_array().is_some_and(|levels|!levels.contains(requested))){self.notice="Unsupported thinking level: choose one advertised for the selected model".into();self.mark();return;}
                }
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
                if self.provider_section == 0 {
                    self.provider_field=if key.code==KeyCode::Down{(self.provider_field+1).min(PROVIDER_FIELDS.len()-1)}else{self.provider_field.saturating_sub(1)};
                }else if self.provider_section == 3 {
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
            KeyCode::Tab | KeyCode::BackTab => {
                if self.provider_section==0{self.provider_field=(self.provider_field+if key.code==KeyCode::Tab{1}else{PROVIDER_FIELDS.len()-1})%PROVIDER_FIELDS.len();}
                else{let code=if key.code==KeyCode::Tab{KeyCode::Down}else{KeyCode::Up};self.provider_key(KeyEvent::new(code,KeyModifiers::NONE));return;}
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
            KeyCode::Char('c') if self.provider_section==0=>{let provider=self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&name]["provider"].as_str().map(str::to_owned);self.open_catalog(CatalogScope::Models(provider));}
            KeyCode::Char('f'|'/') if self.provider_section==0=>self.open_catalog(CatalogScope::Models(None)),
            KeyCode::Char('v') if self.provider_section==0=>self.open_catalog(CatalogScope::Providers),
            KeyCode::Char('e') if self.provider_section==0=>self.provider_edit_field(&name),
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
                    match self.provider_field {
                        0=>self.open_catalog(CatalogScope::Providers),
                        1=>{let provider=self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&name]["provider"].as_str().map(str::to_owned);self.open_catalog(CatalogScope::Models(provider));},
                        2=>self.open_catalog(CatalogScope::Thinking),
                        _=>self.provider_edit_field(&name),
                    }
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
        self.provider_follow=true;self.mark();
    }
    fn provider_edit_field(&mut self,name:&str) {
        let field=PROVIDER_FIELDS[self.provider_field%PROVIDER_FIELDS.len()];
        let profile=&self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][name];
        let value=if field=="thinking"{profile["options"]["reasoning"].as_str().map(|s|Value::String(s.into())).unwrap_or_else(||self.model_metadata(profile)["thinking_default"].clone())}else{profile[field].clone()};
        self.scope_editor=Composer::default();
        self.scope_editor.input(Event::Paste(value.as_str().map(str::to_owned).unwrap_or_else(||value.to_string())));
        self.provider_typing=true;
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
        let mut text = format!("Revision {} · current coordinator {}\n",edit["revision"],saved["coordinator"]["profile_id"]);
        if saved["setup_required"] == true {
            text.push_str("Provider setup required. Retired profiles cannot execute.\nChoose provider/model, S save, then 4 Switch. G explicitly grants selected project context; Enter applies the switch. Existing request policies may need approval in 6 Policy.\nThis form is an unsaved draft; opening it changes no provider settings.\n\n");
        }
        match self.provider_section {
            0 => {
                for id in p["profiles"].as_object().into_iter().flat_map(|o| o.keys()) {
                    let v = &p["profiles"][id];
                    text.push_str(&format!(
                        "{} {}: {}/{} · {}\n",
                        if id == name { "•" } else { " " },
                        id,
                        v["provider"].as_str().unwrap_or("?"),
                        v["model"].as_str().unwrap_or("?"),
                        self.thinking_label(v)
                    ));
                }
                text.push_str("\n←/→ profile · + duplicate · V providers · C provider models · F all models\n");
                for (i, field) in PROVIDER_FIELDS
                    .iter()
                    .enumerate()
                {
                    text.push_str(&format!(
                        "{} {}: {}\n",
                        if i == self.provider_field { "›" } else { " " },
                        if *field=="context_bytes"{"working target bytes (optional; soft)"}else{field},
                        if self.provider_typing && i == self.provider_field {
                            self.scope_editor.text()
                        } else if *field=="thinking"{self.thinking_label(&p["profiles"][name])}else {
                            p["profiles"][name][*field]
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| p["profiles"][name][*field].to_string())
                        }
                    ));
                }
                let state=self.provider_state();let models=state["discovery"]["models"].as_array().cloned().unwrap_or_default();
                let mut providers=models.iter().filter_map(|m|m["provider"].as_str()).collect::<Vec<_>>();providers.sort();providers.dedup();
                text.push_str(&format!("\n{} providers · {} catalog models · authentication not checked\n",state["discovery"]["providers"].as_array().map_or(providers.len(),Vec::len),models.len()));
                if let Some(reason)=state["discovery"]["reason"].as_str(){text.push_str(&format!("Catalog unavailable: {reason}\n"));}
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
    fn capture(app:&mut App,width:u16,height:u16,label:&str)->String {
        let mut terminal=ratatui::Terminal::new(ratatui::backend::TestBackend::new(width,height)).unwrap();
        terminal.draw(|frame|app.render(frame,Duration::ZERO)).unwrap();
        let screen=terminal.backend().buffer().content().chunks(width as usize)
            .map(|row|row.iter().map(|c|c.symbol()).collect::<String>()).collect::<Vec<_>>().join("\n");
        if let Some(root)=std::env::var_os("WAYWR_UI_SNAPSHOT_DIR") {
            let root=std::path::PathBuf::from(root);std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join(format!("{label}-{width}x{height}.txt")),&screen).unwrap();
        }
        screen
    }
    fn navigation_app() -> App {
        let mut app=App::new_live("project".into(),Draft::default());
        let prefs=serde_json::json!({"schema_version":3,"profile":"development-profiles-v1",
            "profiles":{"default":{"provider":"provider-a","model":"model-00","binary":"/missing/node",
                "transport":{"entry":"bundled"},"effort":"provider_default","options":{},"context_bytes":98304}},
            "coordinator":"default","worker_default":"default","roles":{},"fallback":{},"concurrency":2,
            "dependency_network":true,"main_policy":"checked-fast-forward","quality_preference":""});
        let models=(0..60).map(|i|serde_json::json!({"provider":if i<30{"provider-a"}else{"provider-b"},
            "model":format!("model-{i:02}"),"name":format!("Model {i:02}"),"context_window":32768})).collect::<Vec<_>>();
        app.live.as_mut().unwrap().conversation=Some(serde_json::json!({"development":{"providers":{
            "schema_version":1,"revision":1,"preferences":prefs,"coordinator":{"profile_id":"default"},
            "discovery":{"providers":[{"id":"provider-a"},{"id":"provider-b"},{"id":"provider-empty"}],"models":models}}}}));
        app.input(Event::Key(KeyEvent::new(KeyCode::F(3),KeyModifiers::NONE)));
        app
    }
    #[test]
    fn thinking_selection_uses_runtime_choices_and_shows_draft_and_active_levels() {
        let mut app=navigation_app();
        let live=app.live.as_mut().unwrap().conversation.as_mut().unwrap();
        live["development"]["providers"]["discovery"]["models"][0]["thinking_levels"]=serde_json::json!(["off","high","runtime-defined-level"]);
        live["development"]["providers"]["discovery"]["models"][0]["thinking_default"]=serde_json::json!("off");
        live["chat"]=serde_json::json!({"status":"available","profile":{"provider":"provider-a","model":"model-00","options":{"reasoning":"high"}}});
        let key=|code|Event::Key(KeyEvent::new(code,KeyModifiers::NONE));
        app.input(key(KeyCode::Down));app.input(key(KeyCode::Down));app.input(key(KeyCode::Enter));app.input(key(KeyCode::End));
        let screen=capture(&mut app,80,24,"thinking-picker");
        assert!(screen.contains("runtime-defined-level"),"The chooser must use runtime metadata, not a Rust effort enum");
        app.input(key(KeyCode::Enter));
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["options"]["reasoning"],"runtime-defined-level");
        let screen=capture(&mut app,80,24,"thinking-draft");
        assert!(screen.contains("model-00")&&screen.contains("runtime-defined-level"));
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Char('s')));
        assert!(matches!(app.take_intent(),Some(LiveIntent::Dialogue(v)) if v["action"]=="providers.save"&&v["payload"]["preferences"]["profiles"]["default"]["options"]["reasoning"]=="runtime-defined-level"));
        app.input(key(KeyCode::Esc));
        let screen=capture(&mut app,80,24,"active-thinking");
        assert!(screen.contains("model-00")&&screen.contains("thinking high"),"The main view must show the active model and thinking level");
    }
    #[test]
    fn settings_arrows_and_reverse_tab_move_fields_and_escape_cancels_only_the_edit() {
        let mut app=navigation_app();
        let key=|code|Event::Key(KeyEvent::new(code,KeyModifiers::NONE));
        app.input(key(KeyCode::Down));
        assert!(app.provider_body().1.contains("› model:"),"Down must move to the next form field, even with one profile");
        app.input(key(KeyCode::BackTab));
        assert!(app.provider_body().1.contains("› provider:"));
        for _ in 0..6 {app.input(key(KeyCode::Tab));}
        app.input(key(KeyCode::Enter));
        assert!(app.provider_typing);
        app.input(key(KeyCode::Esc));
        assert_eq!(app.modal,Modal::Settings,"Escape cancels an edit before closing its form");
        assert!(!app.provider_typing);
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["context_bytes"],98304);
        assert!(app.take_intent().is_none());
        app.input(key(KeyCode::Enter));
        app.input(key(KeyCode::Tab));
        assert!(!app.provider_typing,"Tab accepts the field and returns to form navigation");
        assert!(app.provider_body().1.contains("› provider:"));
    }
    #[test]
    fn catalog_navigation_reaches_the_last_model_and_applies_only_an_explicit_selection() {
        let mut app=navigation_app();
        let key=|code|Event::Key(KeyEvent::new(code,KeyModifiers::NONE));
        app.input(key(KeyCode::Char('f')));
        app.input(key(KeyCode::End));
        let screen=capture(&mut app,80,24,"model-picker");
        assert!(screen.contains("model-59"),"End must reach and show a model beyond the old preview and initial provider");
        assert!(screen.contains("Esc"),"Back controls remain visible");
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["model"],"model-00");
        app.input(key(KeyCode::PageUp));
        app.input(key(KeyCode::PageDown));
        app.input(key(KeyCode::Up));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["model"],"model-58");
        assert!(app.take_intent().is_none(),"Browsing/selecting does not save, switch or authorize");
        app.input(key(KeyCode::Char('f')));
        app.input(Event::Paste("MODEL-42".into()));
        app.input(key(KeyCode::Enter));
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["model"],"model-42");
    }
    #[test]
    fn settings_focus_and_controls_stay_visible_and_manual_ids_remain_possible() {
        for (width,height) in [(60,18),(80,24),(120,40)] {
            let mut app=navigation_app();let key=|code|Event::Key(KeyEvent::new(code,KeyModifiers::NONE));
            for _ in 0..6{app.input(key(KeyCode::Down));}
            let screen=capture(&mut app,width,height,"settings");
            assert!(screen.contains("› working target"),"Selected field must stay visible at {width}x{height}");
            assert!(screen.contains("Esc back"),"Controls must stay visible");
            app.input(key(KeyCode::PageDown));
            assert!(capture(&mut app,width,height,"settings-scrolled").contains("Esc back"));
            for _ in 0..6{app.input(key(KeyCode::Up));}
            app.input(key(KeyCode::Char('e')));
            app.input(Event::Key(KeyEvent::new(KeyCode::Char('a'),KeyModifiers::CONTROL)));
            app.input(Event::Paste("provider-not-in-catalog".into()));
            app.input(key(KeyCode::Enter));
            assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["provider"],"provider-not-in-catalog");
            app.input(key(KeyCode::Char('f')));
            app.input(Event::Paste("no matching model".into()));
            let screen=capture(&mut app,width,height,"empty-picker");
            assert!(screen.contains("No catalog matches"));
            app.input(key(KeyCode::Enter));app.input(key(KeyCode::Esc));
            assert_eq!(app.modal,Modal::Settings);
            assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"]["default"]["provider"],"provider-not-in-catalog");
            assert!(app.take_intent().is_none());
        }
    }
    #[test]
    fn provider_picker_includes_empty_catalogs_and_escape_returns_one_level() {
        let mut app=navigation_app();
        let key=|code|Event::Key(KeyEvent::new(code,KeyModifiers::NONE));
        app.input(key(KeyCode::Char('v')));
        app.input(Event::Paste("provider-empty".into()));
        assert_eq!(app.catalog_entries().len(),1,"Providers with no catalog models remain visible");
        app.input(key(KeyCode::Enter));
        assert!(app.catalog_entries().is_empty());
        app.input(key(KeyCode::Esc));
        assert!(app.provider_picker.is_some(),"Escape returns to the provider list");
        app.input(key(KeyCode::Esc));
        assert_eq!(app.modal,Modal::Settings);
        assert!(app.provider_picker.is_none());
        assert!(app.take_intent().is_none());
    }
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
        capture(&mut app,80,24,"retired-settings");
        app.input(Event::Key(KeyEvent::new(KeyCode::End,KeyModifiers::NONE)));
        assert!(capture(&mut app,80,24,"retired-settings-scrolled").contains("Provider setup required"),"Setup status stays visible when the form scrolls");
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
        app.input(Event::Paste("Searchable".into()));
        app.handle_modal(KeyEvent::new(KeyCode::Enter,KeyModifiers::NONE));
        let selected=app.provider_names()[app.provider_index].clone();
        assert_eq!(app.provider_edit.as_ref().unwrap()["preferences"]["profiles"][&selected]["provider"],"brand-outside-core");
        assert!(app.provider_body().1.contains("custom-model"));
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.handle_modal(key(KeyCode::Down));
        assert_eq!(app.provider_field, 2); // arrows navigate fields; horizontal arrows choose profiles
        app.handle_modal(key(KeyCode::Right));
        assert_eq!(app.provider_index, 1);
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
