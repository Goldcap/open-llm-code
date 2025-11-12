# Development Guide

## Getting Started

### Prerequisites

1. **Rust Toolchain** (1.70+)
   ```bash
   source /root/.cargo/env
   rustc --version
   ```

2. **OpenSearch** (for session persistence)
   - Domain: `claude-ltm`
   - Endpoint: https://search-claude-ltm-7m5t3scn2lls4drmfth3jpkfaa.us-west-2.es.amazonaws.com
   - Index: `ollm-sessions`

3. **Optional: Ollama** (for local models)
   ```bash
   curl https://ollama.ai/install.sh | sh
   ollama pull codellama:13b
   ```

### Building

```bash
cd /srv/repos/open-llm-code

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code without building
cargo check
```

### Running

```bash
# With cargo
cargo run -- init
cargo run -- --verbose

# After installing binary
ollm init
ollm --help
```

## Project Structure

```
open-llm-code/
├── Cargo.toml              # Dependencies and metadata
├── src/
│   ├── main.rs             # CLI entry point
│   ├── config/             # Configuration management
│   │   └── mod.rs
│   ├── error.rs            # Error types
│   ├── types.rs            # Core data structures
│   ├── llm/                # LLM provider implementations
│   │   ├── mod.rs          # Provider trait
│   │   ├── anthropic.rs    # Claude API client
│   │   ├── ollama.rs       # Ollama client
│   │   └── types.rs        # Common types
│   ├── mcp/                # MCP protocol client
│   │   ├── mod.rs
│   │   ├── client.rs       # MCP client
│   │   ├── protocol.rs     # Protocol types
│   │   └── transport.rs    # STDIO/HTTP transport
│   ├── session/            # Session management
│   │   ├── mod.rs
│   │   ├── manager.rs      # Session CRUD
│   │   └── persistence.rs  # OpenSearch integration
│   ├── tools/              # Tool execution
│   │   ├── mod.rs
│   │   └── executor.rs     # Execute MCP tools
│   └── ui/                 # Terminal UI
│       ├── mod.rs
│       ├── repl.rs         # Interactive REPL
│       └── renderer.rs     # Format output
├── config/                 # Example configs
│   └── config.example.toml
├── docs/                   # Documentation
│   └── ARCHITECTURE.md
└── examples/               # Example code
```

## Implementation Phases

### Phase 1: Core Foundation (Current)
- [x] Project structure
- [x] Configuration system
- [x] Error handling
- [x] Core types
- [x] CLI framework
- [ ] LLM provider trait

### Phase 2: LLM Integration
- [ ] Anthropic provider
  - [ ] Message API
  - [ ] Tool use support
  - [ ] Streaming responses
- [ ] Ollama provider
  - [ ] Chat completion
  - [ ] Model listing
- [ ] Provider selection logic

### Phase 3: MCP Client
- [ ] STDIO transport
- [ ] Protocol messages (initialize, list_tools, call_tool)
- [ ] Tool discovery
- [ ] Tool execution
- [ ] Result formatting

### Phase 4: Session Persistence
- [ ] OpenSearch client (reuse claude-ltm patterns)
- [ ] Session CRUD operations
- [ ] Conversation storage
- [ ] Session search
- [ ] Session export/import

### Phase 5: Terminal UI
- [ ] REPL with rustyline
- [ ] Command history
- [ ] Syntax highlighting
- [ ] Streaming output
- [ ] Progress indicators

### Phase 6: Polish
- [ ] Error messages
- [ ] Performance optimization
- [ ] Documentation
- [ ] Examples
- [ ] Tests

## Coding Standards

### Error Handling

Use the custom `Result` type and `OllmError` enum:

```rust
use crate::error::{OllmError, Result};

fn do_something() -> Result<String> {
    Err(OllmError::Config("Invalid config".to_string()))
}
```

### Async Code

Use `tokio` for async runtime:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // async code
    Ok(())
}

async fn fetch_data() -> Result<Data> {
    // async operations
}
```

### Logging

Use `tracing` for logging:

```rust
use tracing::{debug, info, warn, error};

info!("Starting application");
debug!("Config loaded: {:?}", config);
warn!("Deprecated feature used");
error!("Failed to connect: {}", e);
```

### Configuration

Configuration is loaded from `~/.config/open-llm-code/config.toml`:

```rust
use crate::config::Config;

let config = Config::load(None)?;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        assert_eq!(1 + 1, 2);
    }

    #[tokio::test]
    async fn test_async() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place in `tests/` directory:

```bash
tests/
├── llm_provider_tests.rs
├── mcp_client_tests.rs
└── session_tests.rs
```

## Deployment

### Build Release Binary

```bash
cargo build --release

# Binary will be at: target/release/ollm
```

### Install

```bash
cp target/release/ollm /usr/local/bin/
chmod +x /usr/local/bin/ollm
```

### Configuration Setup

```bash
ollm init
# Edit ~/.config/open-llm-code/config.toml

# Set environment variables
export ANTHROPIC_API_KEY="your-key"
export OPENSEARCH_PASSWORD="your-password"
```

## Debugging

### Enable Verbose Logging

```bash
ollm --verbose
```

### Rust Logging

```bash
RUST_LOG=debug cargo run
RUST_LOG=open_llm_code=trace cargo run
```

### Debugging with rust-lldb

```bash
rust-lldb target/debug/ollm
```

## Common Tasks

### Add a New Dependency

```bash
cargo add serde
cargo add --features derive serde
cargo add --dev mockito
```

### Format Code

```bash
cargo fmt
```

### Lint Code

```bash
cargo clippy
```

### Update Dependencies

```bash
cargo update
```

## Resources

- [Anthropic API Docs](https://docs.anthropic.com/en/api/)
- [MCP Protocol Spec](https://modelcontextprotocol.io/)
- [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [OpenSearch Rust Client](https://github.com/opensearch-project/opensearch-rs)
- [claude-ltm Reference](/srv/repos/claude-ltm)

## Getting Help

- Check existing code in `/srv/repos/claude-ltm` for OpenSearch patterns
- Review MCP servers in `~/mcp-servers/` for protocol examples
- Ask Claude Code for assistance!
