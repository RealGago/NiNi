use crate::tools::resolve_within_root;

use super::Tool;

pub struct ListDirectory;

#[async_trait::async_trait]
impl Tool for ListDirectory {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "Lists files and directories at the given path"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
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

        let mut entries = tokio::fs::read_dir(&resolved)
            .await
            .map_err(|e| format!("failed to read directory {}: {}", path, e))?;

        let mut result = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("failed to read entry: {}", e))?
        {
            let file_type = entry
                .file_type()
                .await
                .map_err(|e| format!("failed to get file type: {}", e))?;

            let marker = if file_type.is_dir() { "/" } else { "" };
            result.push(format!("{}{}", entry.file_name().to_string_lossy(), marker));
        }

        Ok(result.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_directory() {
        let tool = ListDirectory;
        let args = serde_json::json!({ "path": "." });
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
