use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize)]
pub struct Choice {
    pub message: MessageContent,
}

#[derive(Deserialize)]
pub struct MessageContent {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
pub struct ModelEntry {
    pub id: String,
}

#[derive(Deserialize)]
pub struct KeyResponse {
    pub data: KeyData,
}

#[derive(Deserialize)]
pub struct KeyData {
    pub usage_daily: Option<f64>,
}
