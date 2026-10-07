/// Static configuration for a single LLM provider.
/// Adding a new provider means adding one entry to `PROVIDERS` below —
/// no other file needs to change.
pub struct ProviderConfig {
    /// Short name, used in status messages ("fetching {} models...").
    pub label: &'static str,
    /// Longer, descriptive name shown in the popup list.
    pub display_name: &'static str,    /// Base URL, without trailing slash. `/models` and `/chat/completions`
    /// are appended by the caller.
    pub base_url: &'static str,
    /// Env var that holds this provider's API key.
    pub key_env: &'static str,
    /// If true, `/models` results are filtered to ids ending in ":free".
    pub filter_free: bool,
    /// Lower tries first when falling back between providers.
    pub fallback_priority: u8,
}

/// A `Provider` is just a reference into `PROVIDERS`. `'static` + `Copy`
/// means it behaves like the old enum (cheap to pass around, no lifetime headaches), 
/// but the identity now lives in data, not in code :)
pub type Provider = &'static ProviderConfig;

pub const PROVIDERS: &[ProviderConfig] = &[
    ProviderConfig {
        label: "OpenRouter",
        display_name: "OpenRouter (Free Models)",
        base_url: "https://openrouter.ai/api/v1",
        key_env: "OPENROUTER_API_KEY",
        filter_free: true,
        fallback_priority: 1,
    },
    ProviderConfig {
        label: "OpenCode Zen",
        display_name: "OpenCode Zen",
        base_url: "https://opencode.ai/zen/v1",
        key_env: "OPENCODE_API_KEY",
        filter_free: false,
        fallback_priority: 0,
    },
];
/// Providers ordered by fallback priority (lowest first), skipping any
/// whose key env var isn't set.
pub fn available_providers_in_fallback_order(
    api_keys: &std::collections::HashMap<&'static str, String>,
) -> Vec<Provider> {
    let mut providers: Vec<Provider> = PROVIDERS
        .iter()
        .filter(|p| api_keys.contains_key(p.key_env))
        .collect();
    providers.sort_by_key(|p| p.fallback_priority);
    providers
}
