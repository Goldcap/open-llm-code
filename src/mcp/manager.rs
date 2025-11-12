use super::client::McpClient;
use crate::config::McpServerConfig;
use crate::error::{OllmError, Result};
use crate::types::Tool;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, info};

/// Manages multiple MCP server connections
pub struct McpManager {
    clients: HashMap<String, McpClient>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Start and initialize MCP servers from configuration
    pub fn start_servers(&mut self, configs: Vec<McpServerConfig>) -> Result<()> {
        info!("Starting {} MCP servers...", configs.len());

        for config in configs {
            info!("Starting MCP server: {}", config.name);

            match McpClient::start(
                config.name.clone(),
                config.command.clone(),
                config.args.clone(),
                config.env.clone(),
            ) {
                Ok(mut client) => {
                    info!("MCP server '{}' process started, initializing...", config.name);
                    if let Err(e) = client.initialize() {
                        error!("Failed to initialize MCP server '{}': {}", config.name, e);
                        eprintln!("❌ Failed to initialize MCP server '{}': {}", config.name, e);
                        continue;
                    }
                    info!("MCP server '{}' initialized successfully", config.name);
                    self.clients.insert(config.name, client);
                }
                Err(e) => {
                    error!("Failed to start MCP server '{}': {}", config.name, e);
                    eprintln!("❌ Failed to start MCP server '{}': {}", config.name, e);
                    continue;
                }
            }
        }

        info!("Successfully started {} MCP servers", self.clients.len());

        Ok(())
    }

    /// Get all available tools from all MCP servers
    pub fn get_all_tools(&self) -> Vec<Tool> {
        self.clients
            .values()
            .flat_map(|client| client.get_tools())
            .collect()
    }

    /// Call a tool on the appropriate MCP server
    pub fn call_tool(&mut self, tool_name: &str, arguments: Option<Value>) -> Result<String> {
        // Tool name format: "server_name::tool_name"
        let parts: Vec<&str> = tool_name.split("::").collect();

        if parts.len() != 2 {
            return Err(OllmError::Mcp(format!(
                "Invalid tool name format: '{}' (expected 'server::tool')",
                tool_name
            )));
        }

        let server_name = parts[0];
        let actual_tool_name = parts[1];

        let client = self.clients.get_mut(server_name).ok_or_else(|| {
            OllmError::Mcp(format!("MCP server '{}' not found", server_name))
        })?;

        client.call_tool(actual_tool_name, arguments)
    }

    /// Get number of connected servers
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }

    /// Get list of server names
    pub fn server_names(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
