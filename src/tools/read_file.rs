use super::{resolve_within_root, Tool};

pub struct ReadFile;

#[async_trait::async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }
    fn description(&self) -> &str {
        "Reads the contents of a file at the given path (must be inside the project directory)"
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
