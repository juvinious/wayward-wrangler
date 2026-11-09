//! Small, bounded presentation formatter for the production and scripted UI. Input is never
//! interpreted as an action; this module only builds Ratatui spans.
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy)]
pub struct Tone {
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub path: Color,
    pub option: Color,
    pub code: Color,
    pub success: Color,
    pub failure: Color,
    pub attention: Color,
}

impl Tone {
    pub fn for_theme(theme: &str) -> Self {
        match theme {
            "light" => Self {
                text: Color::Black,
                muted: Color::DarkGray,
                accent: Color::Blue,
                path: Color::Blue,
                option: Color::Magenta,
                code: Color::Rgb(116, 54, 0),
                success: Color::Green,
                failure: Color::Red,
                attention: Color::Rgb(127, 75, 0),
            },
            "mono" => Self {
                text: Color::Reset,
                muted: Color::Reset,
                accent: Color::Reset,
                path: Color::Reset,
                option: Color::Reset,
                code: Color::Reset,
                success: Color::Reset,
                failure: Color::Reset,
                attention: Color::Reset,
            },
            _ => Self {
                text: Color::White,
                muted: Color::Gray,
                accent: Color::Cyan,
                path: Color::LightBlue,
                option: Color::Magenta,
                code: Color::Yellow,
                success: Color::LightGreen,
                failure: Color::LightRed,
                attention: Color::Yellow,
            },
        }
    }
}

fn width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn split_long_word(word: &str, available: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut part = String::new();
    let mut used = 0;
    for ch in word.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > available && !part.is_empty() {
            out.push(std::mem::take(&mut part));
            used = 0;
        }
        part.push(ch);
        used += cw;
    }
    if !part.is_empty() {
        out.push(part);
    }
    out
}

fn push_span(spans: &mut Vec<Span<'static>>, text: &str, style: Style) {
    if let Some(last) = spans.last_mut().filter(|s| s.style == style) {
        last.content.to_mut().push_str(text);
    } else if !text.is_empty() {
        spans.push(Span::styled(text.to_owned(), style));
    }
}

/// Wrap styled words after parsing, so an emphasis/code delimiter crossing a
/// visual line cannot become literal or lose its style. Preserve list alignment.
fn wrap_spans(
    spans: Vec<Span<'static>>,
    prefix: &str,
    continuation: &str,
    max_width: usize,
) -> Vec<Line<'static>> {
    let chars: Vec<_> = spans
        .iter()
        .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
        .collect();
    let mut out = Vec::new();
    let mut line = vec![Span::raw(prefix.to_owned())];
    let mut used = width(prefix);
    let mut has_word = false;
    let available = max_width.saturating_sub(width(continuation)).max(1);
    for word in chars
        .split(|(c, _)| c.is_whitespace())
        .filter(|w| !w.is_empty())
    {
        let mut start = 0;
        while start < word.len() {
            let mut end = start;
            let mut piece_width = 0;
            while end < word.len() {
                let cw = UnicodeWidthChar::width(word[end].0).unwrap_or(0);
                if piece_width + cw > available && end > start {
                    break;
                }
                piece_width += cw;
                end += 1;
            }
            let gap = usize::from(has_word);
            if used + gap + piece_width > max_width && has_word {
                out.push(Line::from(line));
                line = vec![Span::raw(continuation.to_owned())];
                used = width(continuation);
                has_word = false;
            }
            if has_word {
                push_span(&mut line, " ", Style::default());
                used += 1;
            }
            for (ch, style) in &word[start..end] {
                push_span(&mut line, &ch.to_string(), *style);
            }
            used += piece_width;
            has_word = true;
            start = end;
        }
    }
    out.push(Line::from(line));
    out
}

fn is_path(token: &str) -> bool {
    let token = token.trim_matches(|c: char| ",.;:()[]{}<>\"'".contains(c));
    token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with('/')
        || token.contains("/artifacts/")
        || [".md", ".json", ".rs", ".py", ".toml"]
            .iter()
            .any(|suffix| token.ends_with(suffix))
}

fn inline(value: &str, tone: Tone, user: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if user {
        return vec![Span::raw(value.to_owned())];
    }
    inline_inner(value, tone, Modifier::empty(), 0, &mut spans);
    spans
}

fn plain(value: &str, tone: Tone, modifier: Modifier, spans: &mut Vec<Span<'static>>) {
    let part = value;
    let mut rest = part;
    while !rest.is_empty() {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        if end == 0 {
            let count = rest.chars().take_while(|c| c.is_whitespace()).count();
            let bytes = rest
                .char_indices()
                .nth(count)
                .map_or(rest.len(), |(i, _)| i);
            push_span(
                spans,
                &rest[..bytes],
                Style::default().add_modifier(modifier),
            );
            rest = &rest[bytes..];
            continue;
        }
        let token = &rest[..end];
        let style = if is_path(token) {
            Style::default().fg(tone.path).add_modifier(Modifier::BOLD)
        } else if ["Allow none", "Require one"].contains(&token) {
            Style::default().fg(tone.option)
        } else {
            Style::default()
        };
        push_span(spans, token, style.add_modifier(modifier));
        rest = &rest[end..];
    }
}

/// Deliberately bounded inline subset: code, strong/emphasis and escaped markers.
/// Unmatched/streaming delimiters stay literal. Code never parses emphasis, and
/// underscores inside identifiers/paths cannot become emphasis.
fn inline_inner(
    value: &str,
    tone: Tone,
    modifier: Modifier,
    depth: usize,
    spans: &mut Vec<Span<'static>>,
) {
    if depth >= 8 {
        plain(value, tone, modifier, spans);
        return;
    }
    let mut i = 0;
    let mut literal = 0;
    while i < value.len() {
        let rest = &value[i..];
        if rest.starts_with('\\') {
            if let Some(ch) = rest.chars().nth(1).filter(|c| "*_`\\".contains(*c)) {
                plain(&value[literal..i], tone, modifier, spans);
                push_span(
                    spans,
                    &ch.to_string(),
                    Style::default().add_modifier(modifier),
                );
                i += 1 + ch.len_utf8();
                literal = i;
                continue;
            }
        }
        let mut matched = None;
        for (mark, extra) in [
            ("`", Modifier::empty()),
            ("**", Modifier::BOLD),
            ("__", Modifier::BOLD),
            ("*", Modifier::ITALIC),
            ("_", Modifier::ITALIC),
        ] {
            if !rest.starts_with(mark) {
                continue;
            }
            if mark.starts_with('_')
                && value[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            if let Some(close) = rest[mark.len()..].find(mark) {
                let end = mark.len() + close;
                let inner = &rest[mark.len()..end];
                let tail = &rest[end + mark.len()..];
                if !inner.is_empty()
                    && (mark == "`" || inner.trim() == inner)
                    && !(mark.starts_with('_')
                        && tail
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_'))
                {
                    matched = Some((mark, extra, inner, end + mark.len()));
                    break;
                }
            }
        }
        if let Some((mark, extra, inner, consumed)) = matched {
            plain(&value[literal..i], tone, modifier, spans);
            if mark == "`" {
                push_span(
                    spans,
                    inner,
                    Style::default()
                        .fg(tone.code)
                        .add_modifier(modifier | Modifier::BOLD),
                );
            } else {
                inline_inner(inner, tone, modifier | extra, depth + 1, spans);
            }
            i += consumed;
            literal = i;
        } else {
            i += rest.chars().next().unwrap().len_utf8();
        }
    }
    plain(&value[literal..], tone, modifier, spans);
}

fn code_line(value: &str, lang: &str, tone: Tone) -> Line<'static> {
    let keywords: &[&str] = match lang {
        "rust" | "rs" => &[
            "fn", "let", "mut", "pub", "struct", "enum", "impl", "match", "if", "else", "use",
            "mod", "return", "Some", "None",
        ],
        "python" | "py" => &[
            "def", "class", "if", "else", "elif", "return", "import", "from", "for", "in", "with",
            "None", "True", "False",
        ],
        "sh" | "bash" | "fish" => &[
            "if", "then", "else", "fi", "for", "in", "do", "done", "set", "function", "end",
        ],
        "json" => &["true", "false", "null"],
        _ => &[],
    };
    let mut spans = vec![Span::raw("    ")];
    let mut rest = value;
    while !rest.is_empty() {
        let first = rest.chars().next().unwrap();
        if first == '"' || (first == '\'' && lang != "json") {
            let mut end = first.len_utf8();
            let mut escaped = false;
            for (i, ch) in rest[end..].char_indices() {
                end = first.len_utf8() + i + ch.len_utf8();
                if ch == first && !escaped {
                    break;
                }
                escaped = ch == '\\' && !escaped;
                if ch != '\\' {
                    escaped = false;
                }
            }
            let string = &rest[..end];
            let after = rest[end..].trim_start();
            let color = if lang == "json" && after.starts_with(':') {
                tone.accent
            } else {
                tone.success
            };
            spans.push(Span::styled(string.to_owned(), Style::default().fg(color)));
            rest = &rest[end..];
        } else if (first == '#' && lang != "rust" && lang != "json")
            || (rest.starts_with("//") && lang == "rust")
        {
            spans.push(Span::styled(
                rest.to_owned(),
                Style::default().fg(tone.muted),
            ));
            break;
        } else if first.is_alphabetic() || first == '_' || first.is_ascii_digit() {
            let end = rest
                .char_indices()
                .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                .map_or(rest.len(), |(i, _)| i);
            let word = &rest[..end];
            let color = if keywords.contains(&word) {
                tone.option
            } else if first.is_ascii_digit() {
                tone.code
            } else {
                tone.text
            };
            spans.push(Span::styled(word.to_owned(), Style::default().fg(color)));
            rest = &rest[end..];
        } else {
            spans.push(Span::raw(first.to_string()));
            rest = &rest[first.len_utf8()..];
        }
    }
    Line::from(spans)
}

pub fn markdown_lines(value: &str, max_width: u16, tone: Tone, user: bool) -> Vec<Line<'static>> {
    let max_width = max_width.max(8) as usize;
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    for raw in value.lines() {
        if let Some(lang) = raw.trim_start().strip_prefix("```") {
            if fence.is_some() {
                fence = None;
            } else {
                let label = lang.trim().to_ascii_lowercase();
                out.push(Line::styled(
                    format!(
                        "  Code · {}",
                        if label.is_empty() { "plain" } else { &label }
                    ),
                    Style::default().fg(tone.muted).add_modifier(Modifier::BOLD),
                ));
                fence = Some(label);
            }
            continue;
        }
        if let Some(lang) = &fence {
            let available = max_width.saturating_sub(4).max(1);
            for part in split_long_word(raw, available) {
                out.push(code_line(&part, lang, tone));
            }
            if raw.is_empty() {
                out.push(Line::from("    "));
            }
            continue;
        }
        if raw.is_empty() {
            out.push(Line::from(""));
            continue;
        }
        let trimmed = raw.trim_start();
        let indent = raw.len() - trimmed.len();
        let (prefix, content, heading, option) = if let Some(content) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            (
                format!("{}• ", " ".repeat(indent + 2)),
                content,
                false,
                false,
            )
        } else if let Some((number, content)) = trimmed.split_once(". ") {
            if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
                (
                    format!("{}{}. ", " ".repeat(indent + 2), number),
                    content,
                    false,
                    true,
                )
            } else {
                ("  ".to_owned(), raw, false, false)
            }
        } else if trimmed.starts_with('#') {
            (
                "  ".to_owned(),
                trimmed.trim_start_matches('#').trim_start(),
                true,
                false,
            )
        } else {
            (format!("{}", " ".repeat(indent + 2)), trimmed, false, false)
        };
        let continuation = " ".repeat(width(&prefix));
        for line in wrap_spans(
            inline(content, tone, user),
            &prefix,
            &continuation,
            max_width,
        ) {
            let style = if heading {
                Style::default()
                    .fg(tone.accent)
                    .add_modifier(Modifier::BOLD)
            } else if option {
                Style::default()
                    .fg(tone.option)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            out.push(line.style(style));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rendered(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn saved_native_plan_renders_strong_without_markers_at_narrow_and_wide_widths() {
        let reply = include_str!("../../../tests/fixtures/claude-conversation/saved-plan.md");
        for theme in ["dark", "light", "mono"] {
            for width in [24, 80, 160] {
                let tone = Tone::for_theme(theme);
                let lines = markdown_lines(reply, width, tone, false);
                let text = rendered(&lines);
                assert!(
                    !text.contains("**"),
                    "literal strong marker at width {width}"
                );
                assert!(text.contains("Goal:"));
                assert!(
                    lines
                        .iter()
                        .flat_map(|l| &l.spans)
                        .any(|s| s.content.contains("Goal:")
                            && s.style.add_modifier.contains(Modifier::BOLD))
                );
                assert!(lines.iter().all(|l| width_of_line(l) <= usize::from(width)));
                assert!(text.contains("println!"));
                assert!(text.contains("src/main.rs") || width == 24);
            }
        }
    }
    fn width_of_line(line: &Line<'_>) -> usize {
        line.spans.iter().map(|s| width(&s.content)).sum()
    }
    #[test]
    fn emphasis_keeps_style_across_visual_wraps_and_code_keeps_literal_markers() {
        let tone = Tone::for_theme("dark");
        let lines = markdown_lines(
            "- **several bold words across multiple lines** and *emphasis* then `**literal**`\n```rust\nlet hello_world = \"**literal**\";\n```",
            24,
            tone,
            false,
        );
        let text = rendered(&lines);
        assert!(!text.contains("**several"));
        for word in ["several", "multiple", "lines"] {
            assert!(
                lines
                    .iter()
                    .flat_map(|l| &l.spans)
                    .any(|s| s.content.contains(word)
                        && s.style.add_modifier.contains(Modifier::BOLD))
            );
        }
        assert!(lines.iter().flat_map(|l| &l.spans).any(|s| s.content == "emphasis" && s.style.add_modifier.contains(Modifier::ITALIC)));
        assert!(text.contains("**literal**"));
        assert!(lines[1].spans[0].content.starts_with("    "));
        let ids = rendered(&markdown_lines(
            "hello_world and foo_bar_baz and __strong__",
            80,
            tone,
            false,
        ));
        assert!(ids.contains("hello_world and foo_bar_baz and strong"));
    }
    #[test]
    fn unmatched_markers_and_user_text_stay_literal() {
        let tone = Tone::for_theme("dark");
        assert!(
            rendered(&markdown_lines(
                "**not completed and `unfinished",
                80,
                tone,
                false
            ))
            .contains("**not completed and `unfinished")
        );
        assert!(rendered(&markdown_lines("\\*literal\\*", 80, tone, false)).contains("*literal*"));
        let user = inline("**plain** `code` foo_bar_baz", tone, true);
        assert_eq!(user[0].content, "**plain** `code` foo_bar_baz");
        assert!(user.iter().all(|s| s.style == Style::default()));
    }
    #[test]
    fn wraps_lists_with_continuation_indent_and_keeps_code_language() {
        let tone = Tone::for_theme("dark");
        let lines = markdown_lines(
            "- An alternative with enough words to wrap cleanly\n```rust\nfn main() { let n = 2; }\n```",
            24,
            tone,
            false,
        );
        assert!(lines.len() >= 4);
        assert!(
            lines[1]
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
                .starts_with("    ")
        );
        assert!(
            lines
                .iter()
                .any(|line| line.spans.iter().any(|s| s.content.contains("rust")))
        );
        assert!(
            lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .any(|s| s.content == "fn" && s.style.fg == Some(tone.option))
        );
    }
    #[test]
    fn path_and_inline_code_styled_but_unformatted_user_text_plain() {
        let tone = Tone::for_theme("light");
        let rich = markdown_lines("See revision-2.md and `decisionId`", 80, tone, false);
        assert!(
            rich[0]
                .spans
                .iter()
                .any(|s| s.content == "revision-2.md" && s.style.fg == Some(tone.path))
        );
        assert!(
            rich[0]
                .spans
                .iter()
                .any(|s| s.content == "decisionId" && s.style.fg == Some(tone.code))
        );
        let user = markdown_lines("See revision-2.md", 80, tone, true);
        assert!(user[0].spans.iter().all(|s| s.style.fg.is_none()));
    }
    #[test]
    fn json_keys_and_numbered_choices_have_distinct_semantics_in_all_themes() {
        for theme in ["dark", "light", "mono"] {
            let tone = Tone::for_theme(theme);
            let lines = markdown_lines(
                "1. Allow none\n```json\n{\"title\":\"Decision\"}\n```",
                40,
                tone,
                false,
            );
            assert!(lines[0].style.add_modifier.contains(Modifier::BOLD));
            assert!(
                lines
                    .iter()
                    .flat_map(|line| line.spans.iter())
                    .any(|span| span.content == "\"title\"" && span.style.fg == Some(tone.accent))
            );
        }
    }
}
