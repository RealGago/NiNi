use super::{resolve_within_root, Tool};

pub struct EditFile;

#[async_trait::async_trait]
impl Tool for EditFile {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Replaces an exact, unique occurrence of text in a file (must be inside the project directory) with new text. Fails if the text is not found or appears more than once."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_str": {
                    "type": "string",
                    "description": "Exact text to find and replace. Must appear exactly once in the file."
                },
                "new_str": {
                    "type": "string",
                    "description": "Text to replace old_str with"
                }
            },
            "required": ["path", "old_str", "new_str"]
        })
    }

    async fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("missing or invalid 'path' argument")?;
        let old_str = args["old_str"]
            .as_str()
            .ok_or("missing or invalid 'old_str' argument")?;
        let new_str = args["new_str"]
            .as_str()
            .ok_or("missing or invalid 'new_str' argument")?;
        let resolved = resolve_within_root(path)?;

        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| format!("failed to read {}: {}", path, e))?;

        let matches = content.matches(old_str).count();
        if matches == 0 {
            return Err(format!("old_str not found in {}", path));
        }
        if matches > 1 {
            return Err(format!(
                "old_str appears {} times in {}, must be unique",
                matches, path
            ));
        }

        let new_content = content.replacen(old_str, new_str, 1);

        tokio::fs::write(&resolved, &new_content)
            .await
            .map_err(|e| format!("failed to write {}: {}", path, e))?;

        Ok(format!("edited {}", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_edit_file() {
        let path = "nini3_test_edit_tmp.txt";
        tokio::fs::write(path, "hello world").await.unwrap();

        let tool = EditFile;
        let args = serde_json::json!({
            "path": path,
            "old_str": "world",
            "new_str": "nini3"
        });
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "hello nini3");

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn test_edit_file_not_unique() {
        let path = "nini3_test_edit_dup_tmp.txt";
        tokio::fs::write(path, "foo foo").await.unwrap();

        let tool = EditFile;
        let args = serde_json::json!({
            "path": path,
            "old_str": "foo",
            "new_str": "bar"
        });
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.is_err());

        let _ = tokio::fs::remove_file(path).await;
    }
}
