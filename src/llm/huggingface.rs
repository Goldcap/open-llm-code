use super::types::*;
use super::LlmProvider;
use crate::error::{OllmError, Result};
use crate::types::{ContentBlock, Message, Role, Tool};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info, warn};

pub struct HuggingFaceProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
    max_tokens: usize,
}

impl HuggingFaceProvider {
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
                "api_key_env not configured for HuggingFace".to_string(),
            ));
        };

        Ok(Self {
            client: Client::new(),
            api_key,
            endpoint: config.huggingface.endpoint.clone(),
            model: config.huggingface.model.clone(),
            max_tokens: config.llm.max_tokens,
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<HFMessage> {
        messages
            .into_iter()
            .map(|m| {
                let content = m
                    .content
                    .iter()
                    .filter_map(|c| {
                        if let ContentBlock::Text { text } = c {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                HFMessage {
                    role: match m.role {
                        Role::System => "system".to_string(),
                        Role::User => "user".to_string(),
                        Role::Assistant => "assistant".to_string(),
                    },
                    content,
                }
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for HuggingFaceProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ChatResponse> {
        debug!(
            "Sending {} messages to HuggingFace ({})",
            messages.len(),
            self.model
        );

        if !tools.is_empty() {
            warn!("HuggingFace Inference API does not support tool use - tools will be ignored");
        }

        let hf_messages = self.convert_messages(messages);

        // Use OpenAI-compatible chat completions API
        let request_body = json!({
            "model": self.model,
            "messages": hf_messages,
            "max_tokens": self.max_tokens,
            "temperature": 0.7,
            "top_p": 0.95
        });

        let url = format!("{}/chat/completions", self.endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
                "HuggingFace API error {}: {}",
                status, error_text
            )));
        }

        let hf_response: HFChatCompletionResponse = response.json().await.map_err(|e| {
            OllmError::LlmProvider(format!("Failed to parse HuggingFace response: {}", e))
        })?;

        if hf_response.choices.is_empty() {
            return Err(OllmError::LlmProvider(
                "Empty response from HuggingFace".to_string(),
            ));
        }

        let generated_text = hf_response.choices[0].message.content.clone();

        info!("Received response from HuggingFace");

        Ok(ChatResponse {
            content: vec![ContentBlock::Text {
                text: generated_text,
            }],
            model: hf_response.model,
            stop_reason: Some(match hf_response.choices[0].finish_reason.as_str() {
                "stop" => StopReason::EndTurn,
                "length" => StopReason::MaxTokens,
                _ => StopReason::EndTurn,
            }),
            usage: TokenUsage {
                input_tokens: hf_response.usage.prompt_tokens,
                output_tokens: hf_response.usage.completion_tokens,
            },
        })
    }

    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<Box<dyn Stream<Item = Result<ChatChunk>> + Send + Unpin>> {
        debug!(
            "Streaming {} messages to HuggingFace ({})",
            messages.len(),
            self.model
        );

        if !tools.is_empty() {
            warn!("HuggingFace Inference API does not support tool use - tools will be ignored");
        }

        let hf_messages = self.convert_messages(messages);

        let request_body = json!({
            "model": self.model,
            "messages": hf_messages,
            "max_tokens": self.max_tokens,
            "temperature": 0.7,
            "top_p": 0.95,
            "stream": true
        });

        let url = format!("{}/chat/completions", self.endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
                "HuggingFace API error {}: {}",
                status, error_text
            )));
        }

        // Parse SSE stream (OpenAI format)
        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| OllmError::LlmProvider(format!("Stream error: {}", e)))
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk);

                    // HuggingFace sends Server-Sent Events like OpenAI
                    if text.starts_with("data: ") {
                        let json_str = text.strip_prefix("data: ").unwrap_or(&text);

                        if json_str.trim() == "[DONE]" {
                            return Ok(ChatChunk::MessageStop);
                        }

                        serde_json::from_str::<HFChatCompletionChunk>(json_str)
                            .map_err(|e| OllmError::LlmProvider(format!("Parse error: {}", e)))
                            .and_then(|hf_chunk| {
                                if let Some(choice) = hf_chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        Ok(ChatChunk::ContentBlockDelta {
                                            index: 0,
                                            delta: ContentDelta::TextDelta {
                                                text: content.clone(),
                                            },
                                        })
                                    } else {
                                        Ok(ChatChunk::Ping)
                                    }
                                } else {
                                    Ok(ChatChunk::Ping)
                                }
                            })
                    } else {
                        Ok(ChatChunk::Ping)
                    }
                })
        });

        Ok(Box::new(Box::pin(stream)))
    }

    fn supports_tools(&self) -> bool {
        false // HuggingFace Inference API doesn't support structured tool use
    }

    fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    fn name(&self) -> &str {
        "huggingface"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// HuggingFace API types (OpenAI-compatible)

#[derive(Debug, Serialize, Deserialize)]
struct HFMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct HFChatCompletionResponse {
    model: String,
    choices: Vec<HFChoice>,
    usage: HFUsage,
}

#[derive(Debug, Deserialize)]
struct HFChoice {
    message: HFMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct HFUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct HFChatCompletionChunk {
    choices: Vec<HFChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct HFChunkChoice {
    delta: HFDelta,
}

#[derive(Debug, Deserialize)]
struct HFDelta {
    content: Option<String>,
}
