# Phase 5: Packaging & Documentation

**Goal**: Create systemd service, cross-compilation, installation scripts, and documentation
**Duration**: 1 week
**Prerequisites**: Phase 4 complete

---

## Task 5.1: System Packaging

**Git**: Create branch `feature/5-1-packaging` when starting first subtask.

### Subtask 5.1.1: Create Systemd Service Configuration (Single Session)

**Prerequisites**:
- [x] 4.2.2: Implement Graceful Shutdown

**Deliverables**:
- [ ] Create systemd service file
- [ ] Add service installation script
- [ ] Create default configuration
- [ ] Document service management

**Files to Create**:

**`packaging/nellie.service`** (complete file):
```ini
[Unit]
Description=Nellie Production - Semantic Code Memory System
Documentation=https://github.com/sequeldata/nellie-rs
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=nellie
Group=nellie
WorkingDirectory=/var/lib/nellie

# Main executable
ExecStart=/usr/local/bin/nellie \
    --data-dir /var/lib/nellie \
    --host 0.0.0.0 \
    --port 8080 \
    --log-level info

# Restart policy
Restart=on-failure
RestartSec=5
StartLimitBurst=3
StartLimitIntervalSec=60

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/nellie /var/log/nellie
PrivateTmp=true
PrivateDevices=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictNamespaces=true
RestrictRealtime=true
RestrictSUIDSGID=true
MemoryDenyWriteExecute=true
LockPersonality=true

# Resource limits
LimitNOFILE=65536
MemoryMax=4G

# Environment
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

# Graceful shutdown
TimeoutStopSec=30
KillMode=mixed
KillSignal=SIGTERM

[Install]
WantedBy=multi-user.target
```

**`packaging/nellie.conf`** (complete file):
```ini
# Nellie Production Configuration
# This file is sourced by the systemd service

# Data directory (must be writable by nellie user)
NELLIE_DATA_DIR=/var/lib/nellie

# Server binding
NELLIE_HOST=0.0.0.0
NELLIE_PORT=8080

# Logging
NELLIE_LOG_LEVEL=info

# Embedding threads (adjust based on CPU cores)
NELLIE_EMBEDDING_THREADS=4

# Watch directories (comma-separated)
# NELLIE_WATCH_DIRS=/home/dev/projects,/opt/code

# Optional: API key for authentication
# NELLIE_API_KEY=your-secret-key
```

**`packaging/install.sh`** (complete file):
```bash
#!/bin/bash
set -euo pipefail

# Nellie Production Installation Script
# Usage: sudo ./install.sh [binary_path]

BINARY_PATH="${1:-./target/release/nellie}"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/nellie"
LOG_DIR="/var/log/nellie"
CONFIG_DIR="/etc/nellie"
USER="nellie"
GROUP="nellie"

echo "Installing Nellie Production..."

# Check root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)"
   exit 1
fi

# Check binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Build with: cargo build --release"
    exit 1
fi

# Create user and group
if ! id -u "$USER" &>/dev/null; then
    echo "Creating user: $USER"
    useradd --system --shell /usr/sbin/nologin --home-dir "$DATA_DIR" "$USER"
fi

# Create directories
echo "Creating directories..."
mkdir -p "$DATA_DIR"
mkdir -p "$DATA_DIR/models"
mkdir -p "$LOG_DIR"
mkdir -p "$CONFIG_DIR"

# Install binary
echo "Installing binary to $INSTALL_DIR..."
cp "$BINARY_PATH" "$INSTALL_DIR/nellie"
chmod 755 "$INSTALL_DIR/nellie"

# Install service file
echo "Installing systemd service..."
cp packaging/nellie.service /etc/systemd/system/
chmod 644 /etc/systemd/system/nellie.service

# Install config if not exists
if [[ ! -f "$CONFIG_DIR/nellie.conf" ]]; then
    echo "Installing default configuration..."
    cp packaging/nellie.conf "$CONFIG_DIR/"
    chmod 640 "$CONFIG_DIR/nellie.conf"
    chown root:$GROUP "$CONFIG_DIR/nellie.conf"
fi

# Set permissions
echo "Setting permissions..."
chown -R "$USER:$GROUP" "$DATA_DIR"
chown -R "$USER:$GROUP" "$LOG_DIR"
chmod 750 "$DATA_DIR"
chmod 750 "$LOG_DIR"

# Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload

# Print next steps
echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Download ONNX model to $DATA_DIR/models/"
echo "     wget -O $DATA_DIR/models/all-MiniLM-L6-v2.onnx <model-url>"
echo "  2. Edit configuration: $CONFIG_DIR/nellie.conf"
echo "  3. Start service: systemctl start nellie"
echo "  4. Enable on boot: systemctl enable nellie"
echo "  5. Check status: systemctl status nellie"
echo "  6. View logs: journalctl -u nellie -f"
echo ""
```

**`packaging/uninstall.sh`** (complete file):
```bash
#!/bin/bash
set -euo pipefail

# Nellie Production Uninstallation Script
# Usage: sudo ./uninstall.sh

echo "Uninstalling Nellie Production..."

# Check root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)"
   exit 1
fi

# Stop service if running
if systemctl is-active --quiet nellie; then
    echo "Stopping service..."
    systemctl stop nellie
fi

# Disable service
if systemctl is-enabled --quiet nellie 2>/dev/null; then
    echo "Disabling service..."
    systemctl disable nellie
fi

# Remove service file
echo "Removing service file..."
rm -f /etc/systemd/system/nellie.service
systemctl daemon-reload

# Remove binary
echo "Removing binary..."
rm -f /usr/local/bin/nellie

echo ""
echo "Uninstallation complete!"
echo ""
echo "The following were NOT removed (manual cleanup if needed):"
echo "  - Data directory: /var/lib/nellie"
echo "  - Log directory: /var/log/nellie"
echo "  - Config directory: /etc/nellie"
echo "  - User: nellie"
echo ""
```

**Verification Commands**:
```bash
# Verify files exist
ls -la packaging/
# Expected: nellie.service, nellie.conf, install.sh, uninstall.sh

# Validate service file syntax (requires systemd)
systemd-analyze verify packaging/nellie.service 2>&1 || echo "systemd-analyze not available"

# Check scripts are executable
chmod +x packaging/install.sh packaging/uninstall.sh
```

**Success Criteria**:
- [ ] Service file created with security hardening
- [ ] Config file with all options documented
- [ ] Install script creates user, dirs, and installs service
- [ ] Uninstall script cleanly removes service
- [ ] Commit made with message "feat(packaging): add systemd service and installation scripts"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `packaging/nellie.service` (X lines)
  - `packaging/nellie.conf` (X lines)
  - `packaging/install.sh` (X lines)
  - `packaging/uninstall.sh` (X lines)
- **Files Modified**: None
- **Tests**: N/A (scripts)
- **Build**: ✅ Scripts validated
- **Branch**: feature/5-1-packaging
- **Notes**: (any additional context)

---

### Subtask 5.1.2: Build Cross-Compilation for ARM64 (Single Session)

**Prerequisites**:
- [x] 5.1.1: Create Systemd Service Configuration

**Deliverables**:
- [ ] Add cross-compilation configuration
- [ ] Create build script for multiple targets
- [ ] Document build requirements
- [ ] Test cross-compilation

**Files to Create**:

**`scripts/build-release.sh`** (complete file):
```bash
#!/bin/bash
set -euo pipefail

# Build Release Script for Nellie Production
# Builds for multiple targets: x86_64 and aarch64 Linux

VERSION="${1:-$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')}"
OUTPUT_DIR="dist"

echo "Building Nellie Production v$VERSION..."

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build for x86_64 Linux
echo "Building for x86_64-unknown-linux-gnu..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/nellie "$OUTPUT_DIR/nellie-linux-x86_64"

# Build for aarch64 Linux (requires cross-compilation toolchain)
if command -v aarch64-linux-gnu-gcc &> /dev/null; then
    echo "Building for aarch64-unknown-linux-gnu..."
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        cargo build --release --target aarch64-unknown-linux-gnu
    cp target/aarch64-unknown-linux-gnu/release/nellie "$OUTPUT_DIR/nellie-linux-aarch64"
else
    echo "Warning: aarch64-linux-gnu-gcc not found, skipping ARM64 build"
    echo "Install with: sudo apt-get install gcc-aarch64-linux-gnu"
fi

# Create checksums
echo "Creating checksums..."
cd "$OUTPUT_DIR"
sha256sum nellie-* > SHA256SUMS
cd -

# Print results
echo ""
echo "Build complete! Artifacts in $OUTPUT_DIR:"
ls -la "$OUTPUT_DIR"
echo ""
cat "$OUTPUT_DIR/SHA256SUMS"
```

**`Cross.toml`** (complete file):
```toml
# Cross-compilation configuration
# Used with the `cross` tool: https://github.com/cross-rs/cross

[build.env]
passthrough = [
    "RUST_BACKTRACE",
    "RUST_LOG",
]

[target.x86_64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:main"

[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main"

[target.x86_64-unknown-linux-musl]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-musl:main"

[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl:main"
```

**Update `.cargo/config.toml`** - add target configs:
```toml
# Add these sections for cross-compilation

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=x86-64-v2"]

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=-crt-static"]

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-gnu-gcc"
rustflags = ["-C", "target-feature=-crt-static"]
```

**Verification Commands**:
```bash
# Install targets
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu

# Build for native target
cargo build --release 2>&1 | tail -5
# Expected: "Finished `release` profile..."

# Verify binary
ls -la target/release/nellie
file target/release/nellie
# Expected: Shows ELF binary info
```

**Success Criteria**:
- [ ] Build script created
- [ ] Cross.toml for cross tool
- [ ] Native release build works
- [ ] Binary info shows correct architecture
- [ ] Commit made with message "feat(packaging): add cross-compilation configuration"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `scripts/build-release.sh` (X lines)
  - `Cross.toml` (X lines)
- **Files Modified**:
  - `.cargo/config.toml` (X lines)
- **Tests**: N/A
- **Build**: ✅ Release build works
- **Branch**: feature/5-1-packaging
- **Notes**: (any additional context)

---

### Task 5.1 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] Release build works
- [ ] Service files valid
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Task 5.2: Documentation

**Git**: Create branch `feature/5-2-documentation` when starting first subtask.

### Subtask 5.2.1: Write Comprehensive README (Single Session)

**Prerequisites**:
- [x] 5.1.2: Build Cross-Compilation

**Deliverables**:
- [ ] Update README with full documentation
- [ ] Add installation instructions
- [ ] Document all CLI options
- [ ] Add usage examples

**Files to Modify**:

**`README.md`** (replace - complete file):
```markdown
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
```

**Verification Commands**:
```bash
# Verify README renders (if markdownlint available)
markdownlint README.md 2>&1 || echo "markdownlint not installed"

# Check word count
wc -l README.md
# Expected: ~250 lines
```

**Success Criteria**:
- [ ] README covers all features
- [ ] Installation instructions clear
- [ ] CLI options documented
- [ ] API documented
- [ ] Commit made with message "docs: write comprehensive README"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: None
- **Files Modified**:
  - `README.md` (X lines)
- **Tests**: N/A
- **Build**: N/A
- **Branch**: feature/5-2-documentation
- **Notes**: (any additional context)

---

### Subtask 5.2.2: Create Operator Guide (Single Session)

**Prerequisites**:
- [x] 5.2.1: Write Comprehensive README

**Deliverables**:
- [ ] Create operator guide for deployments
- [ ] Document backup/restore procedures
- [ ] Add troubleshooting section
- [ ] Document monitoring setup

**Files to Create**:

**`docs/OPERATOR_GUIDE.md`** (complete file):
```markdown
# Nellie Production - Operator Guide

This guide covers deploying and operating Nellie Production in enterprise environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Monitoring](#monitoring)
5. [Backup & Restore](#backup--restore)
6. [Troubleshooting](#troubleshooting)
7. [Security](#security)

## Prerequisites

### Hardware Requirements

| Resource | Minimum | Recommended | Notes |
|----------|---------|-------------|-------|
| CPU | 2 cores | 4+ cores | More cores = faster indexing |
| Memory | 2GB | 4GB+ | Scales with chunk count |
| Disk | 1GB | 10GB+ SSD | SSD strongly recommended |

### Software Requirements

- Linux (x86_64 or ARM64)
- systemd (for service management)
- curl or wget (for downloads)

## Installation

### Automated Installation

```bash
# Download and run installer
curl -sSL https://raw.githubusercontent.com/sequeldata/nellie-rs/main/packaging/install.sh | sudo bash
```

### Manual Installation

1. **Download binary:**
   ```bash
   ARCH=$(uname -m)
   if [ "$ARCH" = "x86_64" ]; then
       curl -LO https://github.com/sequeldata/nellie-rs/releases/latest/download/nellie-linux-x86_64
       sudo mv nellie-linux-x86_64 /usr/local/bin/nellie
   elif [ "$ARCH" = "aarch64" ]; then
       curl -LO https://github.com/sequeldata/nellie-rs/releases/latest/download/nellie-linux-aarch64
       sudo mv nellie-linux-aarch64 /usr/local/bin/nellie
   fi
   sudo chmod +x /usr/local/bin/nellie
   ```

2. **Create user and directories:**
   ```bash
   sudo useradd --system --shell /usr/sbin/nologin --home-dir /var/lib/nellie nellie
   sudo mkdir -p /var/lib/nellie/models /var/log/nellie /etc/nellie
   sudo chown -R nellie:nellie /var/lib/nellie /var/log/nellie
   ```

3. **Install systemd service:**
   ```bash
   sudo curl -o /etc/systemd/system/nellie.service \
       https://raw.githubusercontent.com/sequeldata/nellie-rs/main/packaging/nellie.service
   sudo systemctl daemon-reload
   ```

4. **Download embedding model:**
   ```bash
   sudo -u nellie mkdir -p /var/lib/nellie/models
   # Download model files (URLs to be determined)
   ```

5. **Start service:**
   ```bash
   sudo systemctl start nellie
   sudo systemctl enable nellie
   ```

## Configuration

### Configuration File

Edit `/etc/nellie/nellie.conf`:

```ini
# Data directory
NELLIE_DATA_DIR=/var/lib/nellie

# Network binding
NELLIE_HOST=0.0.0.0
NELLIE_PORT=8080

# Logging
NELLIE_LOG_LEVEL=info

# Performance
NELLIE_EMBEDDING_THREADS=4

# Directories to watch (comma-separated)
NELLIE_WATCH_DIRS=/home/dev/projects

# Optional: API authentication
# NELLIE_API_KEY=your-secret-key
```

### Tuning for Large Deployments

For 1M+ chunks:

```ini
# Increase embedding threads
NELLIE_EMBEDDING_THREADS=8

# Ensure sufficient memory in systemd
# Edit /etc/systemd/system/nellie.service
# MemoryMax=8G
```

## Monitoring

### Health Check

```bash
# Check service status
sudo systemctl status nellie

# HTTP health check
curl -s http://localhost:8080/health | jq .
```

### Prometheus Integration

Add to your Prometheus configuration:

```yaml
scrape_configs:
  - job_name: 'nellie'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: /metrics
```

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `nellie_chunks_total` | Total indexed chunks | N/A (informational) |
| `nellie_request_duration_seconds` | Query latency | p99 > 500ms |
| `nellie_embedding_queue_depth` | Pending embeddings | > 1000 |

### Grafana Dashboard

Import the Nellie dashboard from `docs/grafana-dashboard.json`.

## Backup & Restore

### Database Backup

The Nellie database is a single SQLite file:

```bash
# Stop service for consistent backup
sudo systemctl stop nellie

# Backup database
sudo cp /var/lib/nellie/nellie.db /backup/nellie-$(date +%Y%m%d).db

# Start service
sudo systemctl start nellie
```

### Hot Backup (SQLite Online Backup)

```bash
# Backup while running (uses SQLite backup API)
sqlite3 /var/lib/nellie/nellie.db ".backup '/backup/nellie-hot.db'"
```

### Restore

```bash
sudo systemctl stop nellie
sudo cp /backup/nellie-20240101.db /var/lib/nellie/nellie.db
sudo chown nellie:nellie /var/lib/nellie/nellie.db
sudo systemctl start nellie
```

## Troubleshooting

### Service Won't Start

1. **Check logs:**
   ```bash
   sudo journalctl -u nellie -n 100 --no-pager
   ```

2. **Check permissions:**
   ```bash
   ls -la /var/lib/nellie/
   # Should be owned by nellie:nellie
   ```

3. **Check port availability:**
   ```bash
   sudo ss -tlnp | grep 8080
   ```

### High Memory Usage

1. **Check chunk count:**
   ```bash
   curl -s http://localhost:8080/api/v1/status | jq .stats.chunks
   ```

2. **Increase memory limit if needed:**
   ```bash
   sudo systemctl edit nellie
   # Add: [Service]
   #      MemoryMax=8G
   sudo systemctl restart nellie
   ```

### Slow Queries

1. **Check embedding queue:**
   ```bash
   curl -s http://localhost:8080/metrics | grep embedding_queue
   ```

2. **Increase embedding threads:**
   ```bash
   # Edit /etc/nellie/nellie.conf
   NELLIE_EMBEDDING_THREADS=8
   sudo systemctl restart nellie
   ```

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `database is locked` | Concurrent access | Restart service |
| `out of memory` | Too many chunks | Increase MemoryMax |
| `connection refused` | Service not running | Check systemctl status |

## Security

### Network Security

- Bind to localhost by default
- Use reverse proxy (nginx) for HTTPS
- Configure firewall rules

### Reverse Proxy Example (nginx)

```nginx
server {
    listen 443 ssl;
    server_name nellie.example.com;

    ssl_certificate /etc/ssl/nellie.crt;
    ssl_certificate_key /etc/ssl/nellie.key;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### API Authentication

Enable API key authentication:

```ini
# /etc/nellie/nellie.conf
NELLIE_API_KEY=your-secure-random-key
```

Clients must include header:
```
Authorization: Bearer your-secure-random-key
```

## Updates

### Upgrading

```bash
# Download new binary
curl -LO https://github.com/sequeldata/nellie-rs/releases/latest/download/nellie-linux-x86_64

# Stop service
sudo systemctl stop nellie

# Replace binary
sudo mv nellie-linux-x86_64 /usr/local/bin/nellie
sudo chmod +x /usr/local/bin/nellie

# Start service
sudo systemctl start nellie
```

### Rollback

```bash
# Keep previous version
sudo cp /usr/local/bin/nellie /usr/local/bin/nellie.bak

# To rollback
sudo systemctl stop nellie
sudo mv /usr/local/bin/nellie.bak /usr/local/bin/nellie
sudo systemctl start nellie
```

---

For additional support, file an issue at: https://github.com/sequeldata/nellie-rs/issues
```

**Verification Commands**:
```bash
# Create docs directory
mkdir -p docs

# Verify file created
ls -la docs/OPERATOR_GUIDE.md
```

**Success Criteria**:
- [ ] Operator guide covers deployment
- [ ] Backup procedures documented
- [ ] Troubleshooting section helpful
- [ ] Monitoring setup documented
- [ ] Commit made with message "docs: create operator guide"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `docs/OPERATOR_GUIDE.md` (X lines)
- **Files Modified**: None
- **Tests**: N/A
- **Build**: N/A
- **Branch**: feature/5-2-documentation
- **Notes**: (any additional context)

---

### Task 5.2 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] README comprehensive
- [ ] Operator guide complete
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Phase 5 Complete

**Phase 5 Checklist**:
- [ ] Task 5.1 merged (systemd, cross-compile)
- [ ] Task 5.2 merged (README, operator guide)
- [ ] Release binary builds
- [ ] Documentation complete

---

## Project Complete

**Final Checklist**:
- [ ] All 5 phases merged to main
- [ ] All tests pass (80+ tests)
- [ ] Release build works
- [ ] Documentation complete
- [ ] Ready for production deployment

**Run the verifier agent to validate:**
```
Use the nellie-rs-verifier agent to verify the implementation
```

---

*Phase 5 Plan - Nellie Production*
