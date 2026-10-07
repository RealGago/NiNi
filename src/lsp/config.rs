use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct LspConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub root_markers: Vec<String>, // e.g: ["Cargo.toml", "go.mod"]
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LspSettings {
    #[serde(default)]
    pub lsp: HashMap<String, LspConfig>,
}

impl LspSettings {
    /// Load from `~/.config/nini/lsp.toml` and `./.nini/lsp.toml`
    pub fn load() -> Self {
        let mut settings = LspSettings::default();

        // Global
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("nini/lsp.toml");
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(parsed) = toml::from_str::<LspSettings>(&content) {
                    settings.lsp.extend(parsed.lsp);
                }
            }
        }

        // Project (overwrite global)
        let project_path = Path::new(".nini/lsp.toml");
        if let Ok(content) = std::fs::read_to_string(project_path) {
            if let Ok(parsed) = toml::from_str::<LspSettings>(&content) {
                settings.lsp.extend(parsed.lsp);
            }
        }

        settings
    }

    // Find the LSP that covers the file extension
    pub fn find_for_file(&self, path: &Path) -> Option<(&str, &LspConfig)> {
        let ext = path.extension()?.to_str()?;
        let ext_with_dot = format!(".{}", ext);

        self.lsp
            .iter()
            .find(|(_, cfg)| cfg.extensions.iter().any(|e| e == &ext_with_dot))
            .map(|(name, cfg)| (name.as_str(), cfg))
    }

    /// Lists all configured languages
    pub fn list_languages(&self) -> Vec<&str> {
        self.lsp.keys().map(|s| s.as_str()).collect()
    }
}
