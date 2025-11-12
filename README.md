# Open LLM Code (ollm)

A Rust-based AI coding assistant with pluggable LLM backends and MCP (Model Context Protocol) support.

## Features

- 🔌 **Pluggable LLM Backends**
  - Anthropic Claude API
  - Ollama (CodeLlama, Llama 3, etc.)
  - Easy to add more providers

- 🛠️ **MCP Protocol Support**
  - Connect to multiple MCP servers
  - Auto-discover available tools
  - Execute tools seamlessly in conversations

- 💾 **Session Persistence**
  - Store conversations in OpenSearch
  - Resume previous sessions
  - Full-text search across history

- 🎨 **Clean Terminal UI**
  - Interactive REPL with history
  - Syntax highlighting
  - Streaming responses

## Installation

### Prerequisites

- Rust 1.70+ (`source /root/.cargo/env`)
- OpenSearch 2.11+ (for session persistence)
- Optional: Ollama (for local models)

### Build from Source

```bash
cd /srv/repos/open-llm-code
cargo build --release

# Install binary
cp target/release/ollm /usr/local/bin/
chmod +x /usr/local/bin/ollm
```

## Quick Start

### 1. Initialize Configuration

```bash
ollm init
```

This creates `~/.config/open-llm-code/config.toml` with example configuration.

### 2. Configure Your Setup

Edit `~/.config/open-llm-code/config.toml`:

```toml
[llm]
provider = "anthropic"  # or "ollama"
model = "claude-sonnet-4"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096

[ollama]
endpoint = "http://localhost:11434"
model = "codellama:13b"

[opensearch]
endpoint = "https://your-opensearch-domain.amazonaws.com"
username = "admin"
password_env = "OPENSEARCH_PASSWORD"
index = "ollm-sessions"

[[mcp_servers]]
name = "claude-ltm"
command = "cltm-server"

[[mcp_servers]]
name = "aws-eks"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-aws-eks"]
env = { AWS_REGION = "us-west-2" }
```

### 3. Set Environment Variables

```bash
export ANTHROPIC_API_KEY="your-api-key-here"
export OPENSEARCH_PASSWORD="your-opensearch-password"
```

### 4. Start the REPL

```bash
ollm
```

## Usage

### Interactive Mode

```bash
ollm
```

Start a conversation with your configured LLM. The assistant has access to all MCP tools.

### With Specific Config

```bash
ollm --config /path/to/config.toml
```

### Verbose Logging

```bash
ollm --verbose
```

## Architecture

```
ollm
├── LLM Providers (pluggable)
│   ├── Anthropic (Claude)
│   └── Ollama (CodeLlama, etc.)
├── MCP Client
│   ├── Protocol implementation
│   ├── Tool discovery
│   └── Tool execution
├── Session Manager
│   ├── OpenSearch persistence
│   └── Conversation history
└── Terminal UI
    ├── REPL with history
    └── Streaming output
```

## Configuration

### LLM Providers

**Anthropic:**
```toml
[llm]
provider = "anthropic"
model = "claude-sonnet-4"
api_key_env = "ANTHROPIC_API_KEY"
```

**Ollama:**
```toml
[llm]
provider = "ollama"
model = "codellama:13b"

[ollama]
endpoint = "http://localhost:11434"
model = "codellama:13b"
```

### MCP Servers

Add as many MCP servers as needed:

```toml
[[mcp_servers]]
name = "server-name"
command = "command-to-run"
args = ["arg1", "arg2"]
env = { KEY = "value" }
```

## Development

### Project Structure

```
src/
├── main.rs                 # CLI entry point
├── config/                 # Configuration management
├── error.rs                # Error types
├── types.rs                # Core data structures
├── llm/                    # LLM provider implementations
│   ├── anthropic.rs
│   └── ollama.rs
├── mcp/                    # MCP protocol client
│   ├── client.rs
│   ├── protocol.rs
│   └── transport.rs
├── session/                # Session persistence
│   ├── manager.rs
│   └── opensearch.rs
├── tools/                  # Tool execution
│   └── executor.rs
└── ui/                     # Terminal UI
    ├── repl.rs
    └── renderer.rs
```

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Roadmap

- [x] Project structure
- [x] Configuration management
- [x] CLI framework
- [ ] LLM provider trait
- [ ] Anthropic provider implementation
- [ ] Ollama provider implementation
- [ ] MCP client implementation
- [ ] Session persistence (OpenSearch)
- [ ] Terminal REPL
- [ ] Tool execution
- [ ] Streaming responses
- [ ] Session history/search
- [ ] Cost tracking
- [ ] Multi-session management

## License

MIT

## Authors

87 Technologies LLC
