use crate::error::{OllmError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub huggingface: HuggingFaceConfig,
    pub opensearch: OpenSearchConfig,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider: "anthropic", "ollama", or "huggingface"
    pub provider: String,
    /// Model name
    pub model: String,
    /// Environment variable name for API key (for Anthropic and HuggingFace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Max tokens in response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaConfig {
    /// Ollama API endpoint
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    /// Model to use (e.g., "codellama:13b")
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HuggingFaceConfig {
    /// HuggingFace API endpoint (default is Inference API)
    #[serde(default = "default_huggingface_endpoint")]
    pub endpoint: String,
    /// Model to use (e.g., "codellama/CodeLlama-7b-Instruct-hf")
    #[serde(default = "default_huggingface_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSearchConfig {
    pub endpoint: String,
    pub username: String,
    /// Environment variable name for password
    pub password_env: String,
    #[serde(default = "default_index")]
    pub index: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_max_tokens() -> usize {
    4096
}

fn default_ollama_endpoint() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "codellama:13b".to_string()
}

fn default_index() -> String {
    "ollm-sessions".to_string()
}

fn default_huggingface_endpoint() -> String {
    "https://api-inference.huggingface.co".to_string()
}

fn default_huggingface_model() -> String {
    "codellama/CodeLlama-7b-Instruct-hf".to_string()
}

impl Config {
    /// Load configuration from file
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = path.unwrap_or_else(|| {
            let mut p = dirs::home_dir().expect("Cannot determine home directory");
            p.push(".config");
            p.push("open-llm-code");
            p.push("config.toml");
            p
        });

        if !config_path.exists() {
            return Err(OllmError::Config(format!(
                "Config file not found: {}",
                config_path.display()
            )));
        }

        let config_str = std::fs::read_to_string(&config_path).map_err(|e| {
            OllmError::Config(format!("Failed to read config file: {}", e))
        })?;

        let config: Config = toml::from_str(&config_str).map_err(|e| {
            OllmError::Config(format!("Failed to parse config file: {}", e))
        })?;

        Ok(config)
    }

    /// Generate example configuration
    pub fn example() -> String {
        let example = Config {
            llm: LlmConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4".to_string(),
                api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
                max_tokens: 4096,
            },
            ollama: OllamaConfig {
                endpoint: "http://localhost:11434".to_string(),
                model: "codellama:13b".to_string(),
            },
            huggingface: HuggingFaceConfig {
                endpoint: "https://api-inference.huggingface.co".to_string(),
                model: "codellama/CodeLlama-7b-Instruct-hf".to_string(),
            },
            opensearch: OpenSearchConfig {
                endpoint: "https://search-example.us-west-2.es.amazonaws.com".to_string(),
                username: "admin".to_string(),
                password_env: "OPENSEARCH_PASSWORD".to_string(),
                index: "ollm-sessions".to_string(),
            },
            mcp_servers: vec![
                McpServerConfig {
                    name: "claude-ltm".to_string(),
                    command: "cltm-server".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                },
                McpServerConfig {
                    name: "aws-eks".to_string(),
                    command: "npx".to_string(),
                    args: vec!["-y".to_string(), "@modelcontextprotocol/server-aws-eks".to_string()],
                    env: {
                        let mut env = HashMap::new();
                        env.insert("AWS_REGION".to_string(), "us-west-2".to_string());
                        env
                    },
                },
            ],
        };

        toml::to_string_pretty(&example).unwrap()
    }
}
