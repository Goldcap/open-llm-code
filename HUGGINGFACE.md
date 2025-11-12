# Using HuggingFace Inference API with Open LLM Code

This guide shows you how to use HuggingFace's free Inference API with CodeLlama for minimal-cost testing.

## Setup

### 1. Get a HuggingFace API Token

**Option 1: Quick Setup (Recommended for Testing)**
1. Go to https://huggingface.co/settings/tokens/new?ownUserPermissions=inference.serverless.write&tokenType=fineGrained
2. Name it "ollm-inference" (or whatever you like)
3. The permission "Make calls to Inference Providers" should already be selected
4. Click "Generate token"
5. Copy the token (starts with `hf_...`)

**Option 2: Manual Setup**
1. Go to https://huggingface.co/settings/tokens
2. Click "New token"
3. Select "Fine-grained" token type
4. Name it "ollm-inference"
5. Under "Permissions", enable **"Make calls to Inference Providers"**
6. Click "Generate"
7. Copy the token (starts with `hf_...`)

**Important**: You need a **fine-grained token** with `inference.serverless.write` permission, not just "Read" access.

### 2. Set the Environment Variable

```bash
export HUGGINGFACE_API_KEY="hf_your_token_here"

# Add to your shell profile for persistence
echo 'export HUGGINGFACE_API_KEY="hf_your_token_here"' >> ~/.bashrc
```

### 3. Generate Config File

```bash
ollm init
```

This creates `~/.config/open-llm-code/config.toml`

### 4. Edit Config for HuggingFace

Edit the config file:

```toml
[llm]
provider = "huggingface"
model = "codellama/CodeLlama-7b-Instruct-hf"
api_key_env = "HUGGINGFACE_API_KEY"
max_tokens = 500  # Lower for faster responses in free tier

[huggingface]
endpoint = "https://router.huggingface.co/v1"
model = "codellama/CodeLlama-7b-Instruct-hf"
```

## Test It

```bash
ollm test "Write a Python function to calculate fibonacci numbers"
```

Expected output:
```
🧪 Testing LLM provider...

Provider: huggingface (codellama/CodeLlama-7b-Instruct-hf)
Sending message: Write a Python function to calculate fibonacci numbers

Response:
def fibonacci(n):
    if n <= 1:
        return n
    else:
        return fibonacci(n-1) + fibonacci(n-2)

# Example usage
print(fibonacci(10))  # Output: 55

Tokens: 0 in, 25 out (25)
```

## Available Models

### CodeLlama Models (Recommended)

```toml
# 7B Instruct (Best for instructions)
model = "codellama/CodeLlama-7b-Instruct-hf"

# 13B Instruct (Better quality, slower)
model = "codellama/CodeLlama-13b-Instruct-hf"

# 7B Python (Specialized for Python)
model = "codellama/CodeLlama-7b-Python-hf"
```

### Other Code Models

```toml
# StarCoder2 (Good for code completion)
model = "bigcode/starcoder2-15b"

# DeepSeek Coder (Excellent for many languages)
model = "deepseek-ai/deepseek-coder-6.7b-instruct"

# Phi-3 (Microsoft's small but capable model)
model = "microsoft/Phi-3-mini-4k-instruct"
```

Browse more at: https://huggingface.co/models?pipeline_tag=text-generation

## Free Tier Limits

HuggingFace Inference API (free tier):
- **Rate limit**: ~1-2 requests/second
- **Timeout**: 60 seconds per request
- **Cold starts**: First request may be slow (model loading)
- **Usage**: Free for testing, unlimited for serverless inference

If you hit limits:
1. Wait a few seconds between requests
2. Reduce `max_tokens` in your prompt
3. Use smaller models (7B instead of 13B)
4. Consider upgrading to Pro ($9/month) for dedicated endpoints

## Cost Comparison

| Option | Cost | Response Time | Setup |
|--------|------|---------------|-------|
| **HuggingFace Free Tier** | $0 | 2-10s (cold start: 30s) | API key only |
| HuggingFace Pro | $9/month | 1-3s (no cold starts) | Subscription |
| Ollama Local | $0 | 5-20s | Download model (~4GB) |
| AWS Lambda | ~$0.002/request | 30s+ (cold start) | Complex setup |
| EC2 GPU | ~$350/month | <1s | Full infrastructure |

**For testing**: HuggingFace Free Tier is perfect ✅

## Troubleshooting

### "Model is loading"

The model needs to warm up on first request:
```
Error: Model codellama/CodeLlama-7b-Instruct-hf is currently loading
```

**Solution**: Wait 30-60 seconds and try again. Subsequent requests will be fast.

### Rate Limit Errors

```
Error: Rate limit exceeded
```

**Solution**:
- Wait 10-20 seconds between requests
- Reduce your request frequency
- Consider upgrading to HuggingFace Pro

### Invalid API Key

```
Error: Invalid API token
```

**Solution**:
1. Check your token at https://huggingface.co/settings/tokens
2. Regenerate if needed
3. Update `HUGGINGFACE_API_KEY` environment variable

### Slow Responses

**Solutions**:
- Reduce `max_tokens` (try 200-500)
- Use smaller model (7B instead of 13B)
- First request is always slower (model loading)

## Next Steps

Once you've tested with HuggingFace:

1. **Build MCP integration** - Add tools/function calling
2. **Add session persistence** - Save conversations to OpenSearch
3. **Create REPL interface** - Interactive coding assistant
4. **Compare providers** - Test Anthropic vs Ollama vs HuggingFace

## Tips

- **Start small**: Use 7B models and low max_tokens for testing
- **Be patient**: First request takes 30-60s (model loading)
- **Iterate**: Free tier is perfect for development
- **Upgrade later**: Only pay when you need dedicated resources

Happy coding! 🚀
