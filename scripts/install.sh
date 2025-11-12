#!/bin/bash
# Install open-llm-code

set -e

echo "🔧 Building open-llm-code..."
cd "$(dirname "$0")/.."
source /root/.cargo/env

cargo build --release

echo "📦 Installing binary..."
sudo cp target/release/ollm /usr/local/bin/
sudo chmod +x /usr/local/bin/ollm

echo "✅ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Run: ollm init"
echo "2. Edit: ~/.config/open-llm-code/config.toml"
echo "3. Set environment variables:"
echo "   export ANTHROPIC_API_KEY='your-key'"
echo "   export OPENSEARCH_PASSWORD='your-password'"
echo "4. Run: ollm"
