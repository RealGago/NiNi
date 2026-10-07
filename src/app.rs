use crate::agent::models::Message;
pub use crate::agent::providers::{PROVIDERS, Provider};
use crate::theme::Theme;

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
    SelectProvider {
        selected: usize,
    },
    Loading,
    SelectModel {
        provider: Provider,
        models: Vec<String>,
        selected: usize,
    },
    EditSystemPrompt,
    ConfirmClear,
    ConfirmRunCommand {
        command: String,
    },
    ThemeEditor {
        field: ThemeField,
        channel: RgbChannel,
        input: String,
    },
    SelectSkill {
        skills: Vec<crate::skills::skill::Skill>,
        selected: usize,
    },
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
    pub call: crate::agent::models::ToolCall,
    pub command: String,
    pub results_so_far: Vec<Message>,
    pub remaining: Vec<crate::agent::models::ToolCall>,
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

pub struct ChatSession {
    pub id: usize,
    pub messages: Vec<Message>,
    pub model: String,
    pub provider: Provider,
    pub tool_log: Vec<ToolLogEntry>,
    pub scroll: u16,
    pub is_loading: bool,
}

pub struct App {
    pub sessions: Vec<ChatSession>,
    pub active_session: usize,
    pub next_session_id: usize,
    pub input: String,
    pub cursor_position: usize,
    pub status: String,
    pub should_quit: bool,
    pub free_models: Vec<String>,
    pub theme: Theme,
    pub popup: Popup,
    pub system_prompt: String,
    pub screen: Screen,
    pub tick: u64,
    pub api_keys: std::collections::HashMap<&'static str, String>,
    pub pending_tool_run: Option<PendingToolRun>,
    pub skills: Vec<crate::skills::skill::Skill>,
}

impl App {
    pub fn session(&self) -> &ChatSession {
        &self.sessions[self.active_session]
    }
    pub fn session_mut(&mut self) -> &mut ChatSession {
        &mut self.sessions[self.active_session]
    }

    pub fn session_by_id(&self, id: usize) -> Option<&ChatSession> {
        self.sessions.iter().find(|s| s.id == id)
    }
    pub fn session_by_id_mut(&mut self, id: usize) -> Option<&mut ChatSession> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    pub fn new_session(&mut self) {
        let base = self.session();
        let new_session = ChatSession {
            id: self.next_session_id,
            messages: Vec::new(),
            model: base.model.clone(),
            provider: base.provider,
            tool_log: Vec::new(),
            scroll: 0,
            is_loading: false,
        };
        self.next_session_id += 1;
        self.sessions.push(new_session);
        self.active_session = self.sessions.len() - 1;
    }

    pub fn goto_session(&mut self, index: usize) {
        if index < self.sessions.len() {
            self.active_session = index;
        }
    }

    pub fn open_skills_popup(&mut self) {
        self.popup = Popup::SelectSkill {
            skills: self.skills.clone(),
            selected: 0,
        };
    }

    pub fn get_skill_by_name(&self, name: &str) -> Option<&crate::skills::skill::Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

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
            sessions: vec![ChatSession {
                id: 0,
                messages: Vec::new(),
                model,
                provider: &PROVIDERS[0],
                tool_log: Vec::new(),
                scroll: 0,
                is_loading: false,
            }],
            active_session: 0,
            next_session_id: 1,
            input: String::new(),
            cursor_position: 0,
            status: String::from("ready"),
            should_quit: false,
            free_models,
            popup: Popup::None,
            system_prompt: String::from("You are NiNi, a helpful AI assistant inside a TUI"),
            screen,
            api_keys,
            pending_tool_run: None,
            tick: 0,
            theme: Theme::load(),
            skills: Vec::new(),
        }
    }

    pub fn push_tool_event(&mut self, event: crate::tools::ToolEvent) {
        match event {
            crate::tools::ToolEvent::Started {
                call_id,
                tool_name,
                args_summary,
            } => {
                self.session_mut().tool_log.push(ToolLogEntry {
                    call_id,
                    name: tool_name,
                    args_summary,
                    status: ToolLogStatus::Running,
                });
            }
            crate::tools::ToolEvent::Finished {
                call_id,
                result,
                duration_ms,
            } => {
                if let Some(entry) = self
                    .session_mut()
                    .tool_log
                    .iter_mut()
                    .find(|e| e.call_id == call_id)
                {
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
            Popup::SelectProvider { selected } => {
                let len = PROVIDERS.len();
                if *selected == 0 {
                    *selected = len - 1;
                } else {
                    *selected -= 1;
                }
            }
            Popup::SelectModel { selected, .. } => {
                *selected = selected.saturating_sub(1);
            }
            Popup::SelectSkill { selected, .. } => {
                *selected = selected.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn popup_down(&mut self) {
        match &mut self.popup {
            Popup::SelectProvider { selected } => {
                let len = PROVIDERS.len();
                *selected = (*selected + 1) % len;
            }
            Popup::SelectModel {
                models, selected, ..
            } if !models.is_empty() && *selected < models.len() - 1 => {
                *selected += 1;
            }
            Popup::SelectSkill { skills, selected }
                if !skills.is_empty() && *selected < skills.len() - 1 =>
            {
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
