use crate::lsp::{self, LspManager};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ToolContext {
    pub client: reqwest::Client,
    pub api_keys: std::collections::HashMap<&'static str, String>,
    pub model: String,
    pub subagent_tool_defs: Vec<crate::agent::models::ToolDefinition>,
    pub lsp_manager: Arc<tokio::sync::Mutex<crate::lsp::LspManager>>,
}

#[async_trait]
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    async fn run(&self, args: serde_json::Value) -> Result<String, String>;
}

pub enum ToolEvent {
    Started {
        call_id: String,
        tool_name: String,
        args_summary: String,
    },
    Finished {
        call_id: String,
        result: Result<String, String>,
        duration_ms: u128,
    },
}

mod edit_file;
mod exec;
mod find_symbol;
mod goto_definition;
mod grep;
mod list_directory;
mod read_file;
mod run_command;
mod sandbox;
mod spawn_subagent;
mod write_file;
pub use edit_file::EditFile;
pub use exec::{
    ToolBatchOutcome, continue_after_confirmation, execute_tool_batch, execute_tool_call,
};
pub use find_symbol::FindSymbol;
pub use goto_definition::GotoDefinition;
pub use grep::Grep;
pub use list_directory::ListDirectory;
pub use read_file::ReadFile;
pub use run_command::RunCommand;
pub use sandbox::resolve_within_root;
pub use spawn_subagent::SpawnSubagent;
pub use write_file::WriteFile;
