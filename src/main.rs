use std::cmp::Reverse;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

const STORE_FILE: &str = ".rast";

fn data_file() -> PathBuf {
    let local = PathBuf::from(STORE_FILE);
    if local.exists() {
        return local;
    }
    // Fall back to ~/.rast if no local .rast file
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(STORE_FILE);
    }
    local
}

fn load_commands() -> Vec<String> {
    match fs::read_to_string(data_file()) {
        Ok(s) => s
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .filter(|l| !l.trim().is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn save_commands(cmds: &[String]) -> io::Result<()> {
    let body = if cmds.is_empty() {
        String::new()
    } else {
        let mut s = cmds.join("\n");
        s.push('\n');
        s
    };
    fs::write(data_file(), body)
}

fn push_command(cmds: &mut Vec<String>, cmd: &str) -> io::Result<()> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Ok(());
    }
    if let Some(pos) = cmds.iter().position(|c| c == cmd) {
        let existing = cmds.remove(pos);
        cmds.push(existing);
    } else {
        cmds.push(cmd.to_string());
    }
    save_commands(cmds)
}

/// Split `line` into (command_part, comment_part). The comment part keeps its leading `#`.
/// A `#` is treated as a comment when preceded by whitespace or at the start of the line.
fn split_command_comment(line: &str) -> (&str, Option<&str>) {
    let bytes = line.as_bytes();
    if !bytes.is_empty() && bytes[0] == b'#' {
        return ("", Some(line));
    }
    let mut i = 0;
    while i + 1 < bytes.len() {
        if (bytes[i] == b' ' || bytes[i] == b'\t') && bytes[i + 1] == b'#' {
            return (line[..i].trim_end(), Some(&line[i + 1..]));
        }
        i += 1;
    }
    (line, None)
}

/// Score how well `item` matches `query`. The full stored line — including any `#`-comment —
/// is searched, so tags after `#` are usable patterns.
fn fuzzy_score(query: &str, item: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }
    let item_lower = item.to_lowercase();
    let mut score: i64 = 0;
    for token in query.split_whitespace() {
        let t = token.to_lowercase();
        let pos = item_lower.find(&t)?;
        score += 1000 - (pos as i64).min(999);
        score += (t.len() as i64) * 10;
        if item_lower == t {
            score += 5000;
        }
    }
    score -= item.len() as i64;
    Some(score)
}

fn filter_commands(cmds: &[String], query: &str) -> Vec<usize> {
    let mut scored: Vec<(i64, usize)> = cmds
        .iter()
        .enumerate()
        .filter_map(|(i, c)| fuzzy_score(query, c).map(|s| (s, i)))
        .collect();
    if query.is_empty() {
        scored.sort_by_key(|(_, idx)| Reverse(*idx));
    } else {
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    }
    scored.into_iter().map(|(_, i)| i).collect()
}

#[derive(Clone, Copy)]
enum InputMode {
    Normal,
    Editing(usize),
}

enum NormalParse<'a> {
    Search(&'a str),
    Add(&'a str),
    AddEmpty,
}

fn parse_normal(q: &str) -> NormalParse<'_> {
    if let Some(rest) = q.strip_prefix("/new ") {
        let r = rest.trim_start();
        if r.is_empty() {
            NormalParse::AddEmpty
        } else {
            NormalParse::Add(r)
        }
    } else if q == "/new" {
        NormalParse::AddEmpty
    } else {
        NormalParse::Search(q)
    }
}

enum TuiResult {
    Pick(String),
    Cancel,
}

fn prev_char_boundary(s: &str, i: usize) -> usize {
    if i == 0 {
        return 0;
    }
    let mut j = i - 1;
    while j > 0 && !s.is_char_boundary(j) {
        j -= 1;
    }
    j
}

fn next_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

fn run_tui() -> io::Result<TuiResult> {
    let mut commands = load_commands();

    // Backend writes (frames) go to /dev/tty. We keep a second tty handle for
    // setup/teardown sequences so stdout stays untouched — `$(rast)` captures
    // only the chosen command we explicitly println at the end.
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                "interactive terminal required: could not open /dev/tty",
            )
        })?;
    let mut setup = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                "interactive terminal required: could not open /dev/tty",
            )
        })?;

    enable_raw_mode()?;
    if let Err(e) = execute!(setup, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(e);
    }

    let backend = CrosstermBackend::new(tty);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = execute!(setup, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(e);
        }
    };

    let mut query = String::new();
    let mut cursor: usize = 0;
    let mut selected: usize = 0;
    let mut mode = InputMode::Normal;
    let mut status: Option<(String, Color)> = None;

    let result = loop {
        let filter_str = match mode {
            InputMode::Normal => match parse_normal(&query) {
                NormalParse::Search(s) => s.to_string(),
                _ => String::new(),
            },
            InputMode::Editing(_) => String::new(),
        };
        let matches: Vec<usize> = filter_commands(&commands, &filter_str);
        if matches.is_empty() {
            selected = 0;
        } else if selected >= matches.len() {
            selected = matches.len() - 1;
        }

        let total = commands.len();
        let shown = matches.len();
        let cmds_view = &commands;
        let matches_view = &matches;
        let query_view = &query;
        let cursor_view = cursor;
        let selected_view = selected;
        let status_view = status.clone();
        let mode_view = mode;

        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);

            let (prompt_sym, prompt_color, prompt_title) = match mode_view {
                InputMode::Editing(_) => ("✎ ", Color::Yellow, " edit "),
                InputMode::Normal => match parse_normal(query_view) {
                    NormalParse::Add(_) | NormalParse::AddEmpty => ("+ ", Color::Green, " add "),
                    NormalParse::Search(_) => ("❯ ", Color::Cyan, " search "),
                },
            };

            // Visible terminal cursor inside the input box.
            let prompt_cells: u16 = 2;
            let chars_before = query_view[..cursor_view].chars().count() as u16;
            let inner_w = chunks[0].width.saturating_sub(2);
            let text_x = prompt_cells + chars_before;

            // Horizontal scroll: keep cursor visible when text exceeds input width
            let scroll_offset = if inner_w > 0 && text_x >= inner_w {
                text_x - inner_w + 1
            } else {
                0
            };

            let input = Paragraph::new(Line::from(vec![
                Span::styled(
                    prompt_sym,
                    Style::default()
                        .fg(prompt_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(query_view.as_str()),
            ]))
            .scroll((0, scroll_offset))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        prompt_title,
                        Style::default()
                            .fg(prompt_color)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(input, chunks[0]);

            let cx = chunks[0].x + 1 + (text_x - scroll_offset);
            let cy = chunks[0].y + 1;
            f.set_cursor_position((cx, cy));

            let list_h = chunks[1].height.saturating_sub(2) as usize;
            let offset = if selected_view >= list_h {
                selected_view + 1 - list_h
            } else {
                0
            };

            let items: Vec<ListItem> = matches_view
                .iter()
                .enumerate()
                .skip(offset)
                .take(list_h.max(1))
                .map(|(i, &idx)| {
                    let line_str = &cmds_view[idx];
                    let (cmd_part, comment_part) = split_command_comment(line_str);
                    let is_sel = i == selected_view;
                    let (marker, base_style) = if is_sel {
                        (
                            "▶ ",
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ("  ", Style::default().fg(Color::Gray))
                    };
                    let mut spans = vec![
                        Span::styled(marker, base_style),
                        Span::styled(cmd_part.to_string(), base_style),
                    ];
                    if let Some(c) = comment_part {
                        let comment_style = if is_sel {
                            Style::default()
                                .fg(Color::DarkGray)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::ITALIC)
                        } else {
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC)
                        };
                        spans.push(Span::styled("  ", base_style));
                        spans.push(Span::styled(c.to_string(), comment_style));
                    }
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let title = format!(" {}/{} ", shown, total);
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )),
            );
            f.render_widget(list, chunks[1]);

            let help_line = if let Some((msg, color)) = status_view {
                Line::from(Span::styled(msg, Style::default().fg(color)))
            } else {
                match mode_view {
                    InputMode::Editing(_) => Line::from(vec![
                        Span::styled("enter", Style::default().fg(Color::Yellow)),
                        Span::raw(" save  "),
                        Span::styled("esc", Style::default().fg(Color::DarkGray)),
                        Span::raw(" cancel edit"),
                    ]),
                    InputMode::Normal => Line::from(vec![
                        Span::styled("enter", Style::default().fg(Color::Green)),
                        Span::raw(" pick  "),
                        Span::styled("/new", Style::default().fg(Color::Green)),
                        Span::raw(" add (# tag)  "),
                        Span::styled("^e", Style::default().fg(Color::Yellow)),
                        Span::raw(" edit  "),
                        Span::styled("^d", Style::default().fg(Color::Red)),
                        Span::raw(" del  "),
                        Span::styled("esc", Style::default().fg(Color::DarkGray)),
                        Span::raw(" quit"),
                    ]),
                }
            };
            let help = Paragraph::new(help_line).style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[2]);
        })?;

        // one-shot status
        status = None;

        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = event::read()?
        else {
            continue;
        };
        if kind != KeyEventKind::Press {
            continue;
        }

        match (code, modifiers) {
            (KeyCode::Esc, _) => match mode {
                InputMode::Editing(_) => {
                    mode = InputMode::Normal;
                    query.clear();
                    cursor = 0;
                    selected = 0;
                    status = Some(("edit cancelled".to_string(), Color::DarkGray));
                }
                InputMode::Normal => break TuiResult::Cancel,
            },
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => break TuiResult::Cancel,

            (KeyCode::Enter, _) => match mode {
                InputMode::Editing(orig) => {
                    let new_text = query.trim().to_string();
                    if new_text.is_empty() {
                        status = Some(("cannot save empty entry".to_string(), Color::Yellow));
                    } else {
                        if orig < commands.len() {
                            commands[orig] = new_text.clone();
                            let _ = save_commands(&commands);
                            status = Some((format!("updated: {}", new_text), Color::Green));
                        }
                        mode = InputMode::Normal;
                        query.clear();
                        cursor = 0;
                        selected = 0;
                    }
                }
                InputMode::Normal => match parse_normal(&query) {
                    NormalParse::Add(cmd) => {
                        let to_save = cmd.to_string();
                        match push_command(&mut commands, &to_save) {
                            Ok(_) => {
                                status = Some((format!("saved: {}", to_save), Color::Green));
                                query.clear();
                                cursor = 0;
                                selected = 0;
                            }
                            Err(e) => status = Some((format!("save failed: {}", e), Color::Red)),
                        }
                    }
                    NormalParse::AddEmpty => {
                        status = Some((
                            "type a command after /new to save".to_string(),
                            Color::Yellow,
                        ));
                    }
                    NormalParse::Search(_) => {
                        if let Some(&idx) = matches.get(selected) {
                            break TuiResult::Pick(commands[idx].clone());
                        }
                    }
                },
            },

            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                if matches!(mode, InputMode::Normal)
                    && matches!(parse_normal(&query), NormalParse::Search(_))
                    && let Some(&idx) = matches.get(selected)
                {
                    let text = commands[idx].clone();
                    cursor = text.len();
                    query = text;
                    mode = InputMode::Editing(idx);
                }
            }

            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                if matches!(mode, InputMode::Normal)
                    && matches!(parse_normal(&query), NormalParse::Search(_))
                    && let Some(&idx) = matches.get(selected)
                {
                    let removed = commands.remove(idx);
                    let _ = save_commands(&commands);
                    status = Some((format!("deleted: {}", removed), Color::Red));
                }
            }

            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                selected = selected.saturating_sub(1);
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                if selected + 1 < matches.len() {
                    selected += 1;
                }
            }

            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                cursor = prev_char_boundary(&query, cursor);
            }
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                cursor = next_char_boundary(&query, cursor);
            }
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                cursor = 0;
            }
            (KeyCode::End, _) => {
                cursor = query.len();
            }

            (KeyCode::Backspace, _) => {
                if cursor > 0 {
                    let p = prev_char_boundary(&query, cursor);
                    query.replace_range(p..cursor, "");
                    cursor = p;
                    selected = 0;
                }
            }
            (KeyCode::Delete, _) => {
                if cursor < query.len() {
                    let n = next_char_boundary(&query, cursor);
                    query.replace_range(cursor..n, "");
                    selected = 0;
                }
            }

            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                query.clear();
                cursor = 0;
                selected = 0;
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                query.truncate(cursor);
                selected = 0;
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                let mut s = cursor;
                while s > 0 {
                    let p = prev_char_boundary(&query, s);
                    if query[p..s].starts_with(char::is_whitespace) {
                        s = p;
                    } else {
                        break;
                    }
                }
                while s > 0 {
                    let p = prev_char_boundary(&query, s);
                    if query[p..s].starts_with(char::is_whitespace) {
                        break;
                    } else {
                        s = p;
                    }
                }
                query.replace_range(s..cursor, "");
                cursor = s;
                selected = 0;
            }

            (KeyCode::Char(c), m)
                if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) =>
            {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                query.insert_str(cursor, s);
                cursor += s.len();
                selected = 0;
            }
            _ => {}
        }
    };

    let _ = disable_raw_mode();
    let _ = execute!(setup, LeaveAlternateScreen);
    Ok(result)
}

fn main() {
    match run_tui() {
        Ok(TuiResult::Pick(line)) => {
            // Strip the optional "# tag" portion before handing to the shell —
            // the comment is metadata for search, not for execution.
            let (cmd_part, _) = split_command_comment(&line);
            let _ = writeln!(io::stdout(), "{}", cmd_part);
        }
        Ok(TuiResult::Cancel) => {}
        Err(e) => {
            eprintln!("rast: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_command_comment_keeps_command_and_tag() {
        assert_eq!(
            split_command_comment("docker compose up -d # start stack"),
            ("docker compose up -d", Some("# start stack"))
        );
        assert_eq!(
            split_command_comment("echo foo#bar"),
            ("echo foo#bar", None)
        );
        assert_eq!(
            split_command_comment("# only metadata"),
            ("", Some("# only metadata"))
        );
    }

    #[test]
    fn fuzzy_score_requires_all_tokens_case_insensitively() {
        assert!(fuzzy_score("docker api", "docker compose logs -f api # logs").is_some());
        assert!(fuzzy_score("DOCKER", "docker compose up").is_some());
        assert!(fuzzy_score("missing", "docker compose up").is_none());
    }

    #[test]
    fn filter_empty_query_prefers_recent_commands() {
        let commands = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        assert_eq!(filter_commands(&commands, ""), vec![2, 1, 0]);
    }

    #[test]
    fn char_boundaries_handle_multibyte_text() {
        let text = "aλ🚀";
        let after_lambda = next_char_boundary(text, 1);
        assert_eq!(&text[1..after_lambda], "λ");
        assert_eq!(prev_char_boundary(text, text.len()), after_lambda);
    }
}
