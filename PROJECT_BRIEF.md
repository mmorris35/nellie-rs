# PROJECT_BRIEF.md — Nellie Production

## Project Overview

**Name:** Nellie Production
**Type:** API / Backend Service
**Goal:** Production-grade semantic code memory system for enterprise engineering teams — air-gapped, reliable, and low-maintenance.

## Problem Statement

Engineering teams solving similar problems across multiple projects waste time re-discovering solutions that already exist in their codebase. Existing code search (grep, GitHub search) is keyword-based and misses semantic matches.

The current Nellie prototype proves the concept works but has reliability issues:
- MCP streaming HTTP leaks resources on client disconnect
- In-process embeddings cause GIL contention and semaphore leaks
- Multiple processes (server, ChromaDB, daemon) create operational complexity
- Python async + heavy compute is an unstable combination

## Target Users

- **Primary:** Enterprise engineering teams (5-100 developers) sharing a codebase
- **Secondary:** Sequel Data internal use, customer deployments
- **Operator:** IT/DevOps deploying as VM appliance

## Core Features (MVP)

### 1. Semantic Code Search
- Natural language queries across indexed repositories
- Returns relevant code snippets with file paths and context
- Configurable result limits and similarity thresholds

### 2. Lessons Learned
- Store and retrieve engineering lessons (gotchas, patterns, fixes)
- Tag-based organization
- Search by natural language or tags

### 3. Agent Checkpoints
- Save/restore AI agent working state
- Track what agents are working on
- Support multiple agents per deployment

### 4. Repository Indexing
- Watch directories for file changes
- Incremental indexing (only changed files)
- Support for common code file types
- Configurable ignore patterns (.gitignore aware)

### 5. REST API
- Simple REST endpoints (not MCP streaming)
- OpenAPI/Swagger documentation
- Optional: MCP adapter layer for Claude Code compatibility

### 6. Health & Observability
- Health check endpoint
- Prometheus metrics (indexed files, query latency, queue depth)
- Structured logging

## Technical Requirements

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Nellie Production                         │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   REST API   │    │   Embedding  │    │   File Watcher   │  │
│  │   (Axum)     │───▶│   Worker     │    │   (notify-rs)    │  │
│  │              │    │   (Queue)    │    │                  │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│         │                   │                    │              │
│         ▼                   ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              SQLite + sqlite-vec (embedded)              │   │
│  │         Vector storage + metadata + FTS search           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Language & Runtime

- **Language:** Rust
- **Why Rust:**
  - No GIL — true parallelism
  - Proper async (tokio) — no resource leaks
  - Single binary deployment — no Python environment
  - Memory safety — no segfaults or leaks
  - Performance — fast startup, low memory

### Key Dependencies

| Component | Library | Rationale |
|-----------|---------|-----------|
| **MCP Protocol** | rmcp | Official Anthropic MCP SDK for Rust |
| HTTP Server | axum | Fast, ergonomic, tower ecosystem |
| Async Runtime | tokio | Industry standard (rmcp requires it) |
| Vector Storage | sqlite-vec | Embedded, no separate process |
| Embeddings | ort (ONNX Runtime) | Rust bindings, no Python dependency |
| File Watching | notify | Cross-platform, efficient |
| Serialization | serde | Standard |
| CLI | clap | Standard |

### Embedding Model

- **Model:** all-MiniLM-L6-v2 or nomic-embed-text (ONNX format)
- **Inference:** ONNX Runtime via `ort` crate
- **Threading:** Dedicated embedding thread pool (not on async runtime)

### Storage

- **SQLite** with **sqlite-vec** extension for vector similarity
- Single file database — easy backup, no connection management
- Schema:
  ```sql
  -- Code chunks
  CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- sqlite-vec
    file_hash TEXT NOT NULL,
    indexed_at INTEGER NOT NULL
  );
  
  -- Lessons
  CREATE TABLE lessons (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,
    tags TEXT,  -- JSON array
    severity TEXT DEFAULT 'info',
    created_at INTEGER NOT NULL
  );
  
  -- Checkpoints
  CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    state TEXT NOT NULL,  -- JSON
    created_at INTEGER NOT NULL
  );
  
  -- File state (for incremental indexing)
  CREATE TABLE file_state (
    path TEXT PRIMARY KEY,
    mtime INTEGER NOT NULL,
    hash TEXT NOT NULL
  );
  ```

### API: MCP Protocol (Primary)

**SDK:** Official `rmcp` crate v0.8.0+ (`modelcontextprotocol/rust-sdk`)

MCP is the primary API — not optional, not an afterthought.

**MCP Tools Exposed:**
```
search_code         Semantic code search
search_lessons      Search lessons by query
list_lessons        List all lessons
add_lesson          Create a lesson
delete_lesson       Delete a lesson
get_checkpoint      Get agent checkpoint
add_checkpoint      Create checkpoint
trigger_reindex     Force re-index of path
get_status          Server status and stats
```

**Transport:** HTTP+SSE (Streamable HTTP) — same as Python MCP SDK

**Key Implementation Requirements:**
- Graceful client disconnect handling (no resource leaks)
- Request timeouts with proper cleanup
- Embeddings off the async runtime (dedicated thread pool)
- Connection lifecycle properly managed via tower middleware

### REST API (Secondary)

Simple REST endpoints for non-MCP clients and health checks:

```
GET  /health                    Health check
GET  /metrics                   Prometheus metrics
POST /api/v1/search/code        Semantic code search (REST fallback)
```

## Non-Functional Requirements

### Reliability
- No resource leaks under sustained load
- Graceful handling of client disconnects
- Automatic recovery from transient failures
- Zero-downtime restarts

### Performance
- Query latency: <200ms p95 for 1M chunks
- Indexing throughput: 1000+ files/minute
- Memory usage: <2GB for 1M chunks
- Startup time: <10s cold start

### Operations
- Single binary + single data file
- No external dependencies (embedded everything)
- Systemd service with watchdog
- Graceful shutdown with state persistence

### Security
- No network calls (air-gap compatible)
- Optional API key authentication
- Read-only repository access
- Audit logging

## Nice-to-Have (v2)

- Web UI for search and lessons management
- Clustering/replication for large deployments
- Incremental backup/restore
- IDE plugins (VS Code, JetBrains)
- GPU acceleration for embeddings
- Multi-tenant support

## Constraints

- Must run on Linux x86_64 and ARM64 (ESXi VMs, Mac Mini)
- Must work air-gapped (no cloud APIs)
- Must handle 1M+ code chunks (enterprise scale)
- Single-node only for v1 (no distributed)

## Timeline

- **Phase 0:** Project setup, CI/CD, architecture scaffolding — 1 week
- **Phase 1:** Core search + storage — 2 weeks
- **Phase 2:** File watcher + incremental indexing — 1 week
- **Phase 3:** Lessons + checkpoints — 1 week
- **Phase 4:** API hardening + observability — 1 week
- **Phase 5:** Packaging + OVA + docs — 1 week

**Total:** 7 weeks to production-ready v1

## Success Criteria

1. 72-hour stress test with no restarts required
2. <200ms p95 query latency at 1M chunks
3. Successful deployment on Sequel Data ESXi
4. Claude Code can connect and search (via MCP adapter)
5. Zero external runtime dependencies

---

*This brief is ready for DevPlan MCP to generate a DEVELOPMENT_PLAN.md*
