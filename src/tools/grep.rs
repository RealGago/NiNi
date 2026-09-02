use crate::tools::resolve_within_root;

use super::Tool;
use std::path::Path;

pub struct Grep;

#[async_trait::async_trait]
impl Tool for Grep {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Recursively searches for a literal text pattern in files under a directory, returning matching file paths and line numbers"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to search recursively"
                },
                "pattern": {
                    "type": "string",
                    "description": "Literal text to search for"
                }
            },
            "required": ["path", "pattern"]
        })
    }

    async fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args["path"]
            .as_str()
            .ok_or("missing or invalid 'path' argument")?;
        let pattern = args["pattern"]
            .as_str()
            .ok_or("missing or invalid 'pattern' argument")?;
        let resolved = resolve_within_root(path)?;

        let mut results = Vec::new();
        search_dir(&resolved, pattern, &mut results).await?;

        if results.is_empty() {
            Ok("no matches found".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

#[async_recursion::async_recursion]
async fn search_dir(
    dir: &Path,
    pattern: &str,
    results: &mut Vec<String>,
) -> Result<(), String> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| format!("failed to read directory {}: {}", dir.display(), e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("failed to read entry: {}", e))?
    {
        let entry_path = entry.path();

        // skip common noise directories
        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }
        }

        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("failed to get file type: {}", e))?;

        if file_type.is_dir() {
            search_dir(&entry_path, pattern, results).await?;
        } else if file_type.is_file() {
            if let Ok(content) = tokio::fs::read_to_string(&entry_path).await {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(pattern) {
                        results.push(format!(
                            "{}:{}: {}",
                            entry_path.display(),
                            i + 1,
                            line.trim()
                        ));
                    }
                }
            }
            // silently skip files that aren't valid UTF-8 (binaries, etc.)
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grep() {
        let dir = "nini_test_grep_tmp";
        tokio::fs::create_dir_all(dir).await.unwrap();
        tokio::fs::write(format!("{}/a.txt", dir), "hello world\nfoo bar")
            .await
            .unwrap();
        tokio::fs::write(format!("{}/b.txt", dir), "no match here")
            .await
            .unwrap();

        let tool = Grep;
        let args = serde_json::json!({ "path": dir, "pattern": "hello" });
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("a.txt"));

        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}
