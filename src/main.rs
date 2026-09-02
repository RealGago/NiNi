mod api;
mod app;
mod commands;
mod models;
mod ui;
mod theme;
mod providers;
mod tools;
use anyhow::Result;
use app::{App, Popup, Provider, Screen};
use commands::{parse_command, Command, COMMAND_LIST};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use models::Message;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::env;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use futures::future::join_all;
use crate::models::ChatResponse;


enum AsyncResult {
    Chat(Result<(ChatResponse, Option<String>), String>),
    Models(Provider, Result<Vec<String>, String>),
    NeedsConfirmation(app::PendingToolRun),
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let client = reqwest::Client::new();

    // Public, no key needed to list free models from OpenRouter.
    let free_models = api::fetch_model_ids(&client, &providers::PROVIDERS[0], None).await.unwrap_or_default();

    
    let mut app = App::new("openrouter/free".to_string(), free_models);
    let (tx, mut rx) = mpsc::unbounded_channel::<AsyncResult>();
    let (tool_events_tx , mut tool_events_rx) = mpsc::unbounded_channel::<tools::ToolEvent>();


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
                Event::Key(key) 
                    if key.kind == KeyEventKind::Press => {
                         match &app.screen {
                            Screen::Splash { .. } => handle_splash_key(key.code, &mut app),
                            Screen::Chat => handle_chat_key(key.code, key.modifiers, &mut app, &client, &tx, &tool_events_tx),
                        }                    
                    }
                
                Event::Mouse(mouse_event) => {
                    if let Screen::Chat = app.screen {
                        match mouse_event.kind {
                            MouseEventKind::ScrollUp => {
                                app.scroll = app.scroll.saturating_add(2);
                            }
                            MouseEventKind::ScrollDown => {
                                app.scroll = app.scroll.saturating_sub(2);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        
        while let  Ok(event) = tool_events_rx.try_recv() {
            app.push_tool_event(event);
        }


        while let Ok(result) = rx.try_recv() {
            app.is_loading = false;
            match result {
                AsyncResult::Chat(Ok((chat, notice))) => {
                    if let Some(choice) = chat.choices.first() {
                         if let Some(calls) = choice.message.tool_calls.clone() {
                            app.status = format!("running {} tool call(s)...", calls.len());
                            app.messages.push(Message {
                                role: "assistant".to_string(),
                                content: choice.message.content.clone().unwrap_or_default(),
                                tool_calls: Some(calls.clone()),
                                tool_call_id: None,
                            });
                            let client = client.clone();
                            let api_keys = app.api_keys.clone();
                            let model = app.model.clone();
                            let mut history = app.messages.clone();
                            let tool_events_tx = tool_events_tx.clone();
                            let tx = tx.clone();


                            let ctx = tools::ToolContext {
                                 client: client.clone(),
                                 api_keys: api_keys.clone(),
                                 model: model.clone(),
                                 subagent_tool_defs: build_subagent_tool_defs(),
                            };
                            tokio::spawn(async move {
                                let outcome = tools::execute_tool_batch(&calls, &ctx, &tool_events_tx).await;
                                match outcome {
                                    tools::ToolBatchOutcome::Done(results) => {
                                        history.extend(results);
                                        let tool_defs = build_tool_defs();
                                        let res = api::send_chat(&client, &api_keys, &model, &history, &tool_defs)
                                            .await
                                            .map_err(|e| e.to_string());
                                        tx.send(AsyncResult::Chat(res)).ok();
                                    }
                                    tools::ToolBatchOutcome::NeedsConfirmation {
                                        call,
                                        command,
                                        results_so_far,
                                        remaining,
                                    } => {
                                        tx.send(AsyncResult::NeedsConfirmation(app::PendingToolRun {
                                            call,
                                            command,
                                            results_so_far,
                                            remaining,
                                        }))
                                        .ok();
                                    }
                            }
                });
        } else if let Some(text) = &choice.message.content {
            app.messages.push(Message::assistant(text.clone()));
            app.status = notice.unwrap_or_else(|| "ready".to_string());
        }
    }
}                     


                AsyncResult::Chat(Err(e)) => app.status = format!("error: {}", e),
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
                AsyncResult::NeedsConfirmation(pending) => {
                    let command = pending.command.clone();
                    app.pending_tool_run = Some(pending);
                    app.popup = Popup::ConfirmRunCommand { command };
                    app.is_loading = false;
                }   


            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

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

    if let Screen::Splash { input, pending, idx } = &mut app.screen {
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
        let finished = matches!(&app.screen, Screen::Splash { pending, idx, .. } if *idx >= pending.len());

        if finished {
           if !app.api_keys.is_empty() {
                app.screen = Screen::Chat;
                app.status = "ready".to_string();
            } else {
                app.status = "you must provide at least one API key (OpenRouter or OpenCode) to continue".to_string();
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
) {

    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('t') {
        app.popup = Popup::ThemeEditor { 
            field: crate::app::ThemeField::Primary,
            channel: crate::app::RgbChannel::R,
            input: String::new(),
        };
        return;
    }

    if let  Popup::ThemeEditor { field, channel, input } = &mut app.popup {
        use crate::app::{RgbChannel , ThemeField};

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
                let new_idx = if idx == 0 { ThemeField::ALL.len() - 1 } else { idx - 1 };
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
            KeyCode::Char(c) if c.is_ascii_digit() 
                && input.len() < 3 => {
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
        Popup::SelectModel { provider, selected, .. } => match code {
            KeyCode::Up => PopupAction::Up,
            KeyCode::Down => PopupAction::Down,
            KeyCode::Esc => PopupAction::Close,
            KeyCode::Enter => PopupAction::SelectModel(*provider, *selected),
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
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => PopupAction::ConfirmClearYes,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => PopupAction::Close,
            _ => PopupAction::Handled,
        },

        Popup::ConfirmRunCommand { .. } => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => PopupAction::ConfirmRunCommandYes,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => PopupAction::ConfirmRunCommandNo,
            _ => PopupAction::Handled,
        },
    };

    match action {
        PopupAction::Up => app.popup_up(),
        PopupAction::Down => app.popup_down(),
        PopupAction::Close => app.popup_close(),
        PopupAction::Handled => {}
        PopupAction::ConfirmClearYes => {
            app.messages.clear();
            app.popup_close();
            app.status = "history cleared".to_string();
        }
        PopupAction::ConfirmRunCommandYes => {
    if let Some(pending) = app.pending_tool_run.take() {
        app.popup = Popup::None;
        app.is_loading = true;
        app.status = "running command...".to_string();
        spawn_continue_tool_run(client, tx, app, pending, true, &tool_events_tx);
            }
        }
        PopupAction::ConfirmRunCommandNo => {
            if let Some(pending) = app.pending_tool_run.take() {
                app.popup = Popup::None;
                app.is_loading = true;
                app.status = "command rejected, continuing...".to_string();
                spawn_continue_tool_run(client, tx, app, pending, false, &tool_events_tx);
            }
        }
        
    PopupAction::SelectProvider(selected) => {
        let provider = &providers::PROVIDERS[selected];
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
            if let Popup::SelectModel { models, .. } = &app.popup 
                && let Some(chosen_model) = models.get(selected) {
                    app.model = chosen_model.clone();
                    app.provider = provider;
                    app.status = format!("model switched to: {} ({})", app.model, provider.label);
                }
            app.popup_close();
            }

           
    
        
        PopupAction::None => match code {
            KeyCode::Esc => app.should_quit = true,

            KeyCode::F(2) => {
                let text_to_copy = app
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
                app.scroll = 0;
                app.cursor_position = 0;
                if !text.is_empty() {
                    handle_input(&text, app, client, tx);
                }
            }

            // Cursor navigation is char-boundary aware to stay safe with UTF-8
            // (accented characters like á, ç, ã take more than 1 byte).
            KeyCode::Left
                if app.cursor_position > 0 => {
                    let prev_len = app.input[..app.cursor_position]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    app.cursor_position -= prev_len;
                }
            
            KeyCode::Right 
                if app.cursor_position < app.input.len() => {
                    let next_len = app.input[app.cursor_position..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    app.cursor_position += next_len;
                }
            
            KeyCode::Backspace
                if app.cursor_position > 0 => {
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
                app.scroll = app.scroll.saturating_add(5);
            }
            KeyCode::PageDown => {
                app.scroll = app.scroll.saturating_sub(5);
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
            app.model = m;
        }
        Command::Models => {
            app.open_models_popup();
        }
        Command::SystemPrompt => {
            app.popup = Popup::EditSystemPrompt;
            app.status = "editing system prompt... (Enter to save, Esc to exit)".to_string();
        }
        Command::Chat(msg) => {
            if app.messages.is_empty() {
                app.messages.push(Message::system(app.system_prompt.clone()));            
            }

            app.messages.push(Message::user(msg));


            app.is_loading = true;
            app.status = "thinking...".to_string();
            
            let client = client.clone();
            let api_keys = app.api_keys.clone();
            let model = app.model.clone();
            let history = app.messages.clone();
            let tool_defs = build_tool_defs();            
            let tx = tx.clone();

            tokio::spawn(async move {
                let res = api::send_chat(&client, &api_keys, &model, &history, &tool_defs)
                    .await
                    .map_err(|e| e.to_string());
                tx.send(AsyncResult::Chat(res)).ok();
            });
        }
    }
}

fn build_tool_defs() -> Vec<models::ToolDefinition> {
    vec![
        models::ToolDefinition::from_tool(&tools::SpawnSubagent), 
        models::ToolDefinition::from_tool(&tools::ReadFile),
        models::ToolDefinition::from_tool(&tools::ListDirectory),
        models::ToolDefinition::from_tool(&tools::WriteFile),
        models::ToolDefinition::from_tool(&tools::EditFile),
        models::ToolDefinition::from_tool(&tools::Grep),
        models::ToolDefinition::from_tool(&tools::RunCommand),
        
    ]
}
// Subagents can't spawn more subagents and can't run shell commands.
fn build_subagent_tool_defs() -> Vec<models::ToolDefinition> {
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
) {
    let client = client.clone();
    let api_keys = app.api_keys.clone();
    let model = app.model.clone();
    let mut history = app.messages.clone();
    let tx = tx.clone();
    let tool_events_tx = tool_events_tx.clone();

    let ctx = tools::ToolContext {
        client: client.clone(),
        api_keys: api_keys.clone(),
        model: model.clone(),
        subagent_tool_defs: build_subagent_tool_defs(),
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
                tx.send(AsyncResult::Chat(res)).ok();
            }
            tools::ToolBatchOutcome::NeedsConfirmation {
                call,
                command,
                results_so_far,
                remaining,
            } => {
                tx.send(AsyncResult::NeedsConfirmation(app::PendingToolRun {
                    call,
                    command,
                    results_so_far,
                    remaining,
                }))
                .ok();
            }
        }
    });
}




/// Simple autocomplete: Tab completes to the first command/model that matches the typed text.
fn autocomplete(app: &mut App) {
    if app.input.starts_with('/') && !app.input.starts_with("/model ") {
        if let Some(m) = COMMAND_LIST.iter().find(|c| c.starts_with(app.input.as_str())) {
            app.input = m.to_string();
        }
    } else if let Some(partial) = app.input.strip_prefix("/model ") 
        && let Some(m) = app.free_models.iter().find(|m| m.contains(partial)) {
            app.input = format!("/model {}", m);
        }
    
}

fn copy_to_clipboard(text: &str, app: &mut App) {
    use arboard::{Clipboard, SetExtLinux};

    let text = text.to_string();
    // It runs in a separate thread with `.wait()` to keep it available until the paste happens.
    // This doesnt block the TUI in the meantime, btw.
    std::thread::spawn(move || {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set().wait().text(text);
        }
    });

    app.status = "last answer copied to clipboard!".to_string();
}
