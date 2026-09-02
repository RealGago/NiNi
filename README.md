NiNi

A fast, AI chat client that runs entirely in your terminal (TUI). Built with Rust and ratatui, it connects to OpenRouter and OpenCode Zen so you can chat with AI models without ever leaving the command line.

    Named after Ningning (aespa), because why not.

Features

    Terminal-native chat — smooth, responsive TUI with no browser or Electron overhead

    Multiple providers — switch between OpenRouter (free models) and OpenCode Zen on the fly with /models

    Markdown rendering — bold, italics, headers, bullet lists, inline code, and code blocks render properly instead of showing raw symbols

    Live theme editor — press Ctrl+T to customize every accent color with a real-time RGB preview, saved automatically to theme.toml

    Smart text input — full cursor navigation (arrow keys, backspace) with proper UTF-8 handling for accented characters

    Mouse support — scroll the chat history with your mouse wheel, click the [Copy] button to copy the last AI response to your clipboard

    Animated splash screen — an animated ASCII intro handles first-run API key setup

    Command system — slash commands for switching models, editing the system prompt, checking usage, and clearing history

    Subagent system — main agent can autonomously delegate tasks to specialized subagents for parallel execution

    File operations — read, write, create, and edit files directly through the AI with confirmation popups

    Command execution — run system commands (e.g., cargo build, npm install) with user confirmation

Tech Stack
Purpose	Crate
TUI rendering	ratatui
Terminal control / input	crossterm
Async runtime	tokio
HTTP client	reqwest
Clipboard	arboard
Markdown parsing	pulldown-cmark
Config (theme)	serde + toml
Installation
bash

git clone [https://github.com/RealGago/NiNi.git]
cd NiNi3
cargo build --release

The compiled binary will be at target/release/NiNi3.

    Note: cargo install nini is not officially supported. Build from source.

Setup — API Keys

NiNi needs at least one of the following:

    An OpenRouter API key (gives you access to free models)

    An OpenCode Zen API key

    Any OpenAI-compatible API provider (will work, but /listmodels may be limited)

You have two ways to provide them:

Option 1 — .env file
bash

cp .env.example .env
# then edit .env and paste your key(s) in

Option 2 — first-run splash screen

If no .env is found, NiNi will ask for your API key(s) directly the first time you launch it. Leave a field empty and press Enter to skip it.
Usage

Run it with:
bash

cargo run --release

or directly:
bash

./target/release/NiNi3

Keyboard Shortcuts
Key	Action
Enter	Send message / confirm
Esc	Quit / cancel
Tab	Autocomplete command or model name
F2	Copy last AI response to clipboard
Ctrl+T	Open the theme editor
↑ / ↓	Navigate popups / scroll (theme editor: change field)
PageUp / PageDown	Scroll chat history
Mouse wheel	Scroll chat history
Slash Commands
Command	Description
/system	Edit the system prompt
/usage	Check today's OpenRouter usage
/clear	Clear the conversation history
/models	Switch provider/model
/model <name>	Switch directly to a specific model
Subagent System

NiNi features an autonomous subagent system where the main AI can decide to delegate tasks to specialized subagents for:

    Code analysis — deep dive into codebases

    File operations — read, write, and edit files

    Command execution — run system commands (e.g., cargo build)

How it works:

    Main agent receives your request

    If the task is complex, it spawns a subagent

    Subagent executes the task in parallel

    Results are returned to the main agent

    Main agent incorporates the results into its final response

Current limitations:

    Maximum concurrent subagents: 1 (configurable in code)

    Subagents operate with an isolated context (they don't see the main conversation history) — intentional to reduce token consumption

Theme Editor

Press Ctrl+T to open the live theme editor:
Key	Action
↑ / ↓	Select which color to edit
Tab	Switch between R / G / B channels
0-9	Type a value (0-255)
Enter	Confirm the value
R	Reset all colors to default
Esc	Close the editor

Changes are saved automatically to theme.toml in the project root and reloaded on every launch.
Support & Troubleshooting
Something went wrong?

If you encounter an issue:

    Check the GitHub Issues — open a new issue with:

        Description of the problem

        Steps to reproduce

        Error message (if any)

        Your OS and terminal emulator

    Check logs — look at ~/.cache/nini/logs/ for debug information

    Common issues:

        API key not working — verify .env file is in the project root and keys are correct

        Commands not executing — commands require confirmation by default for security

        Theme not saving — ensure theme.toml directory is writable

Customization

NiNi is highly customizable:

    Theme — edit colors with the live theme editor (Ctrl+T) or directly in theme.toml

    Splash screen — the ASCII art intro is customizable (modify src/splash.rs)

    API providers — works with any OpenAI-compatible API (OpenRouter, OpenCode Zen, etc.)

    Subagent limit — adjustable in the source code (currently hardcoded to 1)

    Note on providers: NiNi works with any OpenAI-compatible API, but /listmodels may not work consistently because each provider has its own endpoint format. OpenRouter and OpenCode Zen are officially supported.

Contributing

Not accepting pull requests at this time. This is a personal hobby project.

If you find bugs or have suggestions, please open an issue on GitHub.
Disclaimer

NiNi is an independent, unaffiliated hobby project. Do not enter API keys or sensitive information beyond what's needed to authenticate with the providers above — you are solely responsible for everything you send to these models.
