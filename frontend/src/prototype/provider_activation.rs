// A local review of a saved profile. Only explicit confirmation emits a switch;
// backend profile, policy, generation and settlement checks remain authoritative.
struct ActivationReview {
    profile_id: String,
    profile: Value,
    revision: u64,
    generation: u64,
    subject: String,
    activate: bool,
    from_composer: bool,
}
impl App {
    fn model_setup_needed(&self)->bool {
        self.development_view()["providers"]["setup_required"]==true
    }
    fn open_model_setup(&mut self){
        self.open(Modal::Settings);self.provider_section=0;
        let state=self.provider_state();
        let names=self.provider_names();
        self.provider_index=names.iter().position(|name|Some(name.as_str())==state["preferences"]["coordinator"].as_str()).unwrap_or(0);
        if state["preferences"]["schema_version"]==3{
            self.review_profile_activation();
            if let Some(review)=self.provider_activation.as_mut(){review.from_composer=true;}
        }else{
            self.notice="Choose a model and thinking level, S saves, then A reviews activation. Your message is still in the composer.".into();
        }
    }
    fn finish_activation_review(&mut self)->Option<ActivationReview>{
        let review=self.provider_activation.take()?;
        self.provider_follow=true;self.modal_scroll=0;
        if review.from_composer{self.modal=Modal::None;self.focus=Focus::Composer;}
        Some(review)
    }
    fn review_profile_activation(&mut self) {
        let saved = self.provider_state();
        let Some(edit) = self.provider_edit.as_ref() else {
            return;
        };
        if saved["preferences"]["schema_version"] != 3
            || edit["preferences"] != saved["preferences"]
        {
            self.notice = "Save changes first with S, then A activates the saved model".into();
            return;
        }
        if matches!(
            saved["switch"]["status"].as_str(),
            Some("requested" | "waiting" | "blocked")
        ) {
            self.notice =
                "A switch is already pending; section4 shows its status and cancellation controls"
                    .into();
            return;
        }
        let names = self.provider_names();
        let Some(name) = names.get(self.provider_index % names.len().max(1)) else {
            return;
        };
        let prefs =
            match waywr_backend::provider_selection::Preferences::parse(&saved["preferences"]) {
                Ok(p) => p,
                Err(reason) => {
                    self.notice = reason;
                    return;
                }
            };
        if !prefs.role_allowed("coordinator").contains(name) {
            self.notice =
                "This saved profile is not allowed for coordinator; review section2 Roles first"
                    .into();
            return;
        }
        let profile = saved["preferences"]["profiles"][name].clone();
        if profile == saved["coordinator"]["profile"] {
            self.notice = "This saved model is already the current coordinator binding".into();
            return;
        }
        let generation = self
            .live
            .as_ref()
            .and_then(|l| l.conversation.as_ref())
            .and_then(|c| c["binding"]["generation"].as_u64());
        let (Some(revision), Some(generation)) = (saved["revision"].as_u64(), generation) else {
            self.notice =
                "Coordinator state is still loading; wait for its verified snapshot".into();
            return;
        };
        self.provider_activation = Some(ActivationReview {
            profile_id: name.clone(),
            profile,
            revision,
            generation,
            subject: self.conversation_subject.clone(),
            activate: false,
            from_composer: false,
        });
        self.modal_scroll = 0;
        self.notice.clear();
    }
    fn activation_review_current(&self, review: &ActivationReview) -> bool {
        let saved = self.provider_state();
        saved["revision"].as_u64() == Some(review.revision)
            && saved["preferences"]["profiles"][&review.profile_id] == review.profile
            && self
                .live
                .as_ref()
                .and_then(|l| l.conversation.as_ref())
                .and_then(|c| c["binding"]["generation"].as_u64())
                == Some(review.generation)
    }
    fn activation_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Esc {
            self.finish_activation_review();
            self.mark();
            return;
        }
        if matches!(
            key.code,
            KeyCode::PageUp | KeyCode::PageDown | KeyCode::Home | KeyCode::End
        ) {
            let step = if key.modifiers.contains(KeyModifiers::ALT) {
                1
            } else {
                8
            };
            self.modal_scroll = match key.code {
                KeyCode::Home => 0,
                KeyCode::End => self.modal_scroll_limit,
                KeyCode::PageUp => self.modal_scroll.saturating_sub(step),
                _ => self
                    .modal_scroll
                    .saturating_add(step)
                    .min(self.modal_scroll_limit),
            };
        } else if matches!(
            key.code,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab
        ) {
            let review = self.provider_activation.as_mut().unwrap();
            review.activate = match key.code {
                KeyCode::Left | KeyCode::Up => false,
                KeyCode::Right | KeyCode::Down => true,
                _ => !review.activate,
            };
        } else if key.code == KeyCode::Enter && key.kind == KeyEventKind::Press {
            let review = self.provider_activation.as_ref().unwrap();
            if !review.activate {
                self.finish_activation_review();
                self.mark();
                return;
            }
            if !self.activation_review_current(review) {
                self.notice =
                    "Saved model or coordinator changed; Esc and reopen the activation review"
                        .into();
                self.mark();
                return;
            }
            let review = self.finish_activation_review().unwrap();
            self.intent(LiveIntent::Dialogue(serde_json::json!({"action":"providers.switch","payload":{
                "profile_id":review.profile_id,"expected_revision":review.revision,"expected_generation":review.generation,
                "subject_id":review.subject,"mode":"after_current_turn","grant_context":true}})));
            self.notice="Activation requested after the current turn; pause and work approvals stay in effect".into();
            self.modal_scroll = 0;
            self.provider_follow = true;
        }
        self.mark();
    }
    fn render_activation_review(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        base: Style,
        accent: Color,
    ) {
        let review = self.provider_activation.as_ref().unwrap();
        let current = self.activation_review_current(review);
        let profile = &review.profile;
        let route = if profile["transport"]["destination"]["kind"] == "runtime_builtin" {
            "SDK provider route".into()
        } else {
            serde_json::to_string(&profile["transport"]["destination"]).unwrap_or_default()
        };
        let body = format!(
            "Use this saved model for coordinator conversations after the current turn. It may receive this project's selected discussion and coordinator context.\n\nDestination: {}\nOptions: {}\n\nExisting requests keep their own policies. A restricted request needs its own policy amendment before this model can work on it; it does not block new discussions. Workers keep their assignments.\n\nYour message stays unsent. After activation, Enter sends it. Activation does not sign in, resume paused work or interrupt an uncertain call.{}",
            route,
            profile["options"],
            if current {
                ""
            } else {
                "\n\nSettings changed: Esc and reopen this review before activation."
            }
        );
        let choices = format!(
            "{} Keep current   {} Activate and share project context\n←/→ or Tab/Shift-Tab chooses · Enter confirms · Esc back\nPgUp/PgDn reads details · Home/End start/end",
            if review.activate { " " } else { "›" },
            if review.activate { "›" } else { " " }
        );
        let footer = Paragraph::new(choices)
            .wrap(Wrap { trim: false })
            .style(base.fg(accent));
        let footer_h = (footer.line_count(area.width) as u16)
            .saturating_add(1)
            .min(area.height);
        let summary = Paragraph::new(Self::safe(&format!(
            "{}/{}\nThinking: {} · saved profile {}",
            profile["provider"].as_str().unwrap_or("?"),
            profile["model"].as_str().unwrap_or("?"),
            self.thinking_label(profile),
            review.profile_id
        )))
        .wrap(Wrap { trim: false })
        .style(base.fg(accent));
        let summary_h = (summary.line_count(area.width) as u16)
            .min(3)
            .min(area.height.saturating_sub(footer_h));
        frame.render_widget(summary, Rect::new(area.x, area.y, area.width, summary_h));
        let content = Rect::new(
            area.x,
            area.y + summary_h,
            area.width,
            area.height.saturating_sub(footer_h + summary_h),
        );
        let text = Paragraph::new(Self::safe(&body))
            .wrap(Wrap { trim: false })
            .style(base);
        self.modal_scroll_limit = text
            .line_count(content.width)
            .saturating_sub(content.height as usize)
            .min(u16::MAX as usize) as u16;
        self.modal_scroll = self.modal_scroll.min(self.modal_scroll_limit);
        frame.render_widget(text.scroll((self.modal_scroll, 0)), content);
        frame.render_widget(
            footer,
            Rect::new(
                area.x,
                content.y + content.height,
                area.width,
                footer_h.saturating_sub(1),
            ),
        );
        if area.height > 0 {
            frame.render_widget(
                Paragraph::new(Self::safe(if current {
                    &self.notice
                } else {
                    "Saved settings changed; Esc and reopen this review"
                }))
                .style(base.fg(accent)),
                Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            );
        }
    }
}
