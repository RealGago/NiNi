use super::Tool;

pub struct RunCommand;


#[async_trait::async_trait]
impl Tool for RunCommand {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Executes a shell command in the current working directory. Requires user confirmation before running."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    async fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let command = args["command"]
            .as_str()
            .ok_or("missing or invalid 'command' argument")?;

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(".")
            .output()
            .await
            .map_err(|e| format!("failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!(
                "exit code: {}\nstdout:\n{}\nstderr:\n{}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command() {
        let tol = RunCommand;
        let args = serde_json::json!({ "command": "echo hello"});
        let result = tool.run(args).await;
        println!("{:?}", result);
        assert!(result.unwrap().contains("hello"));
    }
}

   
