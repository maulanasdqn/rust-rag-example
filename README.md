# Rust RAG Application

A production-ready Retrieval-Augmented Generation (RAG) system built with Rust, featuring an agentic architecture, security controls, and a modern web UI.

## Features

### Core RAG
- **Document ingestion** - Upload PDF and text files
- **Vector search** - SurrealDB with embedded vectors for semantic search
- **Streaming responses** - Real-time SSE streaming from OpenAI
- **Conversation memory** - Persistent chat history with context

### Security
- **Rate limiting** - 60 req/min (general), 20 req/min (expensive operations)
- **API key authentication** - Optional Bearer token or X-API-Key header
- **Input validation** - Max length checks, empty input rejection
- **Prompt injection detection** - Logs suspicious patterns
- **Cost controls** - Configurable max_tokens limit
- **CORS** - Configurable allowed origins

### Agentic System
- **Tool system** - Calculator, DateTime, Web Search tools
- **Multi-agent architecture** - Researcher agent with tool execution
- **Execution loop** - Think → Act → Observe cycle

### UI
- **Leptos frontend** - Rust/WASM single-page application
- **Chat interface** - Conversation history, streaming responses
- **Document management** - Upload, list, delete documents
- **Dark theme** - Modern dark UI with themed scrollbars

### Telegram Bot
- **RAG queries** - Ask questions via Telegram
- **Document upload** - Send PDF/text files to index
- **Conversation memory** - Per-user chat history
- **Commands** - /start, /help, /new, /docs, /history, /clear

## Architecture

```
rag-server/      # Axum HTTP server
rag-config/      # Configuration management
rag-database/    # SurrealDB vector store
rag-documents/   # Document loading and chunking
rag-inference/   # OpenAI LLM provider
rag-query/       # RAG query execution
rag-memory/      # Conversation storage
rag-tools/       # Tool definitions
rag-agents/      # Agent system
rag-types/       # Shared types
rag-errors/      # Error handling
rag-ui/          # Leptos frontend
```

## Quick Start

### Prerequisites
- Rust 1.75+
- Node.js 18+ (for Tailwind CSS)
- OpenAI API key

### Development

```bash
# Clone and setup
git clone <repo>
cd rust-rag-example

# Set environment variables
export OPENAI__API_KEY=your-api-key

# Run with Nix (recommended)
nix develop
dev  # starts backend + frontend with hot reload

# Or manually
cargo run -p rag-server &
cd rag-ui && trunk serve --port 8081
```

### Production

```bash
# Build
cargo build --release -p rag-server
cd rag-ui && trunk build --release

# Run
./target/release/rag-server
```

## Configuration

Environment variables (use `__` for nested keys):

```bash
# Server
SERVER__HOST=0.0.0.0
SERVER__PORT=8080

# Database
DATABASE__PATH=./data/rag.db

# OpenAI
OPENAI__API_KEY=sk-...
OPENAI__API_BASE=https://api.openai.com/v1
OPENAI__EMBEDDING_MODEL=text-embedding-3-small
OPENAI__CHAT_MODEL=gpt-4o

# RAG
RAG__CHUNK_SIZE=1000
RAG__CHUNK_OVERLAP=200
RAG__TOP_K=5
RAG__MIN_RELEVANCE_SCORE=0.10

# Security
SECURITY__REQUIRE_AUTH=false
SECURITY__API_KEYS=key1,key2
SECURITY__RATE_LIMIT_RPM=60
SECURITY__EXPENSIVE_RATE_LIMIT_RPM=20
SECURITY__MAX_QUERY_LENGTH=10000
SECURITY__MAX_TOKENS=4096
SECURITY__ALLOWED_ORIGINS=http://localhost:8081

# Telegram Bot
TELEGRAM__ENABLED=true
TELEGRAM__BOT_TOKEN=123456:ABC-DEF...
TELEGRAM__MAX_MESSAGE_LENGTH=4000
```

## API Endpoints

### Documents
- `POST /api/documents/documents` - Upload document
- `GET /api/documents/documents` - List documents
- `DELETE /api/documents/documents/:id` - Delete document

### Query
- `POST /api/query` - Query documents (non-streaming)
- `POST /api/chat/stream` - Chat with streaming (SSE)

### Conversations
- `POST /api/conversations` - Create conversation
- `GET /api/conversations` - List conversations
- `GET /api/conversations/:id` - Get conversation
- `DELETE /api/conversations/:id` - Delete conversation
- `GET /api/conversations/:id/messages` - Get messages
- `POST /api/conversations/:id/messages` - Add message

### Tools & Agents
- `GET /api/tools` - List available tools
- `POST /api/tools/:name/execute` - Execute tool
- `GET /api/agents` - List agents
- `POST /api/agents/execute` - Execute agent query

### Health
- `GET /api/health` - Health check

## Security

### Enable Authentication

```bash
SECURITY__REQUIRE_AUTH=true
SECURITY__API_KEYS=your-secret-key
```

Then include header in requests:
```
Authorization: Bearer your-secret-key
# or
X-API-Key: your-secret-key
```

### Rate Limiting
Automatically enabled. Returns `429 Too Many Requests` when exceeded.

### Prompt Injection
Detected patterns are logged but requests are processed (to avoid false positives).

## Telegram Bot Setup

1. Create a bot via [@BotFather](https://t.me/BotFather) on Telegram
2. Copy the bot token
3. Set environment variables:

```bash
TELEGRAM__ENABLED=true
TELEGRAM__BOT_TOKEN=123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11
```

4. Start the server - the bot will run alongside the HTTP server

### Bot Commands

| Command | Description |
|---------|-------------|
| `/start` | Welcome message and instructions |
| `/help` | Show available commands |
| `/new` | Start a fresh conversation |
| `/docs` | List uploaded documents |
| `/history` | View recent conversation |
| `/clear` | Clear conversation history |

### Features

- **Ask questions**: Just send any message to query your documents
- **Upload documents**: Send PDF or text files to index them
- **Conversation memory**: Each user has their own conversation history
- **Source attribution**: Shows relevant source documents with confidence scores

## License

MIT
