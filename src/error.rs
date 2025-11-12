use thiserror::Error;

#[derive(Error, Debug)]
pub enum OllmError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("MCP protocol error: {0}")]
    McpProtocol(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("OpenSearch error: {0}")]
    OpenSearch(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OllmError>;
