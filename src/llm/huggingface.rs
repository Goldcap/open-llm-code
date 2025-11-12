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

    fn convert_messages(&self, messages: Vec<Message>) -> String {
        // Format messages for instruction-tuned models
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

                match m.role {
                    Role::System => format!("System: {}", content),
                    Role::User => format!("[INST] {} [/INST]", content),
                    Role::Assistant => content,
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
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

        let prompt = self.convert_messages(messages);

        let request_body = json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": self.max_tokens,
                "temperature": 0.7,
                "top_p": 0.95,
                "return_full_text": false
            }
        });

        let url = format!("{}/models/{}", self.endpoint, self.model);

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

        let hf_response: Vec<HFResponse> = response.json().await.map_err(|e| {
            OllmError::LlmProvider(format!("Failed to parse HuggingFace response: {}", e))
        })?;

        if hf_response.is_empty() {
            return Err(OllmError::LlmProvider(
                "Empty response from HuggingFace".to_string(),
            ));
        }

        let generated_text = hf_response[0].generated_text.clone();

        info!("Received response from HuggingFace");

        // HuggingFace doesn't provide token counts in the free API
        // We'll estimate based on characters
        let estimated_tokens = generated_text.len() / 4;

        Ok(ChatResponse {
            content: vec![ContentBlock::Text {
                text: generated_text,
            }],
            model: self.model.clone(),
            stop_reason: Some(StopReason::EndTurn),
            usage: TokenUsage {
                input_tokens: 0, // Not provided by HF
                output_tokens: estimated_tokens,
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

        let prompt = self.convert_messages(messages);

        let request_body = json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": self.max_tokens,
                "temperature": 0.7,
                "top_p": 0.95,
                "return_full_text": false
            },
            "stream": true
        });

        let url = format!("{}/models/{}", self.endpoint, self.model);

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

        // Parse SSE stream from HuggingFace
        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| OllmError::LlmProvider(format!("Stream error: {}", e)))
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk);

                    // HuggingFace sends Server-Sent Events
                    if text.starts_with("data: ") {
                        let json_str = text.strip_prefix("data: ").unwrap_or(&text);

                        if json_str.trim() == "[DONE]" {
                            return Ok(ChatChunk::MessageStop);
                        }

                        serde_json::from_str::<HFStreamChunk>(json_str)
                            .map_err(|e| OllmError::LlmProvider(format!("Parse error: {}", e)))
                            .map(|hf_chunk| ChatChunk::ContentBlockDelta {
                                index: 0,
                                delta: ContentDelta::TextDelta {
                                    text: hf_chunk.token.text,
                                },
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

// HuggingFace API types

#[derive(Debug, Deserialize)]
struct HFResponse {
    generated_text: String,
}

#[derive(Debug, Deserialize)]
struct HFStreamChunk {
    token: HFToken,
}

#[derive(Debug, Deserialize)]
struct HFToken {
    text: String,
}
