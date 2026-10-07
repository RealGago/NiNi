use async_trait::async_trait;
use serde_json::{Value, json};

use super::{Tool, ToolContext};

pub struct FindSymbol;

#[async_trait]
impl Tool for FindSymbol {
    fn name(&self) -> &str {
        "find_symbol"
    }

    fn description(&self) -> &str {
        "Use this FIRST when asked where a function, struct, or symbol is defined anywhere \
         in the project, without already knowing which file it's in. Searches the whole \
         project's real compiled symbol table (via rust-analyzer) by name — faster and more \
         accurate than grep, since grep only matches text and can't tell a definition from a \
         call site or a comment. Prefer this over grep/read_file for 'where is X defined' \
         or 'find the definition of X' questions."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Name of the function/struct/symbol to find" }
            },
            "required": ["name"]
        })
    }

    async fn run(&self, _args: Value) -> Result<String, String> {
        Err("find_symbol must be dispatched via run_find_symbol (needs ToolContext)".to_string())
    }
}

pub async fn run_find_symbol(ctx: &ToolContext, args: Value) -> Result<String, String> {
    let name = args["name"].as_str().ok_or("missing 'name'")?;
    eprintln!("[find_symbol] Looking for: {}", name); // ← LOG
    // Opt 1: use 'file' if provided
    // Opt 2: it attempts to detect the language through the current project
    let path = if let Some(file) = args["file"].as_str() {
        crate::tools::sandbox::resolve_within_root(file)?
    } else {
        // Detects by the most common project file (Cargo.toml, go.mod, etc.)
        let root = std::env::current_dir().map_err(|e| e.to_string())?;
        detected_project_file(&root)?
    };
    let lsp_arc = {
        let mut manager = ctx.lsp_manager.lock().await;
        manager
            .get_or_start(&path)
            .await
            .map_err(|e| e.to_string())?
    };

    let mut client = lsp_arc.lock().await;
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let main_file = root.join("src/main.rs");
    if main_file.exists() {
        if let Ok(text) = std::fs::read_to_string(&main_file) {
            eprintln!(
                "[find_symbol] Opening {} to trigger indexing",
                main_file.display()
            );
            let _ = client.did_open(&main_file, "rust", &text).await;
            // wait rust-analyzer process
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    eprintln!("[find_symbol] Using path: {:?}", path); // ← LOG
    eprintln!("[find_symbol] LSP started"); // ← LOG
    let result = client
        .workspace_symbol(name)
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("[find_symbol] Raw result: {:?}", result); // ← LOG
    format_symbol_result(result)
}

// Detect which LSP to use based on project files in the root directory.
fn detected_project_file(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let markers = [
        ("Cargo.toml", "src/main.rs"),
        ("go.mod", "fake.go"),
        ("package.json", "fake.ts"),
        ("pyproject.toml", "fake.py"),
        ("setup.py", "fake.py"),
        ("mix.exs", "fake.ex"),
        ("build.zig", "fake.zig"),
    ];
    for (marker, real_file) in markers {
        if root.join(marker).exists() {
            let path = root.join(real_file);
            if path.exists() {
                return Ok(path);
            }
            return Ok(root.join(marker));
        }
    }
    Err("The project language could not be detected. Please pass the `file` parameter.".to_string())
}
fn format_symbol_result(result: Value) -> Result<String, String> {
    let symbols = result.as_array().cloned().unwrap_or_default();
    if symbols.is_empty() {
        return Ok("No matching symbol found.".to_string());
    }

    let mut out = String::new();
    for sym in symbols.iter().take(5) {
        let name = sym["name"].as_str().unwrap_or("?");
        let uri = sym["location"]["uri"].as_str().unwrap_or("?");
        let line = sym["location"]["range"]["start"]["line"]
            .as_u64()
            .unwrap_or(0)
            + 1;
        out.push_str(&format!("{name} — {uri}, line {line}\n"));
    }
    Ok(out.trim_end().to_string())
}
