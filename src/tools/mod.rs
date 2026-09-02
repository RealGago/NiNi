use async_trait::async_trait;

#[derive(Clone)]
pub struct ToolContext {
    pub client: reqwest::Client,
    pub api_keys: std::collections::HashMap<&'static str, String>,
    pub model: String,
    pub subagent_tool_defs: Vec<crate::models::ToolDefinition>,
}

#[async_trait]
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    async fn run(&self, args: serde_json::Value) -> Result<String, String>;
}

pub enum ToolEvent {
    Started { call_id: String, tool_name: String, args_summary: String },
    Finished { call_id: String, result: Result<String, String>, duration_ms: u128 },
}

mod read_file;
mod list_directory;
mod write_file;
mod edit_file;
mod grep;
mod run_command;
mod exec;
mod sandbox;
mod spawn_subagent;
pub use read_file::ReadFile;
pub use list_directory::ListDirectory;
pub use write_file::WriteFile;
pub use edit_file::EditFile;
pub use grep::Grep;
pub use run_command::RunCommand;
pub use exec::{execute_tool_call, execute_tool_batch, continue_after_confirmation, ToolBatchOutcome};
pub use sandbox::resolve_within_root;
pub use spawn_subagent::SpawnSubagent;
