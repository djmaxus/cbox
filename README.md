<div align="center">
  <h1>📦 cbox (Command Box)</h1>
  <p>A tiny, lightning-fast Rust TUI for saving, fuzzy-finding, and safely injecting project-local shell commands.</p>

  <a href="https://crates.io/crates/cbox-tui"><img src="https://img.shields.io/crates/v/cbox-tui.svg" alt="Crates.io" /></a>
  <a href="https://github.com/mukh4w/cbox/actions/workflows/release.yml"><img src="https://github.com/mukh4w/cbox/actions/workflows/release.yml/badge.svg" alt="CI Status" /></a>
  <a href="https://crates.io/crates/cbox-tui"><img src="https://img.shields.io/crates/d/cbox-tui.svg" alt="Downloads" /></a>
  <a href="https://github.com/mukh4w/cbox/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/cbox-tui.svg" alt="License" /></a>
</div>

<br />

<div align="center">
  <!-- Replace this with a real GIF of cbox in action -->
  <img src="https://raw.githubusercontent.com/mukh4w/cbox/main/demo.gif" alt="cbox demo" width="700" />
</div>

## 💡 How is this different from fzf / zoxide?
- **fzf** searches files and terminal history.
- **zoxide** navigates directories. 
- **`cbox`** saves structured, project-specific workflows and **safely injects** them into your prompt without executing them blindly.

## 🚀 Installation

### 1. Pre-built Binaries (Recommended)
You can install `cbox` using our one-liner install script which fetches the correct binary for your OS and architecture:
```bash
curl -sSfL https://raw.githubusercontent.com/mukh4w/cbox/main/install.sh | sh
```

### 2. cargo binstall
If you have `cargo binstall` installed:
```bash
cargo binstall cbox-tui
```

### 3. Build from source
```bash
cargo install cbox-tui
```

## 🔌 Shell Integration

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

*(Restart your terminal or run `source ~/.zshrc` etc. after adding)*

## 🎮 Usage

Once integrated, press **`Ctrl+G`** to open `cbox`.

- **Save local command**: Type `/new docker compose up -d` (saved to `./.cbox`)
- **Save global command**: Type `/newg ping 8.8.8.8` (saved to `~/.config/cbox/commands`)
- **Fuzzy Search**: Just start typing!
- **Inject (Default)**: Press `Enter` to inject the command into your prompt safely.
- **Fast Execute**: Press `Alt+Enter` to run the command immediately without injecting.
- **Edit/Delete**: Press `Ctrl+e` to edit the selected command, `Ctrl+d` to delete it.

## 🛠️ Tips & Tricks

### Floating Tmux Popup
If you use `tmux`, you can launch `cbox` in a beautiful floating popup window rather than occupying your terminal.
Add this to your `~/.tmux.conf`:
```tmux
bind C-g popup -E -w 80% -h 80% "cbox"
```
*(Note: Prompt injection requires running `cbox` directly in your shell via the integration. The tmux popup is best suited for when you exclusively use `Alt+Enter` execution).*

## 📦 Packages (Maintainers)

### Homebrew (macOS / Linux)
We plan to provide a Homebrew tap. A template formula is available in the repository.

### Arch User Repository (AUR)
`cbox` will be available in the AUR as `cbox-bin` and `cbox-git`.

## 📜 License
MIT License.
