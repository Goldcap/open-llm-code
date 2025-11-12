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

pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
    max_tokens: usize,
}

impl OllamaProvider {
    pub fn new(config: &crate::config::Config) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            endpoint: config.ollama.endpoint.clone(),
            model: config.ollama.model.clone(),
            max_tokens: config.llm.max_tokens,
        })
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OllamaMessage> {
        messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                };

                // Combine all text content blocks
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

                OllamaMessage {
                    role: role.to_string(),
                    content,
                }
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ChatResponse> {
        debug!(
            "Sending {} messages to Ollama ({})",
            messages.len(),
            self.model
        );

        if !tools.is_empty() {
            warn!("Ollama provider does not support tool use - tools will be ignored");
        }

        let ollama_messages = self.convert_messages(messages);

        let request_body = json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "num_predict": self.max_tokens,
            }
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.endpoint))
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
                "Ollama API error {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response.json().await.map_err(|e| {
            OllmError::LlmProvider(format!("Failed to parse Ollama response: {}", e))
        })?;

        info!("Received response from Ollama");

        Ok(ChatResponse {
            content: vec![ContentBlock::Text {
                text: ollama_response.message.content,
            }],
            model: ollama_response.model,
            stop_reason: Some(if ollama_response.done {
                StopReason::EndTurn
            } else {
                StopReason::MaxTokens
            }),
            usage: TokenUsage {
                input_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
                output_tokens: ollama_response.eval_count.unwrap_or(0),
            },
        })
    }

    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<Box<dyn Stream<Item = Result<ChatChunk>> + Send + Unpin>> {
        debug!(
            "Streaming {} messages to Ollama ({})",
            messages.len(),
            self.model
        );

        if !tools.is_empty() {
            warn!("Ollama provider does not support tool use - tools will be ignored");
        }

        let ollama_messages = self.convert_messages(messages);

        let request_body = json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": true,
            "options": {
                "num_predict": self.max_tokens,
            }
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.endpoint))
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
                "Ollama API error {}: {}",
                status, error_text
            )));
        }

        // Parse NDJSON stream
        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| OllmError::LlmProvider(format!("Stream error: {}", e)))
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk);
                    serde_json::from_str::<OllamaStreamChunk>(&text)
                        .map_err(|e| OllmError::LlmProvider(format!("Parse error: {}", e)))
                })
                .map(|ollama_chunk| {
                    if ollama_chunk.done {
                        ChatChunk::MessageStop
                    } else {
                        ChatChunk::ContentBlockDelta {
                            index: 0,
                            delta: ContentDelta::TextDelta {
                                text: ollama_chunk.message.content,
                            },
                        }
                    }
                })
        });

        Ok(Box::new(Box::pin(stream)))
    }

    fn supports_tools(&self) -> bool {
        false // Ollama doesn't support structured tool use (yet)
    }

    fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// Ollama API types

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaMessage,
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<usize>,
    #[serde(default)]
    eval_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    model: String,
    message: OllamaMessage,
    done: bool,
}
