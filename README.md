# NiNi

A fast AI chat client that runs entirely in your terminal. Built with Rust and [ratatui](https://github.com/ratatui/ratatui), it talks to OpenAI-compatible providers (OpenRouter and OpenCode Zen out of the box) and can read, search, edit and run things in your project through tool calls.

> Named after Ningning (aespa), because why not.

## Features

- **Terminal-native TUI**: no browser, no Electron.
- **Multiple providers**: switch between OpenRouter (free models) and OpenCode Zen with `/models`. Providers are tried in a fallback order when one fails.
- **Tools the AI can call**: `read_file`, `write_file`, `edit_file`, `list_directory`, `grep`, `run_command`, `spawn_subagent`, and the LSP-backed `find_symbol` and `goto_definition`.
- **LSP integration**: the AI can ask a real language server (for example rust-analyzer) where a symbol is defined, instead of guessing with text search.
- **Subagents**: the main agent can delegate tasks to subagents that run with an isolated context (they do not see the main conversation), which keeps token usage down.
- **Skills**: reusable instruction files (commit messages, PR review, code explanation) that you can invoke with `/skill` or let the AI use on its own.
- **Multiple sessions**: open a new chat with `Ctrl+N` and jump between them with `Alt+1` to `Alt+9`.
- **Markdown rendering**: headers, lists, bold, italics, inline code and code blocks.
- **Live theme editor**: `Ctrl+T`, with a real-time RGB preview.
- **Mouse support**: scroll the history with the wheel, copy the last answer with the `[Copy]` button or `F2`.

## Safety model

- File tools (`read_file`, `write_file`, `edit_file`, ...) are restricted to the directory where you launched NiNi. Paths that escape it (`..`, absolute paths elsewhere, symlinks) are rejected.
- `run_command` is **not** sandboxed. It runs through `sh -c` in the current directory, and the only protection is the confirmation popup that appears before each command. Read the command before accepting.
- File writes and edits also ask for confirmation.

## Installation

Requires a recent Rust toolchain (the project uses Rust edition 2024, so Rust 1.85 or newer).

```bash
git clone https://github.com/RealGago/NiNi.git
cd NiNi
cargo build --release
```

The binary is `target/release/NiNi3`. Run it from the project you want the AI to work on, since file access is limited to the current directory.

## API keys

NiNi needs at least one key:

| Provider | Environment variable |
| --- | --- |
| OpenRouter (free models) | `OPENROUTER_API_KEY` |
| OpenCode Zen | `OPENCODE_API_KEY` |

Either copy the example file and edit it:

```bash
cp .env.example .env
```

or just launch NiNi. If no keys are found, the splash screen asks for them (leave a field empty and press Enter to skip it).

## Usage

```bash
./target/release/NiNi3
# or, during development
cargo run --release
```

### Keyboard shortcuts

| Key | Action |
| --- | --- |
| `Enter` | Send message / confirm |
| `Esc` | Quit / cancel |
| `Tab` | Autocomplete command or model name |
| `F2` | Copy last AI response to the clipboard |
| `Ctrl+T` | Open the theme editor |
| `Ctrl+N` | New chat session |
| `Alt+1` ... `Alt+9` | Go to session 1 to 9 |
| `PageUp` / `PageDown`, mouse wheel | Scroll the chat |
| `Up` / `Down` | Navigate popups |

### Slash commands

| Command | Description |
| --- | --- |
| `/models` | Pick a provider, then a model |
| `/model <name>` | Switch directly to a model |
| `/system` | Edit the system prompt |
| `/skills` | List available skills |
| `/skill <name>` | Invoke a skill |
| `/clear` | Clear the conversation history |
| `/exit` | Quit |

## LSP setup

LSP support is opt-in: NiNi does not ship with any language server configured. Create `~/.config/nini/lsp.toml` (global) or `.nini/lsp.toml` (per project, overrides the global one):

```toml
[lsp.rust]
command = "rust-analyzer"
extensions = [".rs"]
root_markers = ["Cargo.toml"]

# optional
# args = []
# env = { RA_LOG = "error" }
```

- `extensions` must include the leading dot.
- `root_markers` are used to find the project root for the language server.
- The server is started lazily, the first time a tool needs it for a matching file.

Once configured, the AI gets two extra tools:

- `find_symbol`: find where a function, struct or other symbol is defined, by name, anywhere in the project.
- `goto_definition`: from a known file and line where a symbol is used, jump to its declaration.

## Skills

A skill is a folder under `skills/` (in the project) or `~/.config/nini/skills/` (global) containing a `SKILL.md`:

```
skills/
  commit/
    SKILL.md
```

```markdown
---
name: commit
description: Creates commits following Conventional Commits
user_invocable: true
disable_model_invocation: false
---

# Commit

Instructions for the AI go here.
```

| Field | Meaning |
| --- | --- |
| `name`, `description` | Required. |
| `user_invocable` | The skill appears in `/skills` and can be run with `/skill <name>`. |
| `disable_model_invocation` | If `true`, only you can trigger it, the AI will not use it on its own. |

Project skills take priority over global skills with the same name.

## Subagents

For complex tasks the main agent can spawn subagents (code analysis, file operations, command execution) and merge their results back into its answer. Subagents have an isolated context on purpose, to reduce token consumption.

## Theme

Press `Ctrl+T` to edit every accent color with a live RGB preview. Colors are saved to `theme.toml` in the directory where you run NiNi and loaded on every launch. The file is git-ignored.

| Key | Action |
| --- | --- |
| `Up` / `Down` | Select the color to edit |
| `Tab` | Switch between R / G / B |
| `0`-`9` | Type a value (0-255) |
| `Enter` | Confirm the value |
| `R` | Reset all colors to default |
| `Esc` | Close the editor |

## Adding a provider

Providers are data, not code. Add one entry to `PROVIDERS` in `src/agent/providers.rs` with its base URL, the environment variable for its key, and its fallback priority. The provider must expose OpenAI-compatible `/models` and `/chat/completions` endpoints.

## Project layout

```
src/
  agent/    API client, message models, providers
  lsp/      LSP client, manager, config
  skills/   skill loader
  tools/    tools the AI can call, path sandbox
  ui/       chat, popups, commands, colors, splash, layout
  app.rs    application state
  main.rs   event loop
skills/     example skills
```

## Contributing

Not accepting pull requests at this time, this is a personal hobby project. Bug reports and suggestions are welcome as [issues](https://github.com/RealGago/NiNi/issues).

## Disclaimer

NiNi is an independent, unaffiliated hobby project. Your prompts and any file contents the AI reads are sent to the provider you choose, so do not use it on anything you would not want to send to that provider. You are responsible for what you send to the models and for the commands you approve.
