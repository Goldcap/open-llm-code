use super::types::*;
use crate::error::{OllmError, Result};
use crate::types::Tool;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// MCP Server connection via stdio
pub struct McpClient {
    name: String,
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: Arc<AtomicI32>,
    server_info: Option<Implementation>,
    tools: Vec<McpTool>,
}

impl McpClient {
    /// Start an MCP server process
    pub fn start(
        name: String,
        command: String,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
    ) -> Result<Self> {
        info!("Starting MCP server '{}': {} {:?}", name, command, args);

        let mut child = Command::new(&command)
            .args(&args)
            .envs(&env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Silence stderr to avoid mixing with stdout
            .spawn()
            .map_err(|e| {
                OllmError::Mcp(format!("Failed to start MCP server '{}': {}", name, e))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            OllmError::Mcp(format!("Failed to get stdin for MCP server '{}'", name))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            OllmError::Mcp(format!("Failed to get stdout for MCP server '{}'", name))
        })?;

        let stdout = BufReader::new(stdout);

        Ok(Self {
            name,
            process: child,
            stdin,
            stdout,
            request_id: Arc::new(AtomicI32::new(1)),
            server_info: None,
            tools: Vec::new(),
        })
    }

    /// Initialize the MCP server
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing MCP server '{}'", self.name);

        let params = InitializeParams {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "open-llm-code".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let response = self.send_request("initialize", Some(json!(params)))?;

        let result: InitializeResult = serde_json::from_value(response).map_err(|e| {
            OllmError::Mcp(format!("Failed to parse initialize response: {}", e))
        })?;

        info!(
            "MCP server '{}' initialized: {} v{}",
            self.name, result.server_info.name, result.server_info.version
        );

        self.server_info = Some(result.server_info);

        // List available tools
        self.list_tools()?;

        Ok(())
    }

    /// List available tools from the server
    fn list_tools(&mut self) -> Result<()> {
        debug!("Listing tools from MCP server '{}'", self.name);

        let response = self.send_request("tools/list", None)?;

        let result: ListToolsResult = serde_json::from_value(response).map_err(|e| {
            OllmError::Mcp(format!("Failed to parse tools/list response: {}", e))
        })?;

        info!(
            "MCP server '{}' has {} tools",
            self.name,
            result.tools.len()
        );

        self.tools = result.tools;

        Ok(())
    }

    /// Get all available tools
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|t| Tool {
                name: format!("{}::{}", self.name, t.name),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect()
    }

    /// Call a tool on the MCP server
    pub fn call_tool(&mut self, tool_name: &str, arguments: Option<Value>) -> Result<String> {
        debug!(
            "Calling tool '{}' on MCP server '{}'",
            tool_name, self.name
        );

        let params = CallToolParams {
            name: tool_name.to_string(),
            arguments,
        };

        let response = self.send_request("tools/call", Some(json!(params)))?;

        let result: CallToolResult = serde_json::from_value(response).map_err(|e| {
            OllmError::Mcp(format!("Failed to parse tools/call response: {}", e))
        })?;

        // Check for errors
        if result.is_error == Some(true) {
            let error_text = result
                .content
                .iter()
                .filter_map(|c| match c {
                    ToolContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            return Err(OllmError::Mcp(format!("Tool call error: {}", error_text)));
        }

        // Combine all text content
        let result_text = result
            .content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(result_text)
    }

    /// Send a JSON-RPC request and wait for response
    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest::new(id, method.to_string(), params);

        // Send request
        let request_json = serde_json::to_string(&request).map_err(|e| {
            OllmError::Mcp(format!("Failed to serialize JSON-RPC request: {}", e))
        })?;

        debug!("Sending request to '{}': {}", self.name, request_json);

        writeln!(self.stdin, "{}", request_json).map_err(|e| {
            OllmError::Mcp(format!("Failed to write to MCP server '{}': {}", self.name, e))
        })?;

        self.stdin.flush().map_err(|e| {
            OllmError::Mcp(format!(
                "Failed to flush stdin for MCP server '{}': {}",
                self.name, e
            ))
        })?;

        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).map_err(|e| {
            OllmError::Mcp(format!(
                "Failed to read from MCP server '{}': {}",
                self.name, e
            ))
        })?;

        debug!("Received response from '{}': {}", self.name, response_line);

        let response: JsonRpcResponse = serde_json::from_str(&response_line).map_err(|e| {
            OllmError::Mcp(format!("Failed to parse JSON-RPC response: {} - Response was: {}", e, response_line))
        })?;

        // Check for error
        if let Some(error) = response.error {
            return Err(OllmError::Mcp(format!(
                "JSON-RPC error {}: {}",
                error.code, error.message
            )));
        }

        response.result.ok_or_else(|| {
            OllmError::Mcp("JSON-RPC response missing result field".to_string())
        })
    }

    /// Send a JSON-RPC notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let notification_json = serde_json::to_string(&notification).map_err(|e| {
            OllmError::Mcp(format!("Failed to serialize JSON-RPC notification: {}", e))
        })?;

        debug!(
            "Sending notification to '{}': {}",
            self.name, notification_json
        );

        writeln!(self.stdin, "{}", notification_json).map_err(|e| {
            OllmError::Mcp(format!(
                "Failed to write notification to MCP server '{}': {}",
                self.name, e
            ))
        })?;

        self.stdin.flush().map_err(|e| {
            OllmError::Mcp(format!(
                "Failed to flush stdin for MCP server '{}': {}",
                self.name, e
            ))
        })?;

        Ok(())
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        debug!("Shutting down MCP server '{}'", self.name);
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
