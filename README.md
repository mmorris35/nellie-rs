# Nellie Production

Production-grade semantic code memory system for enterprise engineering teams.

## Features

- **Semantic Code Search**: Natural language queries across indexed repositories
- **Lessons Learned**: Store and retrieve engineering lessons with tags
- **Agent Checkpoints**: Save/restore AI agent working state
- **MCP Protocol**: Native Model Context Protocol support for Claude Code

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Nellie Production                        │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   MCP API    │    │   Embedding  │    │   File Watcher   │  │
│  │   (rmcp)     │───▶│   Worker     │    │   (notify-rs)    │  │
│  │              │    │   (Queue)    │    │                  │  │
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

```bash
# Build
cargo build --release

# Run with default config
./target/release/nellie

# Run with custom data directory
./target/release/nellie --data-dir /var/lib/nellie
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `NELLIE_DATA_DIR` | `./data` | Data directory for SQLite database |
| `NELLIE_HOST` | `127.0.0.1` | Server bind address |
| `NELLIE_PORT` | `8080` | Server port |
| `NELLIE_LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |

## Development

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## License

MIT License - see [LICENSE](LICENSE) for details.
