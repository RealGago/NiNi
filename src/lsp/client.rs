use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};

use super::config::LspConfig;
use super::protocol::{read_message, write_message};

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

pub struct LspClient {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: i64,
    pending: PendingMap,
}

impl LspClient {
    pub async fn start(cmd: &str, root: &Path) -> Result<Self> {
        let config = LspConfig {
            command: cmd.to_string(),
            args: vec![],
            extensions: vec![],
            env: HashMap::new(),
            root_markers: vec![],
        };
        Self::start_with_config(&config, root).await
    }

    pub async fn start_with_config(config: &LspConfig, root: &Path) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Applies environment variables :(
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!(
                "Failed to start LSP '{}': {}. Check if it is installed and in the PATH.",
                config.command,
                e
            )
        })?;

        let stdin = Arc::new(Mutex::new(child.stdin.take().expect("stdin piped")));
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout piped"));

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Background task: owns stdout, and is the ONLY thing that reads
        // from the server. Every incoming message gets routed by id.
        let reader_pending = pending.clone();
        let reader_stdin = stdin.clone();
        tokio::spawn(async move {
            loop {
                let msg = match read_message(&mut stdout).await {
                    Ok(m) => m,
                    Err(_) => break, // server died / closed stdout
                };

                let has_method = msg.get("method").is_some();
                let id = msg.get("id").and_then(|v| v.as_i64());

                match (id, has_method) {
                    (Some(id), true) => {
                        // Server -> client REQUEST (e.g. window/workDoneProgress/create).
                        // We don't implement any of these features; ack with a null
                        // result so rust-analyzer doesn't sit there waiting on us.
                        let ack = json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null });
                        let mut stdin = reader_stdin.lock().await;
                        let _ = write_message(&mut stdin, &ack).await;
                    }
                    (Some(id), false) => {
                        // Response to one of OUR requests.
                        let mut map = reader_pending.lock().await;
                        if let Some(tx) = map.remove(&id) {
                            let _ = tx.send(msg);
                        }
                    }
                    (None, _) => {
                        // Notification (e.g. $/progress, publishDiagnostics) — ignored for now.
                    }
                }
            }
        });

        let mut me = Self {
            child,
            stdin,
            next_id: 0,
            pending,
        };
        me.initialize(root).await?;
        Ok(me)
    }

    fn next_id(&mut self) -> i64 {
        self.next_id += 1;
        self.next_id
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        {
            let mut stdin = self.stdin.lock().await;
            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params,
                }),
            )
            .await?;
        }

        let resp = rx.await.map_err(|_| {
            anyhow::anyhow!("LSP server closed connection before responding to '{method}'")
        })?;
        if let Some(err) = resp.get("error") {
            anyhow::bail!("LSP error on '{method}': {err}");
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
            }),
        )
        .await
    }

    async fn initialize(&mut self, root: &Path) -> Result<()> {
        let root_uri = format!("file://{}", root.display());
        self.send_request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {}
            }),
        )
        .await?;
        self.send_notification("initialized", json!({})).await
    }

    pub async fn did_open(&mut self, path: &Path, language_id: &str, text: &str) -> Result<()> {
        let uri = format!("file://{}", path.display());
        self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await
    }

    pub async fn goto_definition(
        &mut self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Value> {
        let uri = format!("file://{}", path.display());
        self.send_request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    pub async fn workspace_symbol(&mut self, query: &str) -> Result<Value> {
        self.send_request(
            "workspace/symbol",
            json!({
                "query": query
            }),
        )
        .await
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
