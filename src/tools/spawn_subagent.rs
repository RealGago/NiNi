use crate::models::{Message, ToolDefinition};
use crate::api;
use super::{Tool, ToolEvent, ToolContext, exec::execute_tool_batch, ToolBatchOutcome};
use tokio::sync::mpsc::UnboundedSender;
use async_trait::async_trait;
use serde_json::json;

pub struct SpawnSubagent;

#[async_trait]
impl Tool for SpawnSubagent {
    fn name(&self) -> &str { "spawn_subagent" }

    fn description(&self) -> &str {
        "Delegate an independent task to a subagent that runs in parallel. \
     Use when the overall task can be split into parts that do NOT depend \
     on each other's results. \
     \
     IMPORTANT: each subagent should stay strictly within the scope of its \
     assigned task. Do NOT create project scaffolding (e.g. pubspec.yaml, \
     package.json, a fresh main.dart/main.py, README files) unless the task \
     explicitly asks for a full project setup — subagents commonly duplicate \
     this scaffolding when run in parallel, causing write conflicts on \
     shared files. If the task is 'create a login screen widget', deliver \
     just that widget file, not an entire app skeleton around it. \
     \
     Do NOT use spawn_subagent if one part needs another part's output, or \
     if two subtasks would need to write to the same file — handle those \
     sequentially in the main agent instead."    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "Full, self-contained task description — the subagent does not see the main conversation's history." }
            },
            "required": ["task"]
        })
    }

    // Never actually called: spawn_subagent is special-cased in exec.rs
    // because it needs ToolContext (client/api_keys/model), which the
    // generic Tool::run signature doesn't carry.
    async fn run(&self, _args: serde_json::Value) -> Result<String, String> {
        Err("spawn_subagent must be dispatched via exec.rs, not Tool::run".into())
    }
}

/// The actual subagent loop. Called directly from exec.rs with a ToolContext.
pub async fn run_subagent_task(
    ctx: &ToolContext,
    task: String,
    events: &UnboundedSender<ToolEvent>,
) -> Result<String, String> {
    println!("[subagent started] task: {}", task);
    let mut history = vec![Message::user(task.clone())];

    loop {
        let res = api::send_chat(&ctx.client, &ctx.api_keys, &ctx.model, &history, &ctx.subagent_tool_defs)
            .await
            .map_err(|e| e.to_string())?;

        let (chat, _notice) = api::send_chat(&ctx.client, &ctx.api_keys, &ctx.model, &history, &ctx.subagent_tool_defs)
            .await
            .map_err(|e| e.to_string())?;

        let choice = chat.choices.first().ok_or("no choice returned")?;

        if let Some(calls) = &choice.message.tool_calls {
            history.push(Message {
                role: "assistant".to_string(),
                content: choice.message.content.clone().unwrap_or_default(),
                tool_calls: Some(calls.clone()),
                tool_call_id: None,
            });

            match execute_tool_batch(calls, ctx, events).await {
                ToolBatchOutcome::Done(results) => {
                    history.extend(results);
                    continue;
                }
                ToolBatchOutcome::NeedsConfirmation { .. } => {
                    // run_command isn't in ctx.subagent_tool_defs, so this
                    // branch should be unreachable in practice.
                    return Err("subagent attempted run_command — not supported".into());
                }
            }
        } else {
            println!("[subagent started] task: {}", task);
            return Ok(choice.message.content.clone().unwrap_or_default());
        }
        
    }
}
