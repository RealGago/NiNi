use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Default)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}
impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message {
            role: "user".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Message {
            role: "assistant".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Message {
            role: "system".to_string(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

}

impl ToolDefinition {
    pub fn from_tool(tool: &dyn crate::tools::Tool) -> Self {
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.schema(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
// comes as a STRING containing JSON, not a direct object :)
}

// to send the request, describing which tools exist
#[derive(Serialize, Clone)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: &'static str, // always function
    pub function: FunctionDefinition,
}

#[derive(Serialize, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
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
    pub tool_calls: Option<Vec<ToolCall>>,
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
