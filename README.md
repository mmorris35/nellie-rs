# Nellie-RS

Your AI agent's **code memory** — semantic search, lessons learned, and checkpoint recovery.

[![CI](https://github.com/mmorris35/nellie-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/mmorris35/nellie-rs/actions/workflows/ci.yml)
[![Release](https://github.com/mmorris35/nellie-rs/actions/workflows/release.yml/badge.svg)](https://github.com/mmorris35/nellie-rs/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## What is Nellie?

Nellie is a local semantic code search server that gives AI agents persistent memory:

- **🔍 Semantic Code Search** — Find code by meaning, not just keywords
- **📚 Lessons Learned** — Teach Nellie patterns, mistakes, and preferences
- **💾 Checkpoints** — Save/restore agent working context for quick recovery
- **👁️ File Watching** — Auto-indexes code changes in real-time
- **🚀 Fast** — SQLite + sqlite-vec for local vector search, ONNX embeddings

## Quick Install

**One-liner (macOS & Linux):**
```bash
curl -sSL https://raw.githubusercontent.com/mmorris35/nellie-rs/main/packaging/install-universal.sh | bash
```

This auto-detects your platform, downloads the binary + embedding model, and sets up the service.

**Manual download:**
- [nellie-macos-aarch64](https://github.com/mmorris35/nellie-rs/releases/latest) — Apple Silicon (M1/M2/M3)
- [nellie-macos-x86_64](https://github.com/mmorris35/nellie-rs/releases/latest) — Intel Mac
- [nellie-linux-x86_64](https://github.com/mmorris35/nellie-rs/releases/latest) — Linux x86_64
- [nellie-linux-aarch64](https://github.com/mmorris35/nellie-rs/releases/latest) — Linux ARM64

## Quick Start

```bash
# Start server watching your code directories
nellie serve --watch ~/code,~/projects --port 8765

# Health check
curl http://localhost:8765/health

# Search your code
curl -X POST http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "search_code", "arguments": {"query": "OAuth authentication", "limit": 5}}'
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          Nellie-RS                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   MCP API    │    │   Embedding  │    │   File Watcher   │  │
│  │   (SSE/HTTP) │    │   (ONNX)     │    │   (notify-rs)    │  │
│  └──────┬───────┘    └──────┬───────┘    └────────┬─────────┘  │
│         │                   │                     │             │
│         └───────────────────┼─────────────────────┘             │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              SQLite + sqlite-vec (embedded)              │   │
│  │         Vector storage + chunks + lessons + checkpoints  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## MCP Integration

Nellie implements the [Model Context Protocol](https://modelcontextprotocol.io/) for AI assistant integration.

### OpenClaw / Claude Code

Add to your MCP configuration:
```yaml
mcp:
  servers:
    nellie:
      transport: sse
      url: http://localhost:8765/sse
```

### Available Tools

| Tool | Description |
|------|-------------|
| `search_code` | Semantic search across indexed code |
| `search_lessons` | Find lessons by natural language |
| `add_lesson` | Record a lesson learned |
| `list_lessons` | List all lessons |
| `add_checkpoint` | Save agent working context |
| `get_checkpoint` | Retrieve checkpoint by agent name |
| `search_checkpoints` | Search checkpoints by content |
| `index_repo` | Index a specific directory |
| `diff_index` | Incremental index update |
| `full_reindex` | Clear and rebuild index |
| `get_status` | Server stats (chunks, files, lessons) |

## REST API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check with version |
| `/sse` | GET | MCP SSE transport |
| `/mcp/tools` | GET | List available tools |
| `/mcp/invoke` | POST | Invoke MCP tool |
| `/api/search` | POST | Direct search API |
| `/api/lessons` | POST | Add lesson |
| `/api/lessons/search` | POST | Search lessons |
| `/api/checkpoints` | POST | Add checkpoint |

## Configuration

### CLI Options

```bash
nellie serve [OPTIONS]

Options:
  --host <HOST>          Bind address [default: 127.0.0.1]
  --port <PORT>          Port [default: 8765]
  --data-dir <DIR>       Data directory [default: ~/.nellie-rs or /var/lib/nellie-rs]
  --watch <DIRS>         Directories to watch (comma-separated)
  --log-level <LEVEL>    Log level: trace/debug/info/warn/error [default: info]
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `NELLIE_DATA_DIR` | Data directory path |
| `NELLIE_HOST` | Bind address |
| `NELLIE_PORT` | Server port |
| `RUST_LOG` | Log level |

## Service Setup

### macOS (launchd)

```bash
# The installer creates this automatically, or manually:
cat > ~/Library/LaunchAgents/com.nellie-rs.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.nellie-rs</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.nellie-rs/nellie</string>
        <string>serve</string>
        <string>--watch</string>
        <string>~/code</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
launchctl load ~/Library/LaunchAgents/com.nellie-rs.plist
```

### Linux (systemd)

```bash
systemctl --user enable nellie
systemctl --user start nellie
sudo loginctl enable-linger $USER  # Start on boot without login
```

## Multi-Machine Sync with Syncthing

For teams or multi-machine setups, use [Syncthing](https://syncthing.net/) to keep code synchronized:

```
BigDev (source) ←→ mini-dev-server ←→ workstation ←→ laptop
                         ↓
                    Nellie indexes
                    local copy
```

Nellie watches local directories — Syncthing handles the sync. This avoids slow network filesystem issues (NFS/SMB).

## Indexing Best Practices

### Manual Indexing Tools

When file watching is unreliable (network mounts, large repos), use manual indexing:

| Tool | Use Case |
|------|----------|
| `index_repo` | Index a directory on demand — best for agent startup |
| `diff_index` | Incremental sync comparing mtimes — fast for routine updates |
| `full_reindex` | Nuclear option — clears and rebuilds entire index |

**Tip**: Call `index_repo` when starting work on a repo to ensure Nellie has fresh context.

### Network Filesystem Limitation

⚠️ **macOS fsevents do not work on NFS/SMB mounts**. The file watcher will start but receive zero events.

**Solutions:**
1. **Syncthing** (recommended) — Sync to local disk, Nellie watches local copy
2. **Polling** — Use `diff_index` via cron/heartbeat for periodic updates
3. **Manual** — Call `index_repo` when you know files changed

## Performance

- **Query latency**: <100ms for 100k+ chunks
- **Indexing**: ~1000 files/minute
- **Memory**: ~500MB for 100k chunks
- **Embedding model**: all-MiniLM-L6-v2 (90MB ONNX)

## Development

```bash
# Clone
git clone https://github.com/mmorris35/nellie-rs.git
cd nellie-rs

# Build
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- serve --watch .
```

## Roadmap

- [ ] [Web Dashboard UI](https://github.com/mmorris35/nellie-rs/issues/21)
- [ ] [PDF Text Extraction](https://github.com/mmorris35/nellie-rs/issues/22)
- [ ] Multi-tenant support
- [ ] Remote/distributed indexing

## Documentation

- [Agent Integration Guide](docs/AGENT_GUIDE.md) — For AI agents installing Nellie
- [Operator Guide](docs/OPERATOR_GUIDE.md) — For sysadmins deploying Nellie

## License

MIT License — see [LICENSE](LICENSE)

## Links

- [GitHub Releases](https://github.com/mmorris35/nellie-rs/releases)
- [Issues](https://github.com/mmorris35/nellie-rs/issues)
