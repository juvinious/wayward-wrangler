use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::Style;
use ratatui_textarea::{CursorMove, TextArea, WrapMode};
use serde::{Deserialize, Serialize};

pub const MAX_DRAFT: usize = 32 * 1024;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Draft {
    pub text: String,
    pub row: usize,
    pub col: usize,
    pub multiline: bool,
    pub literal_paste: bool,
}

pub enum EditResult {
    Changed,
    Submit(String),
    Nothing,
    Refused(&'static str),
}

pub struct Composer {
    pub area: TextArea<'static>,
    pub multiline: bool,
    pub literal_paste: bool,
}

impl Default for Composer {
    fn default() -> Self {
        Self::from_draft(Draft::default())
    }
}

impl Composer {
    pub fn from_draft(draft: Draft) -> Self {
        let mut area = TextArea::new(draft.text.split('\n').map(str::to_owned).collect());
        area.set_max_histories(100);
        area.set_wrap_mode(WrapMode::WordOrGlyph);
        // TextArea defaults to underlining the *entire cursor line*. Keep only
        // its reversed cursor cell so ordinary draft text remains plain.
        area.set_cursor_line_style(Style::default());
        area.set_placeholder_text("Describe work or ask the scripted coordinator…");
        area.move_cursor(CursorMove::Jump(
            draft.row.min(u16::MAX as usize) as u16,
            draft.col.min(u16::MAX as usize) as u16,
        ));
        Self {
            area,
            multiline: draft.multiline,
            literal_paste: draft.literal_paste,
        }
    }

    pub fn text(&self) -> String {
        self.area.lines().join("\n")
    }

    pub fn snapshot(&self) -> Draft {
        let cursor = self.area.cursor();
        Draft {
            text: self.text(),
            row: cursor.0,
            col: cursor.1,
            multiline: self.multiline,
            literal_paste: self.literal_paste,
        }
    }

    pub fn clear(&mut self) {
        self.area.clear();
        self.literal_paste = false;
    }

    pub fn take_submission(&mut self) -> EditResult {
        let text = self.text();
        if text.trim().is_empty() {
            return EditResult::Nothing;
        }
        self.clear();
        EditResult::Submit(text)
    }

    pub fn insert_paste(&mut self, value: &str) -> EditResult {
        if self.text().len() + value.len() > MAX_DRAFT {
            return EditResult::Refused("Paste refused: the draft limit is 32 KiB");
        }
        self.area.insert_str(value);
        self.literal_paste = true;
        EditResult::Changed
    }

    pub fn input(&mut self, event: Event) -> EditResult {
        let Event::Key(key) = event else {
            return if let Event::Paste(value) = event {
                self.insert_paste(&value)
            } else {
                EditResult::Nothing
            };
        };
        if key.kind == KeyEventKind::Release {
            return EditResult::Nothing;
        }
        let KeyEvent {
            code,
            modifiers,
            kind,
            ..
        } = key;
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        if ctrl {
            match code {
                KeyCode::Char('j') => {
                    if self.text().len()>=MAX_DRAFT{return EditResult::Refused("Draft limit is 32 KiB");}
                    self.area.insert_newline();return EditResult::Changed;
                },
                KeyCode::Char('z') => {
                    self.area.undo();
                    return EditResult::Changed;
                }
                KeyCode::Char('y') => {
                    self.area.redo();
                    return EditResult::Changed;
                }
                KeyCode::Char('a') => {
                    self.area.select_all();
                    return EditResult::Changed;
                }
                KeyCode::Char('c') => {
                    self.area.copy();
                    return EditResult::Nothing;
                }
                KeyCode::Char('x') => {
                    self.area.cut();
                    return EditResult::Changed;
                }
                KeyCode::Char('v') => {
                    if self.text().len() + self.area.yank_text().len() > MAX_DRAFT {
                        return EditResult::Refused("Paste refused: the draft limit is 32 KiB");
                    }
                    self.area.paste();
                    return EditResult::Changed;
                }
                KeyCode::Home => {
                    self.area.move_cursor(CursorMove::Top);
                    return EditResult::Changed;
                }
                KeyCode::End => {
                    self.area.move_cursor(CursorMove::Bottom);
                    return EditResult::Changed;
                }
                _ => return EditResult::Nothing,
            }
        }
        match code {
            KeyCode::F(2) => {
                self.multiline = !self.multiline;
                EditResult::Changed
            }
            KeyCode::Enter if shift || self.multiline => {
                if self.text().len() == MAX_DRAFT {
                    return EditResult::Refused("Draft limit is 32 KiB");
                }
                self.area.insert_newline();
                EditResult::Changed
            }
            KeyCode::Enter if kind != KeyEventKind::Repeat => self.take_submission(),
            KeyCode::Enter => EditResult::Nothing,
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                if shift && self.area.selection_range().is_none() {
                    self.area.start_selection();
                }
                if !shift {
                    self.area.cancel_selection();
                }
                self.area.move_cursor(match code {
                    KeyCode::Left => CursorMove::Back,
                    KeyCode::Right => CursorMove::Forward,
                    KeyCode::Up => CursorMove::Up,
                    _ => CursorMove::Down,
                });
                EditResult::Changed
            }
            KeyCode::Home => {
                self.area.move_cursor(CursorMove::Head);
                EditResult::Changed
            }
            KeyCode::End => {
                self.area.move_cursor(CursorMove::End);
                EditResult::Changed
            }
            KeyCode::Backspace => {
                self.area.delete_char();
                EditResult::Changed
            }
            KeyCode::Delete => {
                self.area.delete_next_char();
                EditResult::Changed
            }
            KeyCode::Char(c) if !alt => {
                if self.text().len() + c.len_utf8() > MAX_DRAFT {
                    return EditResult::Refused("Draft limit is 32 KiB");
                }
                self.area.insert_char(c);
                EditResult::Changed
            }
            _ => EditResult::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: KeyCode, mods: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, mods))
    }

    #[test]
    fn enter_submits_and_multiline_fallback_keeps_text() {
        let mut c = Composer::default();
        c.input(key(KeyCode::Char('a'), KeyModifiers::NONE));
        c.input(key(KeyCode::F(2), KeyModifiers::NONE));
        assert!(matches!(
            c.input(key(KeyCode::Enter, KeyModifiers::NONE)),
            EditResult::Changed
        ));
        c.input(key(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(c.text(), "a\nb");
        assert!(matches!(c.take_submission(), EditResult::Submit(s) if s=="a\nb"));
        assert!(c.text().is_empty());
    }

    #[test]
    fn shifted_enter_adds_newline_without_enabling_multiline_mode() {
        let mut c = Composer::default();
        c.input(key(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(matches!(
            c.input(key(KeyCode::Enter, KeyModifiers::SHIFT)),
            EditResult::Changed
        ));
        c.input(key(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(c.text(), "a\nb");
        assert!(!c.multiline);
        assert!(
            matches!(c.input(key(KeyCode::Enter, KeyModifiers::NONE)), EditResult::Submit(s) if s == "a\nb")
        );
    }

    #[test]
    fn paste_is_literal_bounded_and_undoable() {
        let mut c = Composer::default();
        assert!(matches!(
            c.input(Event::Paste("/approve\npath".into())),
            EditResult::Changed
        ));
        assert!(c.literal_paste);
        assert_eq!(c.text(), "/approve\npath");
        assert!(matches!(
            c.input(Event::Paste("x".repeat(MAX_DRAFT))),
            EditResult::Refused(_)
        ));
        assert_eq!(c.text(), "/approve\npath");
        c.input(key(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(c.text().is_empty());
        c.input(key(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(c.text(), "/approve\npath");
    }

    #[test]
    fn movement_and_snapshot_restore() {
        let mut c = Composer::default();
        c.input(Event::Paste("a界\nb".into()));
        c.input(key(KeyCode::Up, KeyModifiers::NONE));
        let before = c.snapshot();
        let again = Composer::from_draft(before.clone());
        assert_eq!(again.text(), before.text);
        assert_eq!(again.area.cursor(), c.area.cursor());
    }
}
