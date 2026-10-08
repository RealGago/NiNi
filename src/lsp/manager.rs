use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::LspClient;
use super::config::{LspConfig, LspSettings};

pub struct LspManager {
    clients: HashMap<String, Arc<Mutex<LspClient>>>,
    settings: LspSettings,
}

impl LspManager {
    pub fn new(settings: LspSettings) -> Self {
        Self {
            clients: HashMap::new(),
            settings,
        }
    }

    //  Get (or start) the correct LSP for this file.
    pub async fn get_or_start(&mut self, path: &Path) -> Result<Arc<Mutex<LspClient>>> {
        let (lang_name, config) = self.settings.find_for_file(path).ok_or_else(|| {
            anyhow::anyhow!(
                "Nenhum LSP configurado para arquivos {:?}. \
                 Adicione em ~/.config/nini/lsp.toml",
                path.extension()
            )
        })?;

        if let Some(client) = self.clients.get(lang_name) {
            return Ok(client.clone());
        }

        // Find the project root (search for root_markers)
        let project_root = find_project_root(path, &config.root_markers)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        crate::log::log(&format!(
            "[lsp] Starting {} with root: {:?}",
            config.command, project_root
        ));

        let client = LspClient::start_with_config(config, &project_root).await?;
        let client = Arc::new(Mutex::new(client));
        self.clients.insert(lang_name.to_string(), client.clone());
        Ok(client)
    }

    // List the languages that have LSP configured.
    pub fn available_languages(&self) -> Vec<&str> {
        self.settings.list_languages()
    }

    // List the languages that already have LSP running
    pub fn running_languages(&self) -> Vec<&str> {
        self.clients.keys().map(|s| s.as_str()).collect()
    }
}

fn find_project_root(start: &Path, markers: &[String]) -> Option<std::path::PathBuf> {
    let mut current = start.parent()?;
    loop {
        for marker in markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }
        current = current.parent()?;
    }
}
