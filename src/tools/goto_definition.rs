use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolContext};

pub struct GotoDefinition;

#[async_trait]
impl Tool for GotoDefinition {
    fn name(&self) -> &str {
        "goto_definition"
    }

    fn description(&self) -> &str {
        "Use this when you already know a file and line where a symbol is USED (a call site), \
         and want to jump to where it's actually declared. If you don't yet know which file \
         contains the usage, use find_symbol instead."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "File path, relative to the project root" },
                "line": { "type": "integer", "description": "Line number, 1-indexed (as shown in the editor)" },
                "character": { "type": "integer", "description": "Column number, 1-indexed" }
            },
            "required": ["file", "line", "character"]
        })
    }

    async fn run(&self, _args: Value) -> Result<String, String> {
        Err(
            "goto_definition must be dispatched via run_goto_definition (needs ToolContext)"
                .to_string(),
        )
    }
}

/// Real implementation, called directly from exec.rs with access to ctx.
pub async fn run_goto_definition(ctx: &ToolContext, args: Value) -> Result<String, String> {
    let file = args["file"].as_str().ok_or("missing 'file'")?;
    let line = args["line"].as_u64().ok_or("missing 'line'")? as u32;
    let character = args["character"].as_u64().ok_or("missing 'character'")? as u32;

    let path = crate::tools::sandbox::resolve_within_root(file)?;

    let lsp_line = line.saturating_sub(1);
    let lsp_char = character.saturating_sub(1);

    // Discover what language and takes the right lsp
    let (lang_id, lsp_arc) = {
        let mut manager = ctx.lsp_manager.lock().await;
        let (lang, _cfg) = crate::lsp::config::LspSettings::load()
            .find_for_file(&path)
            .map(|(l, c)| (l.to_string(), c.clone()))
            .ok_or_else(|| {
                format!(
                    "Nenhum LSP configurado para {:?}. Adicione em ~/.config/nini/lsp.toml",
                    path.extension()
                )
            })?;
        let client = manager
            .get_or_start(&path)
            .await
            .map_err(|e| e.to_string())?;
        (lang, client)
    };

    // Read the archive and send to LSP
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut client = lsp_arc.lock().await;
    client
        .did_open(&path, &lang_id, &text)
        .await
        .map_err(|e| e.to_string())?;

    // Call LSP
    let result = client
        .goto_definition(&path, lsp_line, lsp_char)
        .await
        .map_err(|e| e.to_string())?;

    format_definition_result(result)
}

fn format_definition_result(result: Value) -> Result<String, String> {
    if result.is_null() {
        return Ok("No definition found at that position.".to_string());
    }
    let loc = if result.is_array() {
        result.get(0).cloned().unwrap_or(Value::Null)
    } else {
        result
    };
    let uri = loc["uri"].as_str().unwrap_or("?");
    let line = loc["range"]["start"]["line"].as_u64().unwrap_or(0) + 1;
    let character = loc["range"]["start"]["character"].as_u64().unwrap_or(0) + 1;

    Ok(format!("Defined in {uri}, line {line}, column {character}"))
}
