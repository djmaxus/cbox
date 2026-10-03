use std::fs::OpenOptions;
use std::io;
use std::env;

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

mod store;
use store::{CommandEntry, Store};

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

/// Score how well `item` matches `query`.
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

fn filter_commands(entries: &[CommandEntry], query: &str) -> Vec<usize> {
    let mut scored: Vec<(i64, usize, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            fuzzy_score(query, &c.cmd).map(|s| {
                // tuple for sorting: (score, is_local, original_index)
                // is_local = 1 if local, 0 if global. Prioritize local commands.
                (s, if c.is_global { 0 } else { 1 }, i)
            })
        })
        .collect();

    if query.is_empty() {
        // Sort by local/global, then reverse index
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    } else {
        // Sort by fuzzy score, then local/global, then reverse index
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)));
    }
    scored.into_iter().map(|(_, _, i)| i).collect()
}

#[derive(Clone, Copy)]
enum InputMode {
    Normal,
    Editing(usize), // references the index in the filtered/all list
}

enum NormalParse<'a> {
    Search(&'a str),
    Add(&'a str),
    AddGlobal(&'a str),
    AddEmpty,
}

fn parse_normal(q: &str) -> NormalParse<'_> {
    if let Some(rest) = q.strip_prefix("/newg ") {
        let r = rest.trim_start();
        if r.is_empty() {
            NormalParse::AddEmpty
        } else {
            NormalParse::AddGlobal(r)
        }
    } else if let Some(rest) = q.strip_prefix("/new ") {
        let r = rest.trim_start();
        if r.is_empty() {
            NormalParse::AddEmpty
        } else {
            NormalParse::Add(r)
        }
    } else if q == "/new" || q == "/newg" {
        NormalParse::AddEmpty
    } else {
        NormalParse::Search(q)
    }
}

enum TuiResult {
    Pick { entry: CommandEntry, execute: bool },
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
    let mut store = Store::load();

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
        let entries = store.get_all();

        let filter_str = match mode {
            InputMode::Normal => match parse_normal(&query) {
                NormalParse::Search(s) => s.to_string(),
                _ => String::new(),
            },
            InputMode::Editing(_) => String::new(),
        };
        let matches: Vec<usize> = filter_commands(&entries, &filter_str);
        if matches.is_empty() {
            selected = 0;
        } else if selected >= matches.len() {
            selected = matches.len() - 1;
        }

        let total = entries.len();
        let shown = matches.len();

        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(5), // Preview pane
                    Constraint::Length(1),
                ])
                .split(area);

            let (prompt_sym, prompt_color, prompt_title) = match mode {
                InputMode::Editing(_) => ("✎ ", Color::Yellow, " edit "),
                InputMode::Normal => match parse_normal(&query) {
                    NormalParse::Add(_) | NormalParse::AddGlobal(_) | NormalParse::AddEmpty => {
                        ("+ ", Color::Green, " add ")
                    }
                    NormalParse::Search(_) => ("❯ ", Color::Cyan, " search "),
                },
            };

            let prompt_cells: u16 = 2;
            let chars_before = query[..cursor].chars().count() as u16;
            let inner_w = chunks[0].width.saturating_sub(2);
            let text_x = prompt_cells + chars_before;

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
                Span::raw(query.as_str()),
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
            let offset = if selected >= list_h {
                selected + 1 - list_h
            } else {
                0
            };

            let items: Vec<ListItem> = matches
                .iter()
                .enumerate()
                .skip(offset)
                .take(list_h.max(1))
                .map(|(i, &idx)| {
                    let entry = &entries[idx];
                    let (cmd_part, comment_part) = split_command_comment(&entry.cmd);
                    let is_sel = i == selected;

                    let (marker, base_style, tag_style) = if is_sel {
                        (
                            "▶ ",
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                            Style::default().fg(Color::DarkGray).bg(Color::Cyan),
                        )
                    } else {
                        (
                            "  ",
                            Style::default().fg(if entry.is_global {
                                Color::DarkGray
                            } else {
                                Color::White
                            }),
                            Style::default().fg(if entry.is_global {
                                Color::DarkGray
                            } else {
                                Color::Gray
                            }),
                        )
                    };

                    let mut spans = vec![
                        Span::styled(marker, base_style),
                        Span::styled(
                            if entry.is_global { "[G] " } else { "[L] " },
                            tag_style.add_modifier(Modifier::BOLD),
                        ),
                    ];
                    spans.push(Span::styled(cmd_part, base_style));

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

            // Render Preview Pane
            let preview_content = if let Some(&idx) = matches.get(selected) {
                let entry = &entries[idx];
                let tag = if entry.is_global { "[G]" } else { "[L]" };
                let tag_color = if entry.is_global { Color::DarkGray } else { Color::Gray };
                
                let mut spans = vec![
                    Span::styled(tag, Style::default().fg(tag_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                ];
                
                let (cmd_part, comment_part) = split_command_comment(&entry.cmd);
                spans.push(Span::styled(cmd_part, Style::default().fg(Color::White)));

                if let Some(c) = comment_part {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(c.to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)));
                }
                
                vec![Line::from(spans)]
            } else {
                vec![Line::from(Span::styled("No command selected", Style::default().fg(Color::DarkGray)))]
            };

            let preview = Paragraph::new(preview_content)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(Span::styled(
                            " Preview ",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        )),
                );
            f.render_widget(preview, chunks[2]);

            let help_line = if let Some((msg, color)) = &status {
                Line::from(Span::styled(msg, Style::default().fg(*color)))
            } else {
                match mode {
                    InputMode::Editing(_) => Line::from(vec![
                        Span::styled("enter", Style::default().fg(Color::Yellow)),
                        Span::raw(" save  "),
                        Span::styled("esc", Style::default().fg(Color::DarkGray)),
                        Span::raw(" cancel edit"),
                    ]),
                    InputMode::Normal => Line::from(vec![
                        Span::styled("enter", Style::default().fg(Color::Green)),
                        Span::raw(" inject  "),
                        Span::styled("^enter", Style::default().fg(Color::Green)),
                        Span::raw(" exec  "),
                        Span::styled("/new", Style::default().fg(Color::Green)),
                        Span::raw(" add local  "),
                        Span::styled("/newg", Style::default().fg(Color::Green)),
                        Span::raw(" add global  "),
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
            f.render_widget(help, chunks[3]);
        })?;

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

            (KeyCode::Enter, modifiers) => match mode {
                InputMode::Editing(idx) => {
                    let new_text = query.trim().to_string();
                    if new_text.is_empty() {
                        status = Some(("cannot save empty entry".to_string(), Color::Yellow));
                    } else {
                        let entry = &entries[idx];
                        let _ = store.update_command(
                            entry.is_global,
                            entry.original_index,
                            new_text.clone(),
                        );
                        status = Some((format!("updated: {}", new_text), Color::Green));
                        mode = InputMode::Normal;
                        query.clear();
                        cursor = 0;
                        selected = 0;
                    }
                }
                InputMode::Normal => match parse_normal(&query) {
                    NormalParse::Add(cmd) => {
                        let to_save = cmd.to_string();
                        match store.add_command(&to_save, false) {
                            Ok(_) => {
                                status = Some((format!("saved local: {}", to_save), Color::Green));
                                query.clear();
                                cursor = 0;
                                selected = 0;
                            }
                            Err(e) => status = Some((format!("save failed: {}", e), Color::Red)),
                        }
                    }
                    NormalParse::AddGlobal(cmd) => {
                        let to_save = cmd.to_string();
                        match store.add_command(&to_save, true) {
                            Ok(_) => {
                                status = Some((format!("saved global: {}", to_save), Color::Green));
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
                            let entry = entries[idx].clone();
                            let execute = modifiers.contains(KeyModifiers::CONTROL);
                            break TuiResult::Pick { entry, execute };
                        }
                    }
                },
            }

            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                if matches!(mode, InputMode::Normal)
                    && matches!(parse_normal(&query), NormalParse::Search(_))
                    && let Some(&idx) = matches.get(selected)
                {
                    let text = entries[idx].cmd.clone();
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
                    let entry = &entries[idx];
                    match store.delete_command(entry.is_global, entry.original_index) {
                        Ok(removed) => {
                            status = Some((format!("deleted: {}", removed), Color::Red));
                        }
                        Err(e) => {
                            status = Some((format!("delete failed: {}", e), Color::Red));
                        }
                    }
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

fn print_init_script(shell: &str) {
    match shell {
        "bash" => {
            println!("{}", r#"
# cbox bash integration
_cbox_widget() {
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi

  if [ $ret -eq 2 ]; then
    READLINE_LINE="$out"
    READLINE_POINT=${#READLINE_LINE}
    bind '"\e[0n": accept-line'
    printf '\e[5n'
  else
    READLINE_LINE="$out"
    READLINE_POINT=${#READLINE_LINE}
  fi
}
bind -x '"\C-g": _cbox_widget' 2>/dev/null || true

cbox() {
  if [ $# -gt 0 ]; then
    command cbox "$@"
    return
  fi
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi
  if [ $ret -eq 2 ]; then
    history -s "$out"
    eval "$out"
  else
    history -s "$out"
    bind '"\e[0n": "'"$out"'"'
    printf '\e[5n'
  fi
}
"#);
        }
        "zsh" => {
            println!("{}", r#"
# cbox zsh integration
cbox_widget() {
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    zle && zle redisplay
    return
  fi

  BUFFER="$out"
  CURSOR=${#BUFFER}
  if [ $ret -eq 2 ]; then
    zle accept-line
  fi
  zle && zle redisplay
}
zle -N cbox_widget
bindkey '^g' cbox_widget

cbox() {
  if [ $# -gt 0 ]; then
    command cbox "$@"
    return
  fi
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi
  if [ $ret -eq 2 ]; then
    print -s "$out"
    eval "$out"
  else
    print -z -- "$out"
  fi
}
"#);
        }
        "fish" => {
            println!("{}", r#"
# cbox fish integration
function _cbox_widget
    set -l out (command cbox </dev/tty)
    set -l ret $status
    if test $ret -eq 1; or test -z "$out"
        commandline -f repaint
        return
    end

    commandline -r -- $out
    commandline -C (string length -- $out)
    if test $ret -eq 2
        commandline -f execute
    end
    commandline -f repaint
end
bind \cg _cbox_widget

function cbox
    if count $argv > /dev/null
        command cbox $argv
        return
    end
    set -l out (command cbox </dev/tty)
    set -l ret $status
    if test $ret -eq 1; or test -z "$out"
        return
    end
    if test $ret -eq 2
        eval $out
    else
        echo $out
    end
end
"#);
        }
        "nushell" | "nu" => {
            println!("{}", r#"
# cbox nushell integration
# $env.config = ($env.config | upsert keybindings (
#     $env.config.keybindings | append {
#         name: cbox_widget
#         modifier: control
#         keycode: char_g
#         mode: [emacs, vi_normal, vi_insert]
#         event: {
#             send: executehostcommand
#             cmd: "
#                 let out = (^cbox < /dev/tty | complete);
#                 if $out.exit_code == 1 { return };
#                 let cmd_content = $out.stdout | str trim;
#                 if ($cmd_content | is-empty) { return };
#                 commandline edit --replace $cmd_content;
#             "
#         }
#     }
# ))
"#);
        }
        _ => {
            eprintln!("cbox: unsupported shell '{}'", shell);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && args[1] == "init" {
        print_init_script(&args[2]);
        return;
    }

    match run_tui() {
        Ok(TuiResult::Pick { entry, execute }) => {
            let (cmd_part, _) = split_command_comment(&entry.cmd);
            println!("{}", cmd_part);
            if execute {
                std::process::exit(2);
            } else {
                std::process::exit(0);
            }
        }
        Ok(TuiResult::Cancel) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("cbox: {}", e);
            std::process::exit(1);
        }
    }
}
