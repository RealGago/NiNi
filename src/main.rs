#[macro_use]
mod log;
mod agent;
mod app;
mod lsp;
mod skills;
mod theme;
mod tools;
mod ui;

use agent::api;
use agent::models::{ChatResponse, Message};
use agent::providers::Provider;
use anyhow::Result;
use app::{App, Popup, Screen};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use lsp::{LspManager, config::LspSettings};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use ui::commands::{COMMAND_LIST, Command, parse_command};

enum AsyncResult {
    Chat(usize, Result<(ChatResponse, Option<String>), String>),
    Models(Provider, Result<Vec<String>, String>),
    NeedsConfirmation(usize, app::PendingToolRun),
}

enum PopupAction {
    None,

    Handled,
    Up,
    Down,
    Close,
    ConfirmClearYes,
    SelectProvider(usize),
    SelectModel(Provider, usize),
    ConfirmRunCommandYes,
    ConfirmRunCommandNo,
    SelectSkill(usize),
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let client = reqwest::Client::new();

    let lsp_manager = Arc::new(Mutex::new(LspManager::new(LspSettings::load())));

    let free_models = api::fetch_model_ids(&client, &agent::providers::PROVIDERS[0], None)
        .await
        .unwrap_or_default();

    let mut app = App::new("openrouter/free".to_string(), free_models);
    app.skills = skills::skill::load_all_skills();

    let (tx, mut rx) = mpsc::unbounded_channel::<AsyncResult>();
    let (tool_events_tx, mut tool_events_rx) = mpsc::unbounded_channel::<tools::ToolEvent>();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        app.tick = app.tick.wrapping_add(1);
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match &app.screen {
                    Screen::Splash { .. } => handle_splash_key(key.code, &mut app),
                    Screen::Chat => handle_chat_key(
                        key.code,
                        key.modifiers,
                        &mut app,
                        &client,
                        &tx,
                        &tool_events_tx,
                        &lsp_manager,
                    ),
                },
                Event::Mouse(mouse_event) => {
                    if let Screen::Chat = app.screen {
                        match mouse_event.kind {
                            MouseEventKind::ScrollUp => {
                                app.session_mut().scroll = app.session().scroll.saturating_add(2);
                            }
                            MouseEventKind::ScrollDown => {
                                app.session_mut().scroll = app.session().scroll.saturating_sub(2);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        while let Ok(event) = tool_events_rx.try_recv() {
            app.push_tool_event(event);
        }

        while let Ok(result) = rx.try_recv() {
            app.session_mut().is_loading = false;
            match result {
                AsyncResult::Chat(session_id, Ok((chat, notice))) => {
                    if let Some(choice) = chat.choices.first() {
                        if let Some(calls) = choice.message.tool_calls.clone() {
                            let (model, mut history) =
                                if let Some(session) = app.session_by_id_mut(session_id) {
                                    session.messages.push(Message {
                                        role: "assistant".to_string(),
                                        content: choice.message.content.clone().unwrap_or_default(),
                                        tool_calls: Some(calls.clone()),
                                        tool_call_id: None,
                                    });
                                    (session.model.clone(), session.messages.clone())
                                } else {
                                    app.popup_close();
                                    continue;
                                };

                            let client = client.clone();
                            let api_keys = app.api_keys.clone();
                            let tool_events_tx = tool_events_tx.clone();
                            let tx = tx.clone();

                            let ctx = tools::ToolContext {
                                client: client.clone(),
                                api_keys: api_keys.clone(),
                                model: model.clone(),
                                subagent_tool_defs: build_subagent_tool_defs(),
                                lsp_manager: lsp_manager.clone(),
                            };
                            tokio::spawn(async move {
                                let outcome =
                                    tools::execute_tool_batch(&calls, &ctx, &tool_events_tx).await;
                                match outcome {
                                    tools::ToolBatchOutcome::Done(results) => {
                                        history.extend(results);
                                        let tool_defs = build_tool_defs();
                                        let res = api::send_chat(
                                            &client, &api_keys, &model, &history, &tool_defs,
                                        )
                                        .await
                                        .map_err(|e| e.to_string());
                                        tx.send(AsyncResult::Chat(session_id, res)).ok();
                                    }
                                    tools::ToolBatchOutcome::NeedsConfirmation {
                                        call,
                                        command,
                                        results_so_far,
                                        remaining,
                                    } => {
                                        tx.send(AsyncResult::NeedsConfirmation(
                                            session_id,
                                            app::PendingToolRun {
                                                call,
                                                command,
                                                results_so_far,
                                                remaining,
                                            },
                                        ))
                                        .ok();
                                    }
                                }
                            });
                        } else if let Some(text) = &choice.message.content {
                            if let Some(session) = app.session_by_id_mut(session_id) {
                                session.messages.push(Message::assistant(text.clone()));
                            }
                            app.status = notice.unwrap_or_else(|| "ready".to_string());
                        }
                    }
                }
                AsyncResult::Chat(_, Err(e)) => app.status = format!("error: {}", e),
                AsyncResult::Models(provider, Ok(models)) => {
                    app.status = format!("loaded {} models from {}", models.len(), provider.label);
                    app.popup = Popup::SelectModel {
                        provider,
                        models,
                        selected: 0,
                    };
                }
                AsyncResult::Models(_, Err(e)) => {
                    app.status = format!("error fetching models: {}", e);
                    app.popup = Popup::None;
                }
                AsyncResult::NeedsConfirmation(session_id, pending) => {
                    let command = pending.command.clone();
                    app.pending_tool_run = Some(pending);
                    app.popup = Popup::ConfirmRunCommand { command };
                    if let Some(session) = app.session_by_id_mut(session_id) {
                        session.is_loading = false;
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

/// Handles keys while the splash screen (API keys collection) is active.
fn handle_splash_key(code: KeyCode, app: &mut App) {
    if code == KeyCode::Esc {
        app.should_quit = true;
        return;
    }

    let mut collected: Option<(&'static str, String)> = None;
    let mut advanced = false;

    if let Screen::Splash {
        input,
        pending,
        idx,
    } = &mut app.screen
    {
        match code {
            KeyCode::Char(c) => input.push(c),
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Enter => {
                if pending.is_empty() {
                    advanced = true;
                } else if *idx < pending.len() {
                    let current = &pending[*idx];
                    if !input.is_empty() {
                        collected = Some((current.env_name, input.clone()));
                    }
                    input.clear();
                    *idx += 1;
                    advanced = true;
                }
            }
            _ => {}
        }
    }

    if let Some((env_name, value)) = collected {
        app.api_keys.insert(env_name, value);
    }

    if advanced {
        let finished =
            matches!(&app.screen, Screen::Splash { pending, idx, .. } if *idx >= pending.len());

        if finished {
            if !app.api_keys.is_empty() {
                app.screen = Screen::Chat;
                app.status = "ready".to_string();
            } else {
                app.status =
                    "you must provide at least one API key (OpenRouter or OpenCode) to continue"
                        .to_string();
                if let Screen::Splash { input, idx, .. } = &mut app.screen {
                    input.clear();
                    *idx = 0;
                }
            }
        }
    }
}

/// Handles keys while the chat screen (with or without popup open) is active.
fn handle_chat_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    app: &mut App,
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<AsyncResult>,
    tool_events_tx: &mpsc::UnboundedSender<tools::ToolEvent>,
    lsp_manager: &Arc<Mutex<LspManager>>,
) {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('t') {
        app.popup = Popup::ThemeEditor {
            field: crate::app::ThemeField::Primary,
            channel: crate::app::RgbChannel::R,
            input: String::new(),
        };
        return;
    }

    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('n') {
        app.new_session();
        return;
    }

    if modifiers.contains(KeyModifiers::ALT) {
        if let KeyCode::Char(c) = code {
            if let Some(digit) = c.to_digit(10) {
                if digit >= 1 && digit <= 9 {
                    app.goto_session((digit - 1) as usize);
                    return;
                }
            }
        }
    }

    if let Popup::ThemeEditor {
        field,
        channel,
        input,
    } = &mut app.popup
    {
        use crate::app::{RgbChannel, ThemeField};

        match code {
            KeyCode::Char('r') => {
                app.theme = crate::theme::Theme::default();
                let _ = app.theme.save();
                input.clear();
            }
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Up => {
                let idx = ThemeField::ALL.iter().position(|f| f == field).unwrap();
                let new_idx = if idx == 0 {
                    ThemeField::ALL.len() - 1
                } else {
                    idx - 1
                };
                *field = ThemeField::ALL[new_idx];
                input.clear();
            }
            KeyCode::Down => {
                let idx = ThemeField::ALL.iter().position(|f| f == field).unwrap();
                let new_idx = (idx + 1) % ThemeField::ALL.len();
                *field = ThemeField::ALL[new_idx];
                input.clear();
            }
            KeyCode::Tab => {
                *channel = match channel {
                    RgbChannel::R => RgbChannel::G,
                    RgbChannel::G => RgbChannel::B,
                    RgbChannel::B => RgbChannel::R,
                };
                input.clear();
            }
            KeyCode::Char(c) if c.is_ascii_digit() && input.len() < 3 => {
                input.push(c);
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Enter => {
                if let Ok(value) = input.parse::<u16>() {
                    let clamped = value.min(255) as u8;
                    app.theme.set_channel(*field, *channel, clamped);
                    let _ = app.theme.save();
                }
                input.clear();
            }
            _ => {}
        }
        return;
    }

    let action = match &app.popup {
        Popup::None => PopupAction::None,
        Popup::Loading => {
            if code == KeyCode::Esc {
                PopupAction::Close
            } else {
                PopupAction::Handled
            }
        }
        Popup::ThemeEditor { .. } => PopupAction::Handled,
        Popup::SelectProvider { selected } => match code {
            KeyCode::Up => PopupAction::Up,
            KeyCode::Down => PopupAction::Down,
            KeyCode::Esc => PopupAction::Close,
            KeyCode::Enter => PopupAction::SelectProvider(*selected),
            _ => PopupAction::Handled,
        },
        Popup::SelectModel {
            provider, selected, ..
        } => match code {
            KeyCode::Up => PopupAction::Up,
            KeyCode::Down => PopupAction::Down,
            KeyCode::Esc => PopupAction::Close,
            KeyCode::Enter => PopupAction::SelectModel(*provider, *selected),
            _ => PopupAction::Handled,
        },
        Popup::SelectSkill { selected, .. } => match code {
            KeyCode::Up => PopupAction::Up,
            KeyCode::Down => PopupAction::Down,
            KeyCode::Esc => PopupAction::Close,
            KeyCode::Enter => PopupAction::SelectSkill(*selected),
            _ => PopupAction::Handled,
        },
        Popup::EditSystemPrompt => match code {
            KeyCode::Esc | KeyCode::Enter => PopupAction::Close,
            KeyCode::Backspace => {
                app.system_prompt.pop();
                PopupAction::Handled
            }
            KeyCode::Char(c) => {
                app.system_prompt.push(c);
                PopupAction::Handled
            }
            _ => PopupAction::Handled,
        },
        Popup::ConfirmClear => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                PopupAction::ConfirmClearYes
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => PopupAction::Close,
            _ => PopupAction::Handled,
        },
        Popup::ConfirmRunCommand { .. } => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                PopupAction::ConfirmRunCommandYes
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                PopupAction::ConfirmRunCommandNo
            }
            _ => PopupAction::Handled,
        },
    };

    match action {
        PopupAction::Up => app.popup_up(),
        PopupAction::Down => app.popup_down(),
        PopupAction::Close => app.popup_close(),
        PopupAction::Handled => {}
        PopupAction::ConfirmClearYes => {
            app.session_mut().messages.clear();
            app.popup_close();
            app.status = "history cleared".to_string();
        }
        PopupAction::ConfirmRunCommandYes => {
            if let Some(pending) = app.pending_tool_run.take() {
                app.popup = Popup::None;
                app.session_mut().is_loading = true;
                app.status = "running command...".to_string();
                spawn_continue_tool_run(
                    client,
                    tx,
                    app,
                    pending,
                    true,
                    &tool_events_tx,
                    lsp_manager,
                );
            }
        }
        PopupAction::ConfirmRunCommandNo => {
            if let Some(pending) = app.pending_tool_run.take() {
                app.popup = Popup::None;
                app.session_mut().is_loading = true;
                app.status = "command rejected, continuing...".to_string();
                spawn_continue_tool_run(
                    client,
                    tx,
                    app,
                    pending,
                    false,
                    &tool_events_tx,
                    lsp_manager,
                );
            }
        }
        PopupAction::SelectProvider(selected) => {
            let provider = &agent::providers::PROVIDERS[selected];
            app.popup = Popup::Loading;
            app.status = format!("fetching {} models...", provider.label);

            let client = client.clone();
            let tx = tx.clone();
            let api_key = app.api_keys.get(provider.key_env).cloned();

            tokio::spawn(async move {
                let res = api::fetch_model_ids(&client, provider, api_key.as_deref())
                    .await
                    .map_err(|e| e.to_string());
                tx.send(AsyncResult::Models(provider, res)).ok();
            });
        }
        PopupAction::SelectModel(provider, selected) => {
            if let Popup::SelectModel { models, .. } = &app.popup {
                if let Some(chosen_model) = models.get(selected) {
                    app.session_mut().model = chosen_model.clone();
                    app.session_mut().provider = provider;
                    app.status = format!(
                        "model switched to: {} ({})",
                        app.session().model,
                        provider.label
                    );
                }
            }
            app.popup_close();
        }
        PopupAction::SelectSkill(selected) => {
            if let Popup::SelectSkill { skills, .. } = &app.popup {
                if let Some(skill) = skills.get(selected) {
                    let skill_prompt =
                        format!("Use the following skill:\n\n{}", skill.instructions);
                    app.popup_close();
                    handle_skill_invocation(app, client, tx, skill_prompt);
                }
            }
        }
        PopupAction::None => match code {
            KeyCode::Esc => app.should_quit = true,

            KeyCode::F(2) => {
                let text_to_copy = app
                    .session()
                    .messages
                    .iter()
                    .rfind(|m| m.role == "assistant")
                    .map(|m| m.content.clone());
                if let Some(content) = text_to_copy {
                    copy_to_clipboard(&content, app);
                } else {
                    app.status = "no AI response available to copy".to_string();
                }
            }

            KeyCode::Enter => {
                let text = app.input.trim().to_string();
                app.input.clear();
                app.session_mut().scroll = 0;
                app.cursor_position = 0;
                if !text.is_empty() {
                    handle_input(&text, app, client, tx);
                }
            }

            KeyCode::Left if app.cursor_position > 0 => {
                let prev_len = app.input[..app.cursor_position]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                app.cursor_position -= prev_len;
            }

            KeyCode::Right if app.cursor_position < app.input.len() => {
                let next_len = app.input[app.cursor_position..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                app.cursor_position += next_len;
            }

            KeyCode::Backspace if app.cursor_position > 0 => {
                let prev_len = app.input[..app.cursor_position]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                let new_pos = app.cursor_position - prev_len;
                app.input.remove(new_pos);
                app.cursor_position = new_pos;
            }

            KeyCode::Char(c) => {
                app.input.insert(app.cursor_position, c);
                app.cursor_position += c.len_utf8();
            }

            KeyCode::PageUp => {
                app.session_mut().scroll = app.session().scroll.saturating_add(5);
            }
            KeyCode::PageDown => {
                app.session_mut().scroll = app.session().scroll.saturating_sub(5);
            }

            KeyCode::Tab => autocomplete(app),
            _ => {}
        },
    }
}

fn handle_input(
    text: &str,
    app: &mut App,
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<AsyncResult>,
) {
    match parse_command(text) {
        Command::Exit => app.should_quit = true,
        Command::Clear => {
            app.popup = Popup::ConfirmClear;
        }
        Command::SwitchModel(m) => {
            app.status = format!("model switched to: {}", m);
            app.session_mut().model = m;
        }
        Command::Models => {
            app.open_models_popup();
        }
        Command::SystemPrompt => {
            app.popup = Popup::EditSystemPrompt;
            app.status = "editing system prompt... (Enter to save, Esc to exit)".to_string();
        }
        Command::Skills => {
            app.open_skills_popup();
            app.status = format!("{} skills available", app.skills.len());
        }
        Command::InvokeSkill(name) => {
            if let Some(skill) = app.get_skill_by_name(&name) {
                let skill_prompt = format!("Use the following skill:\n\n{}", skill.instructions);
                handle_skill_invocation(app, client, tx, skill_prompt);
            } else {
                app.status = format!("skill '{}' not found", name);
            }
        }
        Command::Chat(msg) => {
            if app.session().messages.is_empty() {
                let system_prompt = build_system_prompt(app);
                app.session_mut()
                    .messages
                    .push(Message::system(system_prompt));
            }

            app.session_mut().messages.push(Message::user(msg));
            app.session_mut().is_loading = true;
            app.status = "thinking...".to_string();

            let client = client.clone();
            let api_keys = app.api_keys.clone();
            let session_id = app.session().id;
            let model = app.session().model.clone();
            let history = app.session().messages.clone();
            let tool_defs = build_tool_defs();
            let tx = tx.clone();

            tokio::spawn(async move {
                let res = api::send_chat(&client, &api_keys, &model, &history, &tool_defs)
                    .await
                    .map_err(|e| e.to_string());
                tx.send(AsyncResult::Chat(session_id, res)).ok();
            });
        }
    }
}

fn handle_skill_invocation(
    app: &mut App,
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<AsyncResult>,
    skill_prompt: String,
) {
    if app.session().messages.is_empty() {
        let system_prompt = build_system_prompt(app);
        app.session_mut()
            .messages
            .push(Message::system(system_prompt));
    }

    app.session_mut().messages.push(Message::user(skill_prompt));
    app.session_mut().is_loading = true;
    app.status = "running skill...".to_string();

    let client = client.clone();
    let api_keys = app.api_keys.clone();
    let session_id = app.session().id;
    let model = app.session().model.clone();
    let history = app.session().messages.clone();
    let tool_defs = build_tool_defs();
    let tx = tx.clone();

    tokio::spawn(async move {
        let res = api::send_chat(&client, &api_keys, &model, &history, &tool_defs)
            .await
            .map_err(|e| e.to_string());
        tx.send(AsyncResult::Chat(session_id, res)).ok();
    });
}

fn build_system_prompt(app: &App) -> String {
    let mut prompt = app.system_prompt.clone();

    if !app.skills.is_empty() {
        prompt.push_str("\n\n# Skills Disponíveis\n\n");
        prompt.push_str("Você pode usar as seguintes skills quando apropriado. ");
        prompt.push_str("Se o usuário pedir algo relacionado, siga as instruções da skill.\n\n");

        for skill in &app.skills {
            if !skill.disable_model_invocation {
                prompt.push_str(&format!("## {}\n{}\n\n", skill.name, skill.description));
            }
        }
    }

    prompt
}

fn build_tool_defs() -> Vec<agent::models::ToolDefinition> {
    vec![
        agent::models::ToolDefinition::from_tool(&tools::SpawnSubagent),
        agent::models::ToolDefinition::from_tool(&tools::ReadFile),
        agent::models::ToolDefinition::from_tool(&tools::ListDirectory),
        agent::models::ToolDefinition::from_tool(&tools::WriteFile),
        agent::models::ToolDefinition::from_tool(&tools::EditFile),
        agent::models::ToolDefinition::from_tool(&tools::Grep),
        agent::models::ToolDefinition::from_tool(&tools::RunCommand),
        agent::models::ToolDefinition::from_tool(&tools::GotoDefinition),
        agent::models::ToolDefinition::from_tool(&tools::FindSymbol),
    ]
}

// Subagents can't spawn more subagents and can't run shell commands.
fn build_subagent_tool_defs() -> Vec<agent::models::ToolDefinition> {
    build_tool_defs()
        .into_iter()
        .filter(|t| t.function.name != "spawn_subagent" && t.function.name != "run_command")
        .collect()
}

fn spawn_continue_tool_run(
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<AsyncResult>,
    app: &App,
    pending: app::PendingToolRun,
    approved: bool,
    tool_events_tx: &mpsc::UnboundedSender<tools::ToolEvent>,
    lsp_manager: &Arc<Mutex<LspManager>>,
) {
    let client = client.clone();
    let api_keys = app.api_keys.clone();
    let session_id = app.session().id;
    let model = app.session().model.clone();
    let mut history = app.session().messages.clone();
    let tx = tx.clone();
    let tool_events_tx = tool_events_tx.clone();

    let ctx = tools::ToolContext {
        client: client.clone(),
        api_keys: api_keys.clone(),
        model: model.clone(),
        subagent_tool_defs: build_subagent_tool_defs(),
        lsp_manager: lsp_manager.clone(),
    };

    tokio::spawn(async move {
        let outcome = tools::continue_after_confirmation(
            pending.call,
            approved,
            pending.results_so_far,
            pending.remaining,
            &ctx,
            &tool_events_tx,
        )
        .await;
        match outcome {
            tools::ToolBatchOutcome::Done(results) => {
                history.extend(results);
                let tool_defs = build_tool_defs();
                let res = api::send_chat(&client, &api_keys, &model, &history, &tool_defs)
                    .await
                    .map_err(|e| e.to_string());
                tx.send(AsyncResult::Chat(session_id, res)).ok();
            }
            tools::ToolBatchOutcome::NeedsConfirmation {
                call,
                command,
                results_so_far,
                remaining,
            } => {
                tx.send(AsyncResult::NeedsConfirmation(
                    session_id,
                    app::PendingToolRun {
                        call,
                        command,
                        results_so_far,
                        remaining,
                    },
                ))
                .ok();
            }
        }
    });
}

/// Simple autocomplete: Tab completes to the first command/model that matches the typed text.
fn autocomplete(app: &mut App) {
    if app.input.starts_with('/') && !app.input.starts_with("/model ") {
        if let Some(m) = COMMAND_LIST
            .iter()
            .find(|c| c.starts_with(app.input.as_str()))
        {
            app.input = m.to_string();
        }
    } else if let Some(partial) = app.input.strip_prefix("/model ") {
        if let Some(m) = app.free_models.iter().find(|m| m.contains(partial)) {
            app.input = format!("/model {}", m);
        }
    }
}

fn copy_to_clipboard(text: &str, app: &mut App) {
    use arboard::{Clipboard, SetExtLinux};

    let text = text.to_string();
    std::thread::spawn(move || {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set().wait().text(text);
        }
    });

    app.status = "last answer copied to clipboard!".to_string();
}
