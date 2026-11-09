impl App {
    fn project_draft(&self) -> Value {
        self.project_edit.clone().unwrap_or_else(||{
            let state=&self.development_view()["project_setup"];
            if state["settings"].is_object(){state["settings"].clone()}else{
                serde_json::json!({"version":1,"revision":0,"baseline":"HEAD","baseline_input":"committed","target":state["observed"]["recommended_target"],"completion":"checked-local","human_validation":false,"retention":"retain","remote":null,"repository":null,"merge_method":"squash","command_recipes":[]})
            }
        })
    }
    fn project_key(&mut self, key: KeyEvent) {
        const FIELDS: [&str; 11] = [
            "baseline",
            "baseline_input",
            "target",
            "completion",
            "human_validation",
            "retention",
            "generated_outputs",
            "remote",
            "repository",
            "merge_method",
            "command_recipes",
        ];
        if self.project_edit.is_none() {
            self.project_edit = Some(self.project_draft());
        }
        let field = FIELDS[self.provider_row % FIELDS.len()];
        if self.provider_typing {
            match key.code {
                KeyCode::Esc => self.provider_typing = false,
                KeyCode::Enter => {
                    let value = self.scope_editor.text();
                    self.project_edit.as_mut().unwrap()[field] = if matches!(field,"command_recipes"|"generated_outputs") {
                        match serde_json::from_str(&value) {
                            Ok(v) => v,
                            Err(e) => {
                                self.notice = e.to_string();
                                return;
                            }
                        }
                    } else {
                        serde_json::json!(value)
                    };
                    self.provider_typing = false;
                }
                _ => {
                    self.scope_editor.input(Event::Key(key));
                }
            }
            self.mark();
            return;
        }
        match key.code {
            KeyCode::Up => {
                self.provider_row = (self.provider_row + FIELDS.len() - 1) % FIELDS.len()
            }
            KeyCode::Down | KeyCode::Tab => {
                self.provider_row = (self.provider_row + 1) % FIELDS.len()
            }
            KeyCode::Char('l') => self.project_edit = None,
            KeyCode::Char('s') => {
                let settings = self.project_draft();
                match waywr_backend::project_workflow::validate(&settings) {
                    Ok(()) => {
                        self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"project.setup","payload":{"expected_revision":settings["revision"],"settings":settings,"subject_id":self.conversation_subject}})));
                        self.project_edit = None;
                        self.notice="Project setup save pending; initial scope approval grants execution, this save does not".into();
                    }
                    Err(e) => self.notice = e,
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                let v = &mut self.project_edit.as_mut().unwrap()[field];
                let options: &[&str] = match field {
                    "baseline_input" => &["committed", "initialize-files"],
                    "completion" => &["checked-local", "github-pr"],
                    "retention" => &["retain", "retire-clean-owned"],
                    "merge_method" => &["squash", "merge", "rebase"],
                    _ => &[],
                };
                if field == "human_validation" {
                    *v = serde_json::json!(!v.as_bool().unwrap_or(false));
                } else if !options.is_empty() {
                    let pos = options
                        .iter()
                        .position(|x| Some(*x) == v.as_str())
                        .unwrap_or(0);
                    let step = if key.code == KeyCode::Left {
                        options.len() - 1
                    } else {
                        1
                    };
                    *v = serde_json::json!(options[(pos + step) % options.len()]);
                }
            }
            KeyCode::Enter => {
                if matches!(
                    field,
                    "baseline_input"
                        | "completion"
                        | "human_validation"
                        | "retention"
                        | "merge_method"
                ) {
                    self.project_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
                    return;
                }
                self.scope_editor = Composer::default();
                let v = &self.project_draft()[field];
                self.scope_editor.input(Event::Paste(
                    v.as_str().map(str::to_owned).unwrap_or_else(|| {
                        if v.is_null() {
                            String::new()
                        } else {
                            v.to_string()
                        }
                    }),
                ));
                self.provider_typing = true;
            }
            KeyCode::Esc => self.modal = Modal::None,
            _ => {}
        }
        self.mark();
    }
    fn project_body(&self) -> (String, String) {
        let draft = self.project_draft();
        let mut text="[1 Profiles] [2 Roles] [3 Fallback] [4 Switch] [5 Tasks] [6 Policy] [7 Project]\nProject setup · future request scope/policy approval includes these choices.\n↑/↓ selects · ←/→ changes choice · Enter edits · S saves · L reloads\nExisting approved requests keep their exact saved policy.\n\n".to_owned();
        for (i, field) in [
            "baseline",
            "baseline_input",
            "target",
            "completion",
            "human_validation",
            "retention",
            "generated_outputs",
            "remote",
            "repository",
            "merge_method",
            "command_recipes",
        ]
        .iter()
        .enumerate()
        {
            text.push_str(&format!(
                "{} {}: {}\n",
                if self.provider_row == i { "›" } else { " " },
                field,
                draft[*field]
            ));
        }
        text.push_str("\nCommitted input preserves unrelated dirty edits. initialize-files requires explicit approval of the listed source manifest for a non-Git/unborn project. PR policy grants the exact remote/repository and respects repository checks/reviews. Preview recipes are instructions, not permission to start a long-lived process. Generated outputs: optional exact output roots (JSON array); only new inventoried ignored files can retire, never preexisting/changed files or dependency directories.\n");
        text.push_str(&format!(
            "\nObserved Git/setup:\n{}",
            serde_json::to_string_pretty(&self.development_view()["project_setup"])
                .unwrap_or_default()
        ));
        ("Project baseline, target and completion".into(), text)
    }
}
