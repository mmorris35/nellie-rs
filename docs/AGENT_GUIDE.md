# Nellie-RS Agent Guide

Instructions for AI agents (OpenClaw, Claude, etc.) to install, configure, and use Nellie-RS.

## Quick Install (One-Liner)

```bash
curl -sSL https://raw.githubusercontent.com/mmorris35/nellie-rs/main/packaging/install-universal.sh | bash
```

This auto-detects your platform (macOS/Linux, x86_64/ARM64), downloads the binary, embedding model, and sets up the service.

After install, configure your watch directories:
```bash
~/.nellie-rs/nellie serve --watch /path/to/your/code
```

## Install from Shared Server (Private Repo)

For teams without GitHub access to the private repo, binaries can be shared via internal server:

### Setup (Admin)

1. Download release binaries from GitHub:
   ```bash
   gh release download v0.1.1 --repo mmorris35/nellie-rs --dir /shared/nellie
   ```

2. Share the folder containing:
   ```
   /shared/nellie/
   ├── nellie-macos-aarch64    # Apple Silicon
   ├── nellie-macos-x86_64     # Intel Mac
   ├── nellie-linux-x86_64     # Linux
   ├── nellie-linux-aarch64    # Linux ARM
   └── install-local.sh        # Installer script
   ```

### Install (Coworker)

```bash
# Navigate to shared folder (or copy locally first)
cd /path/to/shared/nellie

# Run installer - auto-detects platform
./install-local.sh
```

The installer will:
- Detect macOS/Linux and x86_64/ARM64 automatically
- Copy the correct binary to `~/.nellie-rs/`
- Download the embedding model from Hugging Face
- Set up the service (launchd on macOS, systemd on Linux)
- Create a symlink in `~/.local/bin/`

### Post-Install (IMPORTANT)

**Always ensure the service is running after install:**

**macOS:**
```bash
# Load and start the service
launchctl unload ~/Library/LaunchAgents/com.nellie-rs.plist 2>/dev/null
launchctl load ~/Library/LaunchAgents/com.nellie-rs.plist

# Verify it's running
curl http://localhost:8765/health
```

**Linux:**
```bash
# Enable and start the service
systemctl --user daemon-reload
systemctl --user enable nellie
systemctl --user restart nellie

# Verify it's running
curl http://localhost:8765/health
```

**Configure watch directories:**
```bash
# Edit config
nano ~/.nellie-rs/config.toml

# Add your code directories, then restart:
# macOS: launchctl unload/load ~/Library/LaunchAgents/com.nellie-rs.plist
# Linux: systemctl --user restart nellie
```

---

## What is Nellie?

Nellie is your **code memory** — a local server that:
- **Indexes your codebase** with semantic embeddings
- **Remembers lessons** you teach it (mistakes, patterns, preferences)
- **Saves checkpoints** of your working context for quick recovery
- **Searches semantically** — find code by meaning, not just keywords

## Quick Reference

| Action | MCP Tool | REST API |
|--------|----------|----------|
| Search code | `search_code` | `POST /api/search` |
| Add lesson | `add_lesson` | `POST /api/lessons` |
| Search lessons | `search_lessons` | `POST /api/lessons/search` |
| Save checkpoint | `add_checkpoint` | `POST /api/checkpoints` |
| Get checkpoint | `get_checkpoint` | `GET /api/checkpoints/:id` |
| Index a repo | `index_repo` | `POST /mcp/invoke` |
| Check status | `get_status` | `GET /health` |

---

## Installation (Agent-Guided)

### Step 1: Download Binary

Determine the user's platform and download the correct binary:

```bash
# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
[[ "$OS" == "darwin" ]] && OS="macos"
[[ "$ARCH" == "arm64" ]] && ARCH="aarch64"
[[ "$ARCH" == "x86_64" ]] && ARCH="x86_64"

BINARY="nellie-${OS}-${ARCH}"
echo "Need binary: $BINARY"
```

Binary options:
- `nellie-macos-aarch64` — Apple Silicon Macs (M1/M2/M3)
- `nellie-macos-x86_64` — Intel Macs
- `nellie-linux-aarch64` — Linux ARM64
- `nellie-linux-x86_64` — Linux x86_64

### Step 2: Install

```bash
# Create directories
mkdir -p ~/.nellie-rs/{logs,models}
mkdir -p ~/.local/bin

# Copy binary (assuming it's in current directory)
cp nellie-* ~/.nellie-rs/nellie
chmod +x ~/.nellie-rs/nellie
ln -sf ~/.nellie-rs/nellie ~/.local/bin/nellie

# Download embedding model
curl -L -o ~/.nellie-rs/models/all-MiniLM-L6-v2.onnx \
  "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx"
```

### Step 3: Configure

Create `~/.nellie-rs/config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8765

[watcher]
watch_dirs = [
    "/Users/USERNAME/code",
    "/Users/USERNAME/projects",
]
```

**Ask the user** which directories contain their code, then write the config.

### Step 4: Start Service

**macOS:**
```bash
# Create launchd plist
cat > ~/Library/LaunchAgents/com.nellie-rs.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.nellie-rs</string>
    <key>ProgramArguments</key>
    <array>
        <string>~/.nellie-rs/nellie</string>
        <string>--config</string>
        <string>~/.nellie-rs/config.toml</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>~/.nellie-rs/logs/nellie.log</string>
    <key>StandardErrorPath</key><string>~/.nellie-rs/logs/nellie.log</string>
</dict>
</plist>
EOF

# Start it
launchctl load ~/Library/LaunchAgents/com.nellie-rs.plist
```

**Linux:**
```bash
# Create systemd user service
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/nellie.service << 'EOF'
[Unit]
Description=Nellie-RS
After=network.target

[Service]
ExecStart=%h/.nellie-rs/nellie --config %h/.nellie-rs/config.toml
Restart=on-failure

[Install]
WantedBy=default.target
EOF

# Start it
systemctl --user enable --now nellie
```

### Step 5: Verify

```bash
curl http://localhost:8765/health
# Should return: {"status":"healthy","version":"0.1.0","database":"ok"}
```

---

## Using Nellie

### Option A: MCP Mode (Recommended for OpenClaw)

Add to your OpenClaw config's MCP servers:

```yaml
mcp:
  servers:
    nellie:
      transport: sse
      url: http://localhost:8765/sse
```

Or use mcporter:
```bash
mcporter add nellie --sse http://localhost:8765/sse
```

Then call tools directly:
```bash
mcporter call nellie search_code --query "OAuth authentication"
mcporter call nellie add_lesson --title "Error handling" --content "Always wrap API calls in try/catch"
```

### Option B: REST API Mode

For direct HTTP calls without MCP:

```bash
# Search code
curl -X POST http://localhost:8765/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "database connection pool", "limit": 5}'

# Add lesson
curl -X POST http://localhost:8765/api/lessons \
  -H "Content-Type: application/json" \
  -d '{"title": "Rate limiting", "content": "Use exponential backoff for retries", "tags": ["api", "reliability"]}'

# Search lessons
curl -X POST http://localhost:8765/api/lessons/search \
  -H "Content-Type: application/json" \
  -d '{"query": "error handling patterns"}'
```

### Option C: MCP via HTTP (No SSE)

For environments where SSE doesn't work:

```bash
# List available tools
curl http://localhost:8765/mcp/tools

# Invoke a tool
curl -X POST http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "search_code", "arguments": {"query": "authentication", "limit": 5}}'
```

---

## MCP Tools Reference

### Code Search

**`search_code`** — Semantic search across indexed code
```json
{
  "name": "search_code",
  "arguments": {
    "query": "how to handle file uploads",
    "limit": 10
  }
}
```

**`get_status`** — Check indexing status
```json
{
  "name": "get_status",
  "arguments": {}
}
```
Returns: `{"stats": {"chunks": 10390, "files": 2480, "lessons": 22}}`

### Indexing (Manual)

**`index_repo`** — Index a specific directory
```json
{
  "name": "index_repo",
  "arguments": {
    "path": "/path/to/repo"
  }
}
```

**`diff_index`** — Incremental update (new/modified/deleted files)
```json
{
  "name": "diff_index",
  "arguments": {
    "path": "/path/to/repo"
  }
}
```

**`full_reindex`** — Clear and rebuild index for a path
```json
{
  "name": "full_reindex",
  "arguments": {
    "path": "/path/to/repo"
  }
}
```

### Lessons

**`add_lesson`** — Teach Nellie something
```json
{
  "name": "add_lesson",
  "arguments": {
    "title": "API Design",
    "content": "Always version APIs from day one. Use /v1/ prefix.",
    "tags": ["api", "design", "versioning"]
  }
}
```

**`search_lessons`** — Find relevant lessons
```json
{
  "name": "search_lessons",
  "arguments": {
    "query": "API best practices",
    "limit": 5
  }
}
```

**`list_lessons`** — List all lessons
```json
{
  "name": "list_lessons",
  "arguments": {
    "limit": 50
  }
}
```

### Checkpoints

**`add_checkpoint`** — Save working context
```json
{
  "name": "add_checkpoint",
  "arguments": {
    "agent_name": "my-agent",
    "content": "## Current Task\nImplementing OAuth flow\n\n## Progress\n- [x] Set up routes\n- [ ] Add token refresh",
    "tags": ["oauth", "in-progress"]
  }
}
```

**`get_checkpoint`** — Retrieve a checkpoint
```json
{
  "name": "get_checkpoint",
  "arguments": {
    "agent_name": "my-agent"
  }
}
```

**`search_checkpoints`** — Find checkpoints by content
```json
{
  "name": "search_checkpoints",
  "arguments": {
    "query": "OAuth implementation"
  }
}
```

---

## Agent Best Practices

### 1. Check Nellie on Startup

```bash
curl -s http://localhost:8765/health | jq -e '.status == "healthy"' || echo "Nellie not running"
```

### 2. Save Checkpoints Regularly

When doing complex work, save your state:
```json
{
  "name": "add_checkpoint",
  "arguments": {
    "agent_name": "workspace-agent",
    "content": "## Task: Fix authentication bug\n\n### Done\n- Found issue in token.rs line 42\n- Root cause: expiry not checked\n\n### Next\n- Add expiry validation\n- Write tests",
    "tags": ["auth", "bugfix"]
  }
}
```

### 3. Record Lessons from Mistakes

When you learn something, save it:
```json
{
  "name": "add_lesson", 
  "arguments": {
    "title": "SQLite on NFS",
    "content": "SQLite doesn't work reliably on NFS mounts. Use a local path for the database, or use PostgreSQL for network storage.",
    "tags": ["sqlite", "nfs", "gotcha"]
  }
}
```

### 4. Search Before Asking

Before asking the user how something works:
```json
{
  "name": "search_code",
  "arguments": {
    "query": "how does authentication work in this codebase"
  }
}
```

### 5. Use diff_index for Updates

After the user modifies files:
```json
{
  "name": "diff_index",
  "arguments": {
    "path": "/path/to/modified/repo"
  }
}
```

---

## Troubleshooting

### Nellie not responding
```bash
# Check if running
pgrep -f nellie

# Check logs
tail -50 ~/.nellie-rs/logs/nellie.log

# Restart (macOS)
launchctl unload ~/Library/LaunchAgents/com.nellie-rs.plist
launchctl load ~/Library/LaunchAgents/com.nellie-rs.plist
```

### No search results
```bash
# Check index status
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_status", "arguments": {}}' | jq .content.stats

# If chunks=0, trigger indexing
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "index_repo", "arguments": {"path": "/path/to/code"}}'
```

### Port already in use
Edit `~/.nellie-rs/config.toml` and change the port:
```toml
[server]
port = 8766
```

---

## System Requirements

- **OS**: macOS 12+ or Linux (glibc)
- **Arch**: x86_64 or ARM64
- **RAM**: 2GB minimum, 4GB+ recommended
- **Disk**: 200MB + ~50MB per 10k code chunks
- **Network**: Localhost only by default

---

## Quick Test

After installation, verify everything works:

```bash
# 1. Health check
curl http://localhost:8765/health

# 2. Check indexed files
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "get_status", "arguments": {}}' | jq .content.stats

# 3. Test search
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "search_code", "arguments": {"query": "main function", "limit": 3}}' | jq .content.results

# 4. Add a test lesson
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "add_lesson", "arguments": {"title": "Test", "content": "Nellie is working!", "tags": ["test"]}}'

# 5. Search lessons
curl -s http://localhost:8765/mcp/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "search_lessons", "arguments": {"query": "working"}}' | jq .content.results
```

If all commands succeed, Nellie is ready! 🧠
