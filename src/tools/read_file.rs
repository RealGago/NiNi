use super::{Tool, resolve_within_root};

pub struct ReadFile;

#[async_trait::async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Reads the full contents of a file. For finding where a symbol is defined, \
        prefer find_symbol or goto_definition, which are cheaper and more precise. \
        Use this when you need the actual code, for example after an LSP tool \
        tells you which file and line to look at."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }
    async fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("missing or invalid 'path' argument")?;
        let resolved = resolve_within_root(path)?;
        tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| format!("failed to read {}: {}", path, e))
    }
}
