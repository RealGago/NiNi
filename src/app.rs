use crate::models::Message;
use crate::theme::Theme;
pub use crate::providers::{Provider, PROVIDERS};


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeField {
    Primary,
    Accent,
    Code,
    CodeBg,
    Error,
}

impl ThemeField {
    pub const ALL: [ThemeField; 5] = [
        ThemeField::Primary,
        ThemeField::Accent,
        ThemeField::Code,
        ThemeField::CodeBg,
        ThemeField::Error,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ThemeField::Primary => "Primary (purple/logo/borders)",
            ThemeField::Accent => "Accent (bullets/emphasis)",
            ThemeField::Code => "Code (Code Text)",
            ThemeField::CodeBg => "Code Background",
            ThemeField::Error => "Error/Warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RgbChannel {
    R,
    G,
    B,
}

impl RgbChannel {
    pub fn label(&self) -> &'static str {
        match self {
            RgbChannel::R => "R",
            RgbChannel::G => "G",
            RgbChannel::B => "B",
        }
    }
}



#[derive(Clone)]
pub enum Popup {
    None,
    SelectProvider { selected: usize },
    Loading,
    SelectModel {
        provider: Provider,
        models: Vec<String>,
        selected: usize,
    },
    EditSystemPrompt,
    ConfirmClear,
    ConfirmRunCommand { command: String },
    ThemeEditor {
        field: ThemeField,
        channel: RgbChannel,
        input: String,
    }
}

pub struct PendingKey {
    pub env_name: &'static str,
    pub label: &'static str,
}

pub enum Screen {
    Splash {
        input: String,
        pending: Vec<PendingKey>,
        idx: usize,
    },
    Chat,
}

#[derive(Clone)]
pub struct PendingToolRun {
    pub call: crate::models::ToolCall,
    pub command: String,
    pub results_so_far: Vec<Message>,
    pub remaining: Vec<crate::models::ToolCall>,
}

#[derive(Clone)]
pub struct ToolLogEntry {
    pub call_id: String,
    pub name: String,
    pub args_summary: String,
    pub status: ToolLogStatus,
}

#[derive(Clone)]
pub enum ToolLogStatus {
    Running,
    Done { duration_ms: u128 },
    Error { duration_ms: u128 },
}


pub struct App {
    pub input: String,
    pub messages: Vec<Message>,
    pub cursor_position: usize,
    pub scroll: u16,
    pub model: String,
    pub provider: Provider,
    pub status: String,
    pub is_loading: bool,
    pub should_quit: bool,
    pub free_models: Vec<String>,
    pub theme: Theme,
    pub popup: Popup,
    pub system_prompt: String,
    pub screen: Screen,
    pub tick : u64,
    pub api_keys: std::collections::HashMap<&'static str, String>,
    pub pending_tool_run: Option<PendingToolRun>,
    pub tool_log: Vec<ToolLogEntry>,
}

impl App{
pub fn new(model: String, free_models: Vec<String>) -> Self {
    let mut pending = Vec::new();
    let mut api_keys = std::collections::HashMap::new();

    for provider in PROVIDERS {
        match std::env::var(provider.key_env) {
            Ok(k) if !k.is_empty() => {
                api_keys.insert(provider.key_env, k);
            }
            _ => {
                pending.push(PendingKey {
                    env_name: provider.key_env,
                    label: provider.label,                  
                });
            }
        }
    }

    let screen = Screen::Splash {
        input: String::new(),
        pending,
        idx: 0,
    };

    App {
        input: String::new(),
        messages: Vec::new(),
        model,
        provider: &PROVIDERS[0],
        cursor_position: 0,
        scroll: 0,
        status: String::from("ready"),
        is_loading: false,
        should_quit: false,
        free_models,
        popup: Popup::None,
        system_prompt: String::from("You are NiNi, a helpful AI assistant inside a TUI"),
        screen,
        api_keys,
        pending_tool_run: None,
        tool_log: Vec::new(),
        tick: 0,
        theme: Theme::load(),
    }
}
    
    pub fn push_tool_event(&mut self, event: crate::tools::ToolEvent) {
        match event {
            crate::tools::ToolEvent::Started { call_id, tool_name, args_summary } => {
                self.tool_log.push(ToolLogEntry {
                    call_id,
                    name: tool_name,
                    args_summary,
                    status: ToolLogStatus::Running,
                });
            }
            crate::tools::ToolEvent::Finished { call_id, result, duration_ms } => {
                if let Some(entry) = self.tool_log.iter_mut().find(|e| e.call_id == call_id) {
                    entry.status = match result {
                        Ok(_) => ToolLogStatus::Done { duration_ms },
                        Err(_) => ToolLogStatus::Error { duration_ms },
                    };
                }
            }
        }
    }

    pub fn open_models_popup(&mut self) {
        self.popup = Popup::SelectProvider { selected: 0 };
        self.input.clear();
    }

    pub fn popup_up(&mut self) {
        match &mut self.popup {
            Popup::SelectProvider{ selected } => {
                let len = PROVIDERS.len();
                if *selected == 0 {
                    *selected = len - 1;
                } else {
                    *selected -= 1;
                }
            }
            Popup::SelectModel {selected, .. } => {            
                *selected = selected.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn popup_down(&mut self) {
        match &mut self.popup {
            Popup::SelectProvider { selected } => {
                let len = PROVIDERS.len();
                *selected = (*selected +1) % len;
            }
            Popup::SelectModel { models, selected, .. } 
                if !models.is_empty() && *selected < models.len() - 1 => {
                    *selected += 1;
                }
            
            _ => {}
        }
    }

    pub fn popup_close(&mut self) {
        self.popup = Popup::None;
        self.status = String::from("ready");
    }
}
