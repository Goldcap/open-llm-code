use crate::types::{ContentBlock, Role};
use serde::{Deserialize, Serialize};

/// Response from an LLM chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Response content
    pub content: Vec<ContentBlock>,

    /// Model that generated the response
    pub model: String,

    /// Stop reason
    pub stop_reason: Option<StopReason>,

    /// Token usage information
    pub usage: TokenUsage,
}

/// Reason why generation stopped
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of response
    EndTurn,
    /// Hit max tokens
    MaxTokens,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested
    ToolUse,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl TokenUsage {
    pub fn total(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

/// Chunk from a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatChunk {
    /// Content delta (text or tool use)
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },

    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },

    ContentBlockStop {
        index: usize,
    },

    /// Message metadata
    MessageStart {
        message: MessageMetadata,
    },

    MessageDelta {
        delta: MessageDelta,
    },

    MessageStop,

    /// Ping (keep-alive)
    Ping,

    /// Error
    Error {
        error: String,
    },
}

/// Content delta in a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

/// Message metadata from stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub id: String,
    pub role: Role,
    pub model: String,
}

/// Message delta from stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<StopReason>,
    pub usage: Option<TokenUsage>,
}
