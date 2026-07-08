use crate::models::{ChatResponse, KeyResponse, Message, ModelsResponse};
use anyhow::{Context, Result};

pub const DAILY_LIMIT_HINT: &str = "limit: 50/day with no credits purchased, 1000/day with $10+ purchased";

pub enum ProviderError {
    RateLimited,
    Other(anyhow::Error),
}

async fn send_chat_to_provider(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    history: &[Message],
) -> Result<String, ProviderError> {
    let body = serde_json::json!({
        "model": model,
        "messages": history,
    });

    let resp = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ProviderError::Other(e.into()))?;

    if resp.status().as_u16() == 429 {
        return Err(ProviderError::RateLimited);
    }

    let chat: ChatResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Other(e.into()))?;

    Ok(chat
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default())
}

fn map_provider_err(e: ProviderError) -> anyhow::Error {
    match e {
        ProviderError::RateLimited => anyhow::anyhow!("provider hit its rate limit"),
        ProviderError::Other(err) => err,
    }
}

/// Default chat flow: tries OpenCode Zen first (if key set) as a safety net,
/// falls back to OpenRouter with the given model on rate limit/failure.
/// Skips the OpenRouter fallback entirely if openrouter_key is empty, to avoid
/// a guaranteed 401 when only OpenCode was configured.
pub async fn send_chat(
    client: &reqwest::Client,
    opencode_key: Option<&str>,
    openrouter_key: &str,
    model: &str,
    history: &[Message],
) -> Result<(String, Option<String>)> {
    if let Some(oc_key) = opencode_key {
        match send_chat_to_provider(client, "https://opencode.ai/zen/v1", oc_key, "big-pickle", history).await {
            Ok(reply) => return Ok((reply, None)),
            Err(err) => {
                if openrouter_key.is_empty() {
                    return Err(map_provider_err(err));
                }
                let notice = match err {
                    ProviderError::RateLimited => "OpenCode Zen rate-limited, used OpenRouter",
                    ProviderError::Other(_) => "OpenCode Zen failed, used OpenRouter",
                };
                let reply = send_chat_to_provider(client, "https://openrouter.ai/api/v1", openrouter_key, model, history)
                    .await
                    .map_err(map_provider_err)?;
                return Ok((reply, Some(notice.to_string())));
            }
        }
    }

    if openrouter_key.is_empty() {
        anyhow::bail!("no API key configured for any provider");
    }

    let reply = send_chat_to_provider(client, "https://openrouter.ai/api/v1", openrouter_key, model, history)
        .await
        .map_err(map_provider_err)?;
    Ok((reply, None))
}

/// Used when the user explicitly picked OpenCode Zen via the /models popup.
/// No automatic fallback here — if it fails, the error is surfaced directly.
pub async fn send_chat_opencode_direct(
    client: &reqwest::Client,
    opencode_key: Option<&str>,
    model: &str,
    history: &[Message],
) -> Result<(String, Option<String>)> {
    let key = opencode_key.context("OPENCODE_API_KEY is not set")?;
    let reply = send_chat_to_provider(client, "https://opencode.ai/zen/v1", key, model, history)
        .await
        .map_err(map_provider_err)?;
    Ok((reply, None))
}

/// Fetches the list of free model IDs available on OpenRouter.
pub async fn fetch_free_model_ids(client: &reqwest::Client) -> Result<Vec<String>> {
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await
        .context("failed to connect to the OpenRouter models API")?;

    let data: ModelsResponse = resp
        .json()
        .await
        .context("failed to parse OpenRouter models response")?;

    let mut ids: Vec<String> = data
        .data
        .into_iter()
        .map(|m| m.id)
        .filter(|id| id.ends_with(":free"))
        .collect();
    ids.sort();
    Ok(ids)
}

/// Fetches the list of model IDs available on OpenCode Zen.
pub async fn fetch_opencode_model_ids(client: &reqwest::Client, api_key: &str) -> Result<Vec<String>> {
    let resp = client
        .get("https://opencode.ai/zen/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("failed to connect to the OpenCode Zen models API")?;

    let data: ModelsResponse = resp
        .json()
        .await
        .context("failed to parse OpenCode Zen models response")?;

    let mut ids: Vec<String> = data.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    Ok(ids)
}

/// Fetches how many requests have already been used today on the OpenRouter key.
/// Returns an error immediately if no key is set, instead of calling the API with an empty key.
pub async fn get_usage_daily(client: &reqwest::Client, api_key: &str) -> Result<Option<f64>> {
    if api_key.is_empty() {
        anyhow::bail!("OpenRouter API key is not set");
    }

    let resp = client
        .get("https://openrouter.ai/api/v1/key")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("failed to connect to the key/usage API")?;

    let data: KeyResponse = resp
        .json()
        .await
        .context("failed to parse usage response")?;

    Ok(data.data.usage_daily)
}
