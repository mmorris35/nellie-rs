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
