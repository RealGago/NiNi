use super::{resolve_within_root, Tool};

pub struct WriteFile;

#[async_trait::async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Writes content to a file at the given path (must be inside the project directory), creating it if it doesn't exist and overwriting it if it does"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("missing or invalid 'path' argument")?;
        let content = args["content"]
            .as_str()
            .ok_or("missing or invalid 'content' argument")?;
        let resolved = resolve_within_root(path)?;

        tokio::fs::write(&resolved, content)
            .await
            .map_err(|e| format!("failed to write {}: {}", path, e))?;

        Ok(format!("wrote {} bytes to {}", content.len(), path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_file() {
        let tool = WriteFile;
        let args = serde_json::json!({
            "path": "nini3_test_write_tmp.txt",
            "content": "hello from nini3"
        });
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.is_ok());

        let written = tokio::fs::read_to_string("nini3_test_write_tmp.txt")
            .await
            .unwrap();
        assert_eq!(written, "hello from nini3");

        // cleanup
        let _ = tokio::fs::remove_file("nini3_test_write_tmp.txt").await;
    }
}
