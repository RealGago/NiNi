use crate::models::{Message, ToolCall};
use super::{Tool, ToolEvent, ToolContext};
use tokio::sync::mpsc::UnboundedSender;

pub async fn execute_tool_call(
    call: &ToolCall,
    ctx: &ToolContext,
    events: &UnboundedSender<ToolEvent>,
) -> Message {
    let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
        .unwrap_or(serde_json::Value::Null);

    let _ = events.send(ToolEvent::Started { 
        call_id: call.id.clone(), 
        tool_name: call.function.name.clone(), 
        args_summary: call.function.arguments.clone(), 
    });

    let start = std::time::Instant::now();

    let result: Result<String, String> = match call.function.name.as_str() {
        "read_file" => super::ReadFile.run(args).await,
        "list_directory" => super::ListDirectory.run(args).await,
        "write_file" => super::WriteFile.run(args).await,
        "edit_file" => super::EditFile.run(args).await,
        "grep" => super::Grep.run(args).await,
        "run_command" => super::RunCommand.run(args).await,
        "spawn_subagent" => {
            let task = args["task"].as_str().unwrap_or("").to_string();
            super::spawn_subagent::run_subagent_task(ctx, task, events).await
        }
        other => Err(format!("unknow tool: {}", other)),
    };

    let duration_ms = start.elapsed().as_millis();
    let _ = events.send(ToolEvent::Finished { 
        call_id: call.id.clone(),
        result: result.clone(), 
        duration_ms, 
    });

    let content = match result {
        Ok(output) => output,
        Err(e) => format!("error: {}", e),
    };

    Message {
        role: "tool".to_string(),
        content,
        tool_calls: None,
        tool_call_id: Some(call.id.clone()),
    }
}

pub fn rejected_message(call: &ToolCall) -> Message {
    Message {
        role: "tool".to_string(),
        content: "command rejected by user".to_string(),
        tool_calls: None,
        tool_call_id: Some(call.id.clone()),
    }
}

pub enum ToolBatchOutcome {
    Done(Vec<Message>),
    NeedsConfirmation {
        call: ToolCall,
        command: String,
        results_so_far: Vec<Message>,
        remaining: Vec<ToolCall>,
    },
}

/// Runs tool calls in order, stopping (without running it) at the first
/// `run_command`, so the caller can ask the user for confirmation first.
pub async fn execute_tool_batch(
    calls: &[ToolCall],
    ctx: &ToolContext,
    events: &UnboundedSender<ToolEvent>,
) -> ToolBatchOutcome {
    execute_tool_batch_from(Vec::new(), calls, ctx, events).await
}

async fn execute_tool_batch_from(
    mut results: Vec<Message>,
    calls: &[ToolCall],
    ctx: &ToolContext,
    events: &UnboundedSender<ToolEvent>,
) -> ToolBatchOutcome {
    if let Some(idx) = calls.iter().position(|c| c.function.name == "run_command") {
        // parallelize everything before the run_command, then pause
        let futures = calls[..idx].iter().map(|c| execute_tool_call(c, ctx, events));
        results.extend(futures::future::join_all(futures).await);

        let call = &calls[idx];
        let args: serde_json::Value =
            serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::Value::Null);
        let command = args["command"].as_str().unwrap_or("").to_string();

            let _ = events.send(ToolEvent::Started { 
                call_id: call.id.clone(), 
                tool_name: call.function.name.clone(), 
                args_summary: call.function.arguments.clone(), 
            });

            ToolBatchOutcome::NeedsConfirmation {
                call: call.clone(),
                command,
                results_so_far: results,
                remaining: calls[idx + 1..].to_vec(),
            }
        } else {
        // no run_command in this batch: everything can run in parallel
        let futures = calls.iter().map(|c| execute_tool_call(c, ctx, events));
        results.extend(futures::future::join_all(futures).await);
        ToolBatchOutcome::Done(results)
      }
}

/// Called after the user answers the confirmation popup — runs (or rejects)
/// the pending `run_command`, then keeps processing whatever tool calls remained.
pub async fn continue_after_confirmation(
    call: ToolCall,
    approved: bool,
    mut results_so_far: Vec<Message>,
    remaining: Vec<ToolCall>,
    ctx: &ToolContext,
    events: &UnboundedSender<ToolEvent>,
) -> ToolBatchOutcome {
    if approved {
        results_so_far.push(execute_tool_call(&call, ctx, events).await);
    } else {
        results_so_far.push(rejected_message(&call));
    }
    execute_tool_batch_from(results_so_far, &remaining, ctx, events).await
}
