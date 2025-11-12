use super::types::*;
use super::LlmProvider;
use crate::error::{OllmError, Result};
use crate::types::{ContentBlock, Message, Role, Tool};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;
use tracing::{debug, info};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: usize,
}

impl AnthropicProvider {
    pub fn new(config: &crate::config::Config) -> Result<Self> {
        let api_key = if let Some(ref env_var) = config.llm.api_key_env {
            std::env::var(env_var).map_err(|_| {
                OllmError::Config(format!(
                    "Environment variable {} not set",
                    env_var
                ))
            })?
        } else {
            return Err(OllmError::Config(
                "api_key_env not configured for Anthropic".to_string(),
            ));
        };

        let model = if config.llm.model.is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            config.llm.model.clone()
        };

        Ok(Self {
            client: Client::new(),
            api_key,
            model,
            max_tokens: config.llm.max_tokens,
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<ApiMessage> {
        messages
            .into_iter()
            .filter(|m| m.role != Role::System) // System messages handled separately
            .map(|m| ApiMessage {
                role: match m.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => "user".to_string(), // Fallback
                },
                content: m.content.iter().map(|c| self.convert_content(c)).collect(),
            })
            .collect()
    }

    fn convert_content(&self, content: &ContentBlock) -> ApiContent {
        match content {
            ContentBlock::Text { text } => ApiContent::Text {
                r#type: "text".to_string(),
                text: text.clone(),
            },
            ContentBlock::ToolUse { id, name, input } => ApiContent::ToolUse {
                r#type: "tool_use".to_string(),
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => ApiContent::ToolResult {
                r#type: "tool_result".to_string(),
                tool_use_id: tool_use_id.clone(),
                content: content.clone(),
                is_error: *is_error,
            },
        }
    }

    fn convert_tools(&self, tools: Vec<Tool>) -> Vec<ApiTool> {
        tools
            .into_iter()
            .map(|t| ApiTool {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
            })
            .collect()
    }

    fn parse_response(&self, response: ApiResponse) -> Result<ChatResponse> {
        let content = response
            .content
            .into_iter()
            .map(|c| match c {
                ApiContent::Text { text, .. } => ContentBlock::Text { text },
                ApiContent::ToolUse { id, name, input, .. } => {
                    ContentBlock::ToolUse { id, name, input }
                }
                ApiContent::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                    ..
                } => ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                },
            })
            .collect();

        Ok(ChatResponse {
            content,
            model: response.model,
            stop_reason: response.stop_reason.and_then(|s| match s.as_str() {
                "end_turn" => Some(StopReason::EndTurn),
                "max_tokens" => Some(StopReason::MaxTokens),
                "stop_sequence" => Some(StopReason::StopSequence),
                "tool_use" => Some(StopReason::ToolUse),
                _ => None,
            }),
            usage: TokenUsage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
            },
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ChatResponse> {
        debug!(
            "Sending {} messages to Anthropic ({})",
            messages.len(),
            self.model
        );

        // Extract system messages
        let system_message = messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| {
                m.content.first().and_then(|c| {
                    if let ContentBlock::Text { text } = c {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            });

        let api_messages = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let mut request_body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": api_messages,
        });

        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        if !api_tools.is_empty() {
            request_body["tools"] = json!(api_tools);
        }

        let response = self
            .client
            .post(format!("{}/messages", ANTHROPIC_API_BASE))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OllmError::LlmProvider(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OllmError::LlmProvider(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse = response.json().await.map_err(|e| {
            OllmError::LlmProvider(format!("Failed to parse response: {}", e))
        })?;

        info!(
            "Received response: {} tokens",
            api_response.usage.output_tokens
        );

        self.parse_response(api_response)
    }

    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<Box<dyn Stream<Item = Result<ChatChunk>> + Send + Unpin>> {
        debug!(
            "Streaming {} messages to Anthropic ({})",
            messages.len(),
            self.model
        );

        // Extract system message
        let system_message = messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| {
                m.content.first().and_then(|c| {
                    if let ContentBlock::Text { text } = c {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            });

        let api_messages = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let mut request_body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": api_messages,
            "stream": true,
        });

        if let Some(system) = system_message {
            request_body["system"] = json!(system);
        }

        if !api_tools.is_empty() {
            request_body["tools"] = json!(api_tools);
        }

        let response = self
            .client
            .post(format!("{}/messages", ANTHROPIC_API_BASE))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OllmError::LlmProvider(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OllmError::LlmProvider(format!(
                "API error {}: {}",
                status, error_text
            )));
        }

        // Create event source stream
        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|event| match event {
                Ok(event) => {
                    if event.event == "message_start"
                        || event.event == "content_block_start"
                        || event.event == "content_block_delta"
                        || event.event == "content_block_stop"
                        || event.event == "message_delta"
                        || event.event == "message_stop"
                    {
                        // Parse the event data as ChatChunk
                        serde_json::from_str::<ChatChunk>(&event.data)
                            .map_err(|e| OllmError::LlmProvider(format!("Parse error: {}", e)))
                    } else if event.event == "ping" {
                        Ok(ChatChunk::Ping)
                    } else {
                        Ok(ChatChunk::Ping) // Ignore unknown events
                    }
                }
                Err(e) => Err(OllmError::LlmProvider(format!("Stream error: {}", e))),
            });

        Ok(Box::new(Box::pin(stream)))
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// API types for Anthropic

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: Vec<ApiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ApiContent {
    Text {
        r#type: String,
        text: String,
    },
    ToolUse {
        r#type: String,
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        r#type: String,
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ApiContent>,
    model: String,
    stop_reason: Option<String>,
    usage: ApiUsage,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: usize,
    output_tokens: usize,
}
