# rast

A small Rust TUI launcher for your frequently-used shell commands. Think `fzf`, but focused on one job: **find a saved command by a fragment of its name and drop it into your shell prompt** so you can edit and run it yourself.

```
┌ search ─────────────────────────────────────────┐
│ ❯ ping google                                   │
└─────────────────────────────────────────────────┘
┌ 2/7 ────────────────────────────────────────────┐
│ ▶ ping -c 5 google.com   # quick connectivity   │
│   ping -c 1 8.8.8.8                             │
└─────────────────────────────────────────────────┘
enter pick · /new add (# tag) · ^e edit · ^d del · esc quit
```

## Features

- **Per-directory storage.** Commands live in `./.rast` — the file in the directory you launched from. Each project gets its own set.
- **Searchable `#` tags.** Save `/new docker compose up -d # start stack` — you can later find it by typing `start`. The `# tag` portion is stripped before the command is handed back to your shell.
- **Inline edit:** `^e` loads the highlighted command into the input; tweak it and press `Enter` to overwrite.
- **No direct execution.** The picked command is dropped into your shell's prompt buffer via `print -z` (zsh) so you review/edit before pressing Enter yourself.
- **Single ~700 KB binary**, no runtime.

## Install

### One-shot

```bash
git clone https://github.com/<your-username>/rast-tui.git
cd rast-tui
./install.sh
source ~/.zshrc
```

The installer:
1. runs `cargo build --release`
2. installs the binary to `~/.local/bin/rast`
3. appends the right wrapper to your shell rc (zsh and/or bash) — skipped if already present

After installing:

- **zsh** — `source ~/.zshrc`, then just run `rast`. The picked command lands in the next prompt via `print -z`.
- **bash** — `source ~/.bashrc`, then press `Ctrl+G`. The widget calls `rast` and sets `READLINE_LINE` so the picked command appears at the cursor.

### Manual

```bash
cargo build --release
install -m755 target/release/rast ~/.local/bin/rast
```

For zsh, add to `~/.zshrc`:

```zsh
rast() {
  local cmd
  cmd="$(command rast)" || return
  [[ -n "$cmd" ]] && print -z -- "$cmd"
}
```

For bash, add to `~/.bashrc`:

```bash
_rast_widget() {
  local cmd
  cmd="$(command rast </dev/tty)" || return
  if [ -n "$cmd" ]; then
    READLINE_LINE="$cmd"
    READLINE_POINT=${#READLINE_LINE}
  fi
}
bind -x '"\C-g": _rast_widget'
```

Re-source the rc file.

## Usage

Launch `rast` in any directory; the TUI opens.

| Action | How |
|---|---|
| Search | just type — space-separated tokens, case-insensitive, all must match |
| Add a command | `/new <command>` then `Enter` |
| Add with tags | `/new <command> # <tags>` |
| Pick | `Enter` — command lands in your shell prompt for review |
| Edit selected | `^e` to load, change it, `Enter` to save |
| Delete selected | `^d` |
| Quit | `Esc` or `^c` |

### List navigation

| Key | Action |
|---|---|
| `↑` / `^p` | up |
| `↓` / `^n` | down |

### Input editing (readline-style)

| Key | Action |
|---|---|
| `←` / `^b` | cursor left |
| `→` / `^f` | cursor right |
| `Home` / `^a` | start |
| `End` | end |
| `Backspace` | delete char left |
| `Delete` | delete char right |
| `^w` | delete word left |
| `^u` | clear input |
| `^k` | kill to end of line |

## Example session

```text
$ cd ~/myproject
$ rast
  > /new docker compose up -d # start stack
  > /new docker compose logs -f api # api logs
  > /new docker compose down # stop
  esc

$ rast            # reopen — same set is available
  > start
  ▶ docker compose up -d   # start stack
  enter
$ docker compose up -d█    ← already in the prompt, you press Enter yourself
```

`cd ~/other-project && rast` shows a different (empty) set because each directory has its own `./.rast`.

## How it works

- The TUI renders directly to `/dev/tty`, so `stdout` stays clean.
- When you pick a command, the binary prints it to `stdout` (without the `# tag`).
- The shell wrapper captures that output with `$(...)`:
  - **zsh** uses the built-in `print -z` to push the string into the next prompt's editor buffer.
  - **bash** uses a `bind -x` widget that sets `READLINE_LINE` directly on the active prompt.
- Direct shell injection (`TIOCSTI`) requires `CAP_SYS_ADMIN` on modern Linux kernels, so these shell-native paths are used instead — robust and no hacks.

## Dependencies

- Rust 1.88+ (uses let-chains)
- `ratatui` 0.29, `crossterm` 0.28

## License

MIT — see [LICENSE](LICENSE).
