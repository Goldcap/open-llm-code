pub mod anthropic;
pub mod ollama;
pub mod types;

use crate::error::Result;
use crate::types::{Message, Tool};
use async_trait::async_trait;
use futures::Stream;
pub use types::*;

/// LLM Provider trait - abstraction over different LLM backends
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages and get a response
    async fn chat(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ChatResponse>;

    /// Send messages and stream the response
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<Box<dyn Stream<Item = Result<ChatChunk>> + Send + Unpin>>;

    /// Check if this provider supports tool use
    fn supports_tools(&self) -> bool;

    /// Get maximum tokens supported
    fn max_tokens(&self) -> usize;

    /// Get provider name
    fn name(&self) -> &str;

    /// Get model name
    fn model(&self) -> &str;
}

/// Create a provider based on configuration
pub async fn create_provider(
    config: &crate::config::Config,
) -> Result<Box<dyn LlmProvider>> {
    match config.llm.provider.as_str() {
        "anthropic" => {
            let provider = anthropic::AnthropicProvider::new(config)?;
            Ok(Box::new(provider))
        }
        "ollama" => {
            let provider = ollama::OllamaProvider::new(config)?;
            Ok(Box::new(provider))
        }
        _ => Err(crate::error::OllmError::Config(format!(
            "Unknown LLM provider: {}",
            config.llm.provider
        ))),
    }
}
