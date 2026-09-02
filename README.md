# NiNi

A fast,AI chat client that runs entirely in your terminal (TUI). Built with Rust and [ratatui](https://ratatui.rs), it connects to [OpenRouter](https://openrouter.ai) and [OpenCode Zen](https://opencode.ai) so you can chat with AI models without ever leaving the command line.

> *Named after Ningning (aespa), because why not.*

## Features

- **Terminal-native chat** — smooth, responsive TUI with no browser or Electron overhead
- **Multiple providers** — switch between OpenRouter (free models) and OpenCode Zen on the fly with `/models`
- **Markdown rendering** — bold, italics, headers, bullet lists, inline code, and code blocks render properly instead of showing raw symbols
- **Live theme editor** — press `Ctrl+T` to customize every accent color with a real-time RGB preview, saved automatically to `theme.toml`
- **Smart text input** — full cursor navigation (arrow keys, backspace) with proper UTF-8 handling for accented characters
- **Mouse support** — scroll the chat history with your mouse wheel, click the `[Copy]` button to copy the last AI response to your clipboard
- **Animated splash screen** — an animated ASCII intro handles first-run API key setup
- **Command system** — slash commands for switching models, editing the system prompt, checking usage, and clearing history

## Tech stack

| Purpose | Crate |
|---|---|
| TUI rendering | `ratatui` |
| Terminal control / input | `crossterm` |
| Async runtime | `tokio` |
| HTTP client | `reqwest` |
| Clipboard | `arboard` |
| Markdown parsing | `pulldown-cmark` |
| Config (theme) | `serde` + `toml` |

## Installation

```bash
git clone https://codeberg.org/SEU_USUARIO/NiNi3.git
cd NiNi3
cargo build --release
```

The compiled binary will be at `target/release/NiNi3`.

## Setup — API keys

NiNi3 needs at least **one** of the following:

- An [OpenRouter](https://openrouter.ai/keys) API key (gives you access to free models)
- An [OpenCode Zen](https://opencode.ai) API key

You have two ways to provide them:

**Option 1 — `.env` file**

```bash
cp .env.example .env
# then edit .env and paste your key(s) in
```

**Option 2 — first-run splash screen**

If no `.env` is found, NiNi3 will ask for your API key(s) directly the first time you launch it. Leave a field empty and press Enter to skip it.


## Usage

Run it with:

```bash
cargo run --release
```

or directly:

```bash
./target/release/NiNi3
```

### Keyboard shortcuts

| Key | Action |
|---|---|
| `Enter` | Send message / confirm |
| `Esc` | Quit / cancel |
| `Tab` | Autocomplete command or model name |
| `F2` | Copy last AI response to clipboard |
| `Ctrl+T` | Open the theme editor |
| `↑` / `↓` | Navigate popups / scroll (theme editor: change field) |
| `PageUp` / `PageDown` | Scroll chat history |
| Mouse wheel | Scroll chat history |

### Slash commands

| Command | Description |
|---|---|
| `/system` | Edit the system prompt |
| `/usage` | Check today's OpenRouter usage |
| `/clear` | Clear the conversation history |
| `/models` | Switch provider/model |
| `/model <name>` | Switch directly to a specific model |

### Theme editor

Press `Ctrl+T` to open the live theme editor:

| Key | Action |
|---|---|
| `↑` / `↓` | Select which color to edit |
| `Tab` | Switch between R / G / B channels |
| `0-9` | Type a value (0-255) |
| `Enter` | Confirm the value |
| `R` | Reset all colors to default |
| `Esc` | Close the editor |

Changes are saved automatically to `theme.toml` in the project root and reloaded on every launch.

## Disclaimer

NiNi3 is an independent, unaffiliated hobby project. Do not enter API keys or sensitive information beyond what's needed to authenticate with the providers above — you are solely responsible for everything you send to these models.
