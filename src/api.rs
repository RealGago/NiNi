use crate::models::{ChatResponse, KeyResponse, Message, ModelsResponse, ToolDefinition};
use anyhow::{Context, Result};
use crate::app::Provider;

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
    tools: &[ToolDefinition],
) -> Result<ChatResponse, ProviderError> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": history,
    });

    if !tools.is_empty() {
        body["tools"] = serde_json::to_value(tools).map_err(|e| ProviderError::Other(e.into()))?;
    }

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

    resp.json::<ChatResponse>()
        .await
        .map_err(|e| ProviderError::Other(e.into()))
}
  
fn map_provider_err(e: ProviderError) -> anyhow::Error {
    match e {
        ProviderError::RateLimited => anyhow::anyhow!("provider hit its rate limit"),
        ProviderError::Other(err) => err,
    }
}

pub async fn send_chat(
    client: &reqwest::Client,
    api_keys: &std::collections::HashMap<&'static str, String>,
    model: &str,
    history: &[Message],
    tools: &[ToolDefinition],
) -> Result<(ChatResponse, Option<String>)> {
    let ordered = crate::providers::available_providers_in_fallback_order(api_keys);

    if ordered.is_empty() {
          anyhow::bail!("no API key configured for any provider");
    }

    let mut last_err = None;
    for (i, provider) in ordered.iter().enumerate() {
        let key = &api_keys[provider.key_env];
        match send_chat_to_provider(client, provider.base_url, key, model, history, tools).await {
            Ok(reply) => {
                let notice = if i == 0 {
                    None
                } else {
                    Some(format!("{} failed, used {}", ordered[i - 1].label, provider.label))
                };
                return Ok((reply, notice));
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(map_provider_err(last_err.unwrap()))
}

   


// a guaranteed 401 when only OpenCode was configured.
/// Fetches the list of free model IDs available on OpenRouter.
pub async fn fetch_model_ids(client: &reqwest::Client, provider: Provider, api_key: Option<&str>) -> Result<Vec<String>> {
    let mut req = client.get(format!("{}/models", provider.base_url));
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req.send().await.context("failed to connect to models API")?;
    let data: ModelsResponse = resp.json().await.context("failed to parse models response")?;

    let mut ids: Vec<String> = data.data.into_iter().map(|m| m.id).collect();
    if provider.filter_free {
        ids.retain(|id| id.ends_with(":free"));
    }
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


