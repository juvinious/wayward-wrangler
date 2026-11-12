// Catalog metadata is supplied by the shared runtime, never a core allowlist.
enum CatalogScope {
    Providers,
    Models(Option<String>),
    Thinking,
}
struct CatalogPicker {
    scope: CatalogScope,
    query: Composer,
    index: usize,
    offset: usize,
    page_rows: usize,
    back_to_providers: bool,
}
impl App {
    fn catalog_entries(&self) -> Vec<Value> {
        let Some(picker) = &self.provider_picker else {
            return vec![];
        };
        let state = self.provider_state();
        let models = state["discovery"]["models"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut entries = match &picker.scope {
            CatalogScope::Providers => {
                let mut ids = state["discovery"]["providers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["id"].as_str().map(str::to_owned))
                    .collect::<Vec<_>>();
                ids.extend(
                    models
                        .iter()
                        .filter_map(|m| m["provider"].as_str().map(str::to_owned)),
                );
                ids.sort();
                ids.dedup();
                ids.into_iter()
                    .map(|id| {
                        let name = state["discovery"]["providers"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .find(|p| p["id"] == id)
                            .and_then(|p| p["name"].as_str())
                            .unwrap_or(&id);
                        serde_json::json!({"provider":id,"name":name,
                        "model_count":models.iter().filter(|m|m["provider"]==id).count()})
                    })
                    .collect::<Vec<_>>()
            }
            CatalogScope::Models(provider) => models
                .into_iter()
                .filter(|m| provider.as_ref().is_none_or(|p| m["provider"] == *p))
                .collect(),
            CatalogScope::Thinking => {
                let names = self.provider_names();
                names.get(self.provider_index%names.len().max(1)).map(|name|{
                    let profile=&self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][name];
                    self.model_metadata(profile)["thinking_levels"].as_array().into_iter().flatten().filter_map(|level|level.as_str())
                        .map(|level|serde_json::json!({"provider":profile["provider"],"model":profile["model"],"level":level,"name":level})).collect()
                }).unwrap_or_default()
            }
        };
        let query = picker.query.text().to_lowercase();
        entries.retain(|m| {
            let text = format!(
                "{}/{} {}",
                m["provider"].as_str().unwrap_or(""),
                m["model"].as_str().unwrap_or(""),
                m["name"].as_str().unwrap_or("")
            )
            .to_lowercase();
            query.split_whitespace().all(|word| text.contains(word))
        });
        entries
    }
    fn open_catalog(&mut self, scope: CatalogScope) {
        self.provider_typing = false;
        self.provider_picker = Some(CatalogPicker {
            scope,
            query: Composer::default(),
            index: 0,
            offset: 0,
            page_rows: 8,
            back_to_providers: false,
        });
        self.provider_picker
            .as_mut()
            .unwrap()
            .query
            .area
            .set_placeholder_text("Type provider, model ID or name");
        let names = self.provider_names();
        if let Some(name) = names.get(self.provider_index % names.len().max(1)) {
            let p = &self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][name];
            let index = self
                .catalog_entries()
                .iter()
                .position(|m| {
                    if matches!(
                        self.provider_picker.as_ref().unwrap().scope,
                        CatalogScope::Thinking
                    ) {
                        return m["level"] == p["options"]["reasoning"];
                    }
                    m["provider"] == p["provider"]
                        && (matches!(
                            self.provider_picker.as_ref().unwrap().scope,
                            CatalogScope::Providers
                        ) || m["model"] == p["model"])
                })
                .unwrap_or(0);
            self.provider_picker.as_mut().unwrap().index = index;
        }
        self.mark();
    }
    fn catalog_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Esc {
            if self.provider_picker.as_ref().unwrap().back_to_providers {
                self.open_catalog(CatalogScope::Providers);
            } else {
                self.provider_picker = None;
                self.provider_follow = true;
                self.mark();
            }
            return;
        }
        let entries = self.catalog_entries();
        let count = entries.len();
        if key.code == KeyCode::Enter {
            if key.kind != KeyEventKind::Press {
                return;
            }
            let picker = self.provider_picker.as_ref().unwrap();
            if let Some(selected) = entries.get(picker.index.min(count.saturating_sub(1))) {
                if matches!(picker.scope, CatalogScope::Providers) {
                    self.open_catalog(CatalogScope::Models(
                        selected["provider"].as_str().map(str::to_owned),
                    ));
                    self.provider_picker.as_mut().unwrap().back_to_providers = true;
                    return;
                }
                let names = self.provider_names();
                let name = &names[self.provider_index % names.len()];
                let p = &mut self.provider_edit.as_mut().unwrap()["preferences"]["profiles"][name];
                let thinking = matches!(picker.scope, CatalogScope::Thinking);
                if thinking {
                    p["options"]["reasoning"] = selected["level"].clone();
                } else {
                    let old_level = p["options"]["reasoning"].clone();
                    let level = if selected["thinking_levels"]
                        .as_array()
                        .is_some_and(|levels| levels.contains(&old_level))
                    {
                        old_level
                    } else {
                        selected["thinking_default"].clone()
                    };
                    if p["provider"] != selected["provider"] || p["model"] != selected["model"] {
                        p["options"] = serde_json::json!({});
                    }
                    p["provider"] = selected["provider"].clone();
                    p["model"] = selected["model"].clone();
                    if level.is_string() {
                        p["options"]["reasoning"] = level;
                    }
                    p["effort"] = serde_json::json!("provider_default");
                }
                self.provider_picker = None;
                self.provider_field = if thinking { 2 } else { 1 };
                self.provider_follow = true;
                self.notice="Model/thinking selected in draft; S saves. Coordinator switching and context grants stay explicit.".into();
            }
            self.mark();
            return;
        }
        let picker = self.provider_picker.as_mut().unwrap();
        let step = if key.modifiers.contains(KeyModifiers::ALT) {
            1
        } else {
            picker.page_rows.max(1)
        };
        match key.code {
            KeyCode::Up | KeyCode::BackTab => picker.index = picker.index.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => {
                picker.index = (picker.index + 1).min(count.saturating_sub(1))
            }
            KeyCode::PageUp => picker.index = picker.index.saturating_sub(step),
            KeyCode::PageDown => picker.index = (picker.index + step).min(count.saturating_sub(1)),
            KeyCode::Home => picker.index = 0,
            KeyCode::End => picker.index = count.saturating_sub(1),
            _ => {
                picker.query.input(Event::Key(key));
                picker.index = 0;
                picker.offset = 0;
            }
        }
        self.mark();
    }
    fn provider_footer(&self) -> String {
        if self.provider_picker.is_some() {
            return "Type to filter · ↑/↓ or Tab/Shift-Tab select · PgUp/PgDn page\nHome/End first/last · Enter choose · Esc back".into();
        }
        if self.provider_typing {
            return "Editing · arrows/Home/End cursor · Enter accept · Esc cancel\nTab/Shift-Tab accept and move focus · S saves after leaving the editor".into();
        }
        let local = match self.provider_section {
            0 => "↑/↓ or Tab/Shift-Tab field · Enter choose/edit · F models · A activate saved",
            1 => "↑/↓ role · ←/→ profile · Space allowed · Enter default · I inherit · W worker",
            2 => "↑/↓ role · ←/→ profile · Space append/remove fallback",
            3 => "←/→ profile · G context grant · I interrupt mode · Enter switch · C cancel",
            4 => "↑/↓ task · ←/→ profile · P pin · A allowed set · I inherit",
            5 => "Enter approve exact saved policy amendment",
            _ => "↑/↓ or Tab/Shift-Tab field · ←/→ choice · Enter edit",
        };
        format!(
            "{local}\n1–7 section · S save · L reload · R refresh · PgUp/PgDn scroll · Esc back"
        )
    }
    fn render_provider_modal(
        &mut self,
        frame: &mut Frame<'_>,
        screen: Rect,
        base: Style,
        accent: Color,
    ) {
        let rect = Rect::new(
            screen.x + 1,
            screen.y + 1,
            screen.width.saturating_sub(2).min(112),
            screen.height.saturating_sub(2).min(38),
        );
        let rect = Rect::new(
            screen.x + (screen.width.saturating_sub(rect.width)) / 2,
            screen.y + (screen.height.saturating_sub(rect.height)) / 2,
            rect.width,
            rect.height,
        );
        self.modal_rect=rect;
        frame.render_widget(Clear, rect);
        let title = if self.provider_activation.is_some(){"Activate saved model".into()}else if let Some(picker) = &self.provider_picker {
            match &picker.scope {
                CatalogScope::Providers => "Choose provider".to_owned(),
                CatalogScope::Models(Some(p)) => format!("Choose model · {p}"),
                CatalogScope::Models(None) => "Choose model · all providers".into(),
                CatalogScope::Thinking => "Choose thinking level".into(),
            }
        } else {
            self.provider_body().0
        };
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(Self::safe(&title))
                .style(base)
                .border_style(base.fg(accent)),
            rect,
        );
        let inner = Rect::new(
            rect.x + 1,
            rect.y + 1,
            rect.width.saturating_sub(2),
            rect.height.saturating_sub(2),
        );
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        if self.provider_activation.is_some(){self.render_activation_review(frame,inner,base,accent);return;}
        let footer = self.provider_footer();
        let footer = Paragraph::new(footer)
            .wrap(Wrap { trim: false })
            .style(base.fg(accent));
        let status_h = if self.provider_picker.is_none() && !self.notice.is_empty() {
            1
        } else {
            0
        };
        let footer_h = (footer.line_count(inner.width) as u16)
            .saturating_add(status_h)
            .min(inner.height);
        let area = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner.height.saturating_sub(footer_h),
        );
        frame.render_widget(
            footer,
            Rect::new(
                inner.x,
                inner.y + area.height,
                inner.width,
                footer_h.saturating_sub(status_h),
            ),
        );
        if status_h > 0 {
            frame.render_widget(
                Paragraph::new(Self::safe(&self.notice)).style(base.fg(accent)),
                Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
            );
        }
        if area.height == 0 {
            return;
        }
        if self.provider_picker.is_some() {
            let entries = self.catalog_entries();
            let picker = self.provider_picker.as_mut().unwrap();
            picker.index = picker.index.min(entries.len().saturating_sub(1));
            let header_h = 3.min(area.height);
            let rows = area.height.saturating_sub(header_h) as usize;
            picker.page_rows = rows.max(1);
            if picker.index < picker.offset {
                picker.offset = picker.index;
            }
            if picker.index >= picker.offset + rows.max(1) {
                picker.offset = picker.index + 1 - rows.max(1);
            }
            picker.offset = picker.offset.min(entries.len().saturating_sub(rows.max(1)));
            let details = entries
                .get(picker.index)
                .map(|m| {
                    if matches!(picker.scope, CatalogScope::Providers) {
                        format!(
                            "{} · {} catalog models",
                            m["name"].as_str().unwrap_or(""),
                            m["model_count"]
                        )
                    } else if matches!(picker.scope, CatalogScope::Thinking) {
                        format!(
                            "Thinking level for {}/{}",
                            m["provider"].as_str().unwrap_or(""),
                            m["model"].as_str().unwrap_or("")
                        )
                    } else {
                        format!(
                            "{} · context {}",
                            m["name"].as_str().unwrap_or(""),
                            m["context_window"]
                        )
                    }
                })
                .unwrap_or_default();
            let header = format!(
                "Search:\n{} matches · selected {}/{} · authentication not checked\n{}",
                entries.len(),
                if entries.is_empty() {
                    0
                } else {
                    picker.index + 1
                },
                entries.len(),
                Self::safe(&details)
            );
            frame.render_widget(
                Paragraph::new(header).style(base),
                Rect::new(area.x, area.y, area.width, header_h),
            );
            picker.query.area.set_style(base);
            picker
                .query
                .area
                .set_cursor_style(base.add_modifier(Modifier::REVERSED));
            frame.render_widget(
                &picker.query.area,
                Rect::new(
                    area.x + 8.min(area.width),
                    area.y,
                    area.width.saturating_sub(8),
                    1,
                ),
            );
            let lines = entries
                .iter()
                .enumerate()
                .skip(picker.offset)
                .take(rows)
                .map(|(i, m)| {
                    let label = if matches!(picker.scope, CatalogScope::Providers) {
                        format!(
                            "{} {} · {} models",
                            if i == picker.index { "›" } else { " " },
                            m["provider"].as_str().unwrap_or("?"),
                            m["model_count"]
                        )
                    } else if matches!(picker.scope, CatalogScope::Thinking) {
                        format!(
                            "{} {}",
                            if i == picker.index { "›" } else { " " },
                            m["level"].as_str().unwrap_or("?")
                        )
                    } else {
                        format!(
                            "{} {}/{}",
                            if i == picker.index { "›" } else { " " },
                            m["provider"].as_str().unwrap_or("?"),
                            m["model"].as_str().unwrap_or("?")
                        )
                    };
                    Line::styled(
                        Self::safe(&label),
                        if i == picker.index {
                            base.fg(accent)
                                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
                        } else {
                            base
                        },
                    )
                })
                .collect::<Vec<_>>();
            let list = Rect::new(area.x, area.y + header_h, area.width, rows as u16);
            if entries.is_empty() {
                let empty = if matches!(picker.scope, CatalogScope::Thinking) {
                    "Thinking levels not reported. Esc returns; R refreshes metadata. Exact options remain editable."
                } else {
                    "No catalog matches. Esc returns; E edits an exact provider/model ID in the form."
                };
                frame.render_widget(
                    Paragraph::new(empty).wrap(Wrap { trim: false }).style(base),
                    list,
                );
            } else {
                frame.render_widget(Paragraph::new(lines), list);
            }
            return;
        }
        let (_, body) = self.provider_body();
        let tabs = "1 Profiles · 2 Roles · 3 Fallback · 4 Switch · 5 Tasks · 6 Policy · 7 Project";
        let profile_line = if self.provider_section == 6 {
            "Project setup · draft only".to_owned()
        } else {
            format!(
                "Profile: {} · ←/→ chooses profile",
                self.provider_names()
                    .get(self.provider_index % self.provider_names().len().max(1))
                    .map(String::as_str)
                    .unwrap_or("default")
            )
        };
        let setup_line = if self.provider_state()["setup_required"] == true {
            "\nProvider setup required"
        } else {
            ""
        };
        let names = self.provider_names();
        let draft = names
            .get(self.provider_index % names.len().max(1))
            .map(|name| &self.provider_edit.as_ref().unwrap()["preferences"]["profiles"][name]);
        let draft_line = draft
            .map(|p| {
                format!(
                    "\nDraft: {} · thinking {}",
                    p["model"].as_str().unwrap_or("unconfigured"),
                    self.thinking_label(p)
                )
            })
            .unwrap_or_default();
        let active_line = self
            .active_model_label()
            .map(|s| format!("\nActive {s}"))
            .unwrap_or_default();
        let header = Paragraph::new(format!(
            "{tabs}\n{profile_line}{draft_line}{active_line}{setup_line}"
        ))
        .wrap(Wrap { trim: false })
        .style(base.fg(accent));
        let header_h = (header.line_count(area.width) as u16).min(area.height);
        frame.render_widget(header, Rect::new(area.x, area.y, area.width, header_h));
        let mut content = Rect::new(
            area.x,
            area.y + header_h,
            area.width,
            area.height.saturating_sub(header_h),
        );
        if self.provider_typing && content.height >= 4 {
            content.height -= 4;
            frame.render_widget(
                Paragraph::new("Edit value:").style(base.fg(accent)),
                Rect::new(content.x, content.y + content.height, content.width, 1),
            );
            self.scope_editor.area.set_style(base);
            self.scope_editor
                .area
                .set_cursor_style(base.add_modifier(Modifier::REVERSED));
            frame.render_widget(
                &self.scope_editor.area,
                Rect::new(content.x, content.y + content.height + 1, content.width, 3),
            );
        }
        if content.width == 0 || content.height == 0 {
            return;
        }
        let paragraph = Paragraph::new(Text::from(markdown_lines(
            &Self::safe(&body),
            content.width,
            Tone::for_theme(&self.journal.theme),
            false,
        )))
        .wrap(Wrap { trim: false })
        .style(base);
        self.modal_scroll_limit = paragraph
            .line_count(content.width)
            .saturating_sub(content.height as usize)
            .min(u16::MAX as usize) as u16;
        if self.provider_follow {
            if let Some(index) = body.lines().position(|line| line.starts_with("› ")) {
                let prefix = body.lines().take(index + 1).collect::<Vec<_>>().join("\n");
                let line = Paragraph::new(Text::from(markdown_lines(
                    &Self::safe(&prefix),
                    content.width,
                    Tone::for_theme(&self.journal.theme),
                    false,
                )))
                .wrap(Wrap { trim: false })
                .line_count(content.width)
                .saturating_sub(1)
                .min(u16::MAX as usize) as u16;
                if line < self.modal_scroll {
                    self.modal_scroll = line;
                }
                if line >= self.modal_scroll.saturating_add(content.height) {
                    self.modal_scroll = line.saturating_add(1).saturating_sub(content.height);
                }
            }
            self.provider_follow = false;
        }
        self.modal_scroll = self.modal_scroll.min(self.modal_scroll_limit);
        frame.render_widget(paragraph.scroll((self.modal_scroll, 0)), content);
    }
}
