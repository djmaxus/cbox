# cbox

<!-- TODO: Add vhs/asciinema GIF here showing 1. Launch 2. Search 3. Prompt injection 4. Typing flag and Enter -->

`cbox` (Command Box) is a tiny Rust TUI launcher for saving, fuzzy-finding, and injecting project-local shell commands.

## How is this different from fzf / zoxide?
**fzf** searches files and history. **zoxide** navigates directories. 
**`cbox`** saves structured, project-specific workflows and safely injects them into your prompt without executing them blindly.

## Installation

### 1. Pre-built Binaries (Recommended)
You can install `cbox` using our one-liner install script which fetches the correct binary for your OS and architecture:
```bash
curl -sSfL https://raw.githubusercontent.com/mukh4w/cbox/main/install.sh | sh
```

### 2. cargo binstall
If you have `cargo binstall` installed:
```bash
cargo binstall cbox
```

### 3. Build from source
```bash
cargo install --git https://github.com/mukh4w/cbox
```

## Shell Integration

To make `cbox` inject commands directly into your shell prompt and map it to `Ctrl+G`, add the corresponding line to your shell configuration file:

**Bash** (`~/.bashrc`):
```bash
eval "$(cbox init bash)"
```

**Zsh** (`~/.zshrc`):
```bash
eval "$(cbox init zsh)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
cbox init fish | source
```

**Nushell** (`config.nu`):
Run `cbox init nu` to see the manual instructions to add to your Nu config.

## Usage

- **Save local command**: Type `/new docker compose up -d`
- **Save global command**: Type `/newg ping 8.8.8.8`
- **Pick**: Press `Enter` to inject the command into your prompt.
- **Fast Execute**: Press `Ctrl+x` to run immediately without injecting.
- **Placeholders**: You can use `{...}` or `<...>` in your commands (e.g. `git commit -m "{message}"`). `cbox` will automatically place your cursor at the placeholder upon injection!
- **Edit/Delete**: Press `Ctrl+e` to edit, `Ctrl+d` to delete.

## Tips & Tricks

### Floating Tmux Popup
If you use `tmux`, you can launch `cbox` in a beautiful floating popup window rather than occupying your terminal.
Add this to your `~/.tmux.conf`:
```tmux
bind C-g popup -E -w 80% -h 80% "cbox"
```
*(Note: Prompt injection requires running `cbox` directly in your shell, so the tmux popup is best suited for when you use `Ctrl+x` execution).*

## Packages (Maintainers)

### Homebrew (macOS / Linux)
We plan to provide a Homebrew tap. A template formula is available in the repository.

### Arch User Repository (AUR)
`cbox` will be available in the AUR as `cbox-bin` and `cbox-git`.

## License
MIT
