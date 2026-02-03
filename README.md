# Nellie Production

Production-grade semantic code memory system for enterprise engineering teams.

[![CI](https://github.com/sequeldata/nellie-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sequeldata/nellie-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Overview

Nellie is a semantic code search and knowledge management system that helps engineering teams:

- **Find code semantically**: Search your codebase using natural language queries
- **Learn from experience**: Store and retrieve lessons learned across projects
- **Maintain context**: Save and restore AI agent working state with checkpoints

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Nellie Production                        │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   MCP API    │    │   Embedding  │    │   File Watcher   │  │
│  │   (rmcp)     │───▶│   Worker     │    │   (notify-rs)    │  │
│  │              │    │   (ONNX)     │    │                  │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│         │                   │                    │              │
│  ┌──────────────┐           │                    │              │
│  │   REST API   │           │                    │              │
│  │   (axum)     │───────────┤                    │              │
│  └──────────────┘           │                    │              │
│         │                   ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              SQLite + sqlite-vec (embedded)              │   │
│  │         Vector storage + metadata + FTS search           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Installation

Download the latest release for your platform:

```bash
# Linux x86_64
curl -LO https://github.com/sequeldata/nellie-rs/releases/latest/download/nellie-linux-x86_64
chmod +x nellie-linux-x86_64
sudo mv nellie-linux-x86_64 /usr/local/bin/nellie

# Linux ARM64
curl -LO https://github.com/sequeldata/nellie-rs/releases/latest/download/nellie-linux-aarch64
chmod +x nellie-linux-aarch64
sudo mv nellie-linux-aarch64 /usr/local/bin/nellie
```

### Running

```bash
# Start with default settings
nellie

# Start with custom data directory
nellie --data-dir /var/lib/nellie

# Start with watch directories
nellie --watch /home/user/projects,/opt/code

# Full options
nellie --host 0.0.0.0 --port 8080 --log-level debug
```

### Systemd Service

For production deployments, install as a systemd service:

```bash
# Clone repository
git clone https://github.com/sequeldata/nellie-rs.git
cd nellie-rs

# Build release
cargo build --release

# Install service (requires root)
sudo ./packaging/install.sh target/release/nellie

# Start service
sudo systemctl start nellie
sudo systemctl enable nellie

# Check status
sudo systemctl status nellie
```

## Configuration

### CLI Options

| Option | Environment Variable | Default | Description |
|--------|---------------------|---------|-------------|
| `--data-dir` | `NELLIE_DATA_DIR` | `./data` | Data directory for database |
| `--host` | `NELLIE_HOST` | `127.0.0.1` | Server bind address |
| `--port` | `NELLIE_PORT` | `8080` | Server port |
| `--log-level` | `NELLIE_LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |
| `--watch` | `NELLIE_WATCH_DIRS` | - | Directories to watch (comma-separated) |
| `--embedding-threads` | `NELLIE_EMBEDDING_THREADS` | `4` | Embedding worker threads |

### Embedding Model

Nellie uses ONNX models for embedding generation. Download the model:

```bash
mkdir -p data/models
# Download all-MiniLM-L6-v2 ONNX model
wget -O data/models/all-MiniLM-L6-v2.onnx <model-url>
wget -O data/models/tokenizer.json <tokenizer-url>
```

## API

### MCP Protocol

Nellie implements the Model Context Protocol (MCP) for integration with AI assistants.

**Available Tools:**

| Tool | Description |
|------|-------------|
| `search_code` | Semantic code search across indexed repositories |
| `search_lessons` | Search lessons by natural language |
| `add_lesson` | Record a lesson learned |
| `add_checkpoint` | Save agent checkpoint |
| `get_recent_checkpoints` | Retrieve recent checkpoints |
| `get_status` | Server status and statistics |

**Claude Code Configuration:**

Add to your MCP settings:

```json
{
  "mcpServers": {
    "nellie": {
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

### REST API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/metrics` | GET | Prometheus metrics |
| `/api/v1/status` | GET | Server status |

### Health Check

```bash
curl http://localhost:8080/health
# {"status":"healthy","version":"0.1.0","database":"ok"}
```

### Prometheus Metrics

```bash
curl http://localhost:8080/metrics
# HELP nellie_chunks_total Total number of indexed code chunks
# TYPE nellie_chunks_total gauge
# nellie_chunks_total 12345
```

## Development

### Building from Source

```bash
# Clone
git clone https://github.com/sequeldata/nellie-rs.git
cd nellie-rs

# Build
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Project Structure

```
nellie-rs/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── config/              # Configuration
│   ├── error/               # Error types
│   ├── storage/             # SQLite + sqlite-vec
│   ├── embeddings/          # ONNX embedding worker
│   ├── watcher/             # File watching & indexing
│   └── server/              # MCP & REST API
├── tests/                   # Integration tests
├── packaging/               # Systemd & installation
└── scripts/                 # Build scripts
```

### Running Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific module
cargo test storage::

# Integration tests
cargo test --test '*'
```

## Performance

**Targets:**
- Query latency: <200ms p95 at 1M chunks
- Indexing throughput: 1000+ files/minute
- Memory usage: <2GB for 1M chunks
- Startup time: <10s cold start

## Requirements

- **OS**: Linux (x86_64 or ARM64)
- **Memory**: 2GB minimum, 4GB recommended
- **Disk**: 1GB + indexed data
- **CPU**: 2 cores minimum, 4 recommended

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

- [GitHub Issues](https://github.com/sequeldata/nellie-rs/issues)
- [Documentation](https://github.com/sequeldata/nellie-rs/wiki)
