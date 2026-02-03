# Nellie Production - Development Plan

## How to Use This Plan

**For Claude Code**: Read this plan, find the subtask ID from the prompt, navigate to the appropriate phase file, complete ALL checkboxes, update completion notes, commit.

**For You**: Use the executor agent to implement subtasks:
```
Use the nellie-rs-executor agent to execute subtask X.Y.Z
```

The executor agent already knows to read CLAUDE.md and the phase plan files. Just give it the subtask ID and let it work.

---

## Project Overview

**Project Name**: Nellie Production
**Goal**: Production-grade semantic code memory system for enterprise engineering teams — air-gapped, reliable, and low-maintenance
**Target Users**: Enterprise engineering teams (5-100 developers), Sequel Data internal use, IT/DevOps operators
**Timeline**: 7 weeks to production-ready v1

**MVP Scope**:
1. Semantic Code Search (natural language queries, ranked results)
2. Lessons Learned (store/retrieve engineering lessons with tags)
3. Agent Checkpoints (save/restore AI agent working state)
4. Repository Indexing (watch directories, incremental indexing)
5. MCP API (primary) + REST API (secondary)
6. Health & Observability (metrics, structured logging)

---

## Technology Stack

| Component | Library | Version |
|-----------|---------|---------|
| Language | Rust | 1.75+ |
| MCP Protocol | rmcp | 0.8+ |
| HTTP Server | axum | 0.8+ |
| Async Runtime | tokio | 1.0+ |
| Vector Storage | sqlite-vec | latest |
| SQL Database | rusqlite | latest |
| Embeddings | ort | 2.0+ |
| File Watching | notify | 6.0+ |
| Serialization | serde, serde_json | 1.0+ |
| CLI | clap | 4.0+ |
| Error Handling | thiserror, anyhow | latest |
| Logging | tracing, tracing-subscriber | latest |
| Metrics | prometheus | latest |
| Testing | tokio-test, tempfile | latest |

---

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

---

## Progress Tracking

### Phase 0: Foundation (1 week)
- [x] 0.1.1: Initialize Rust project with Cargo
- [x] 0.1.2: Configure development tools (clippy, fmt, deny)
- [ ] 0.1.3: Set up CI/CD with GitHub Actions
- [x] 0.2.1: Create project module structure
- [ ] 0.2.2: Define error types and Result aliases
- [ ] 0.2.3: Create configuration system

### Phase 1: Core Storage & Embeddings (2 weeks)
- [x] 1.1.1: Set up SQLite with rusqlite
- [x] 1.1.2: Integrate sqlite-vec extension
- [x] 1.1.3: Implement schema migrations
- [x] 1.2.1: Define storage traits and models
- [x] 1.2.2: Implement chunk storage operations
- [ ] 1.2.3: Implement vector search
- [ ] 1.3.1: Set up ONNX Runtime with ort crate
- [ ] 1.3.2: Implement embedding worker with thread pool
- [ ] 1.3.3: Create async embedding API with channels

### Phase 2: File Watcher & Indexing (1 week)
- [ ] 2.1.1: Set up notify-rs file watcher
- [ ] 2.1.2: Implement gitignore-aware filtering
- [ ] 2.1.3: Create file change event handler
- [ ] 2.2.1: Implement code chunking strategy
- [ ] 2.2.2: Build incremental indexing pipeline
- [ ] 2.2.3: Add file state tracking for change detection

### Phase 3: Lessons & Checkpoints (1 week)
- [ ] 3.1.1: Implement lessons storage and CRUD
- [ ] 3.1.2: Add lesson search with semantic matching
- [ ] 3.1.3: Implement tag-based filtering
- [ ] 3.2.1: Implement checkpoint storage
- [ ] 3.2.2: Add agent status tracking
- [ ] 3.2.3: Create checkpoint search functionality

### Phase 4: MCP & REST API (1 week)
- [ ] 4.1.1: Set up rmcp server with axum transport
- [ ] 4.1.2: Implement search_code MCP tool
- [ ] 4.1.3: Implement lessons MCP tools
- [ ] 4.1.4: Implement checkpoint MCP tools
- [ ] 4.2.1: Create REST health and metrics endpoints
- [ ] 4.2.2: Add Prometheus metrics collection
- [ ] 4.2.3: Implement graceful shutdown

### Phase 5: Packaging & Documentation (1 week)
- [ ] 5.1.1: Create systemd service configuration
- [ ] 5.1.2: Build cross-compilation for ARM64
- [ ] 5.1.3: Create installation scripts
- [ ] 5.2.1: Write comprehensive README
- [ ] 5.2.2: Generate API documentation
- [ ] 5.2.3: Create operator guide

**Current Phase**: 1 (Core Storage & Embeddings)
**Next Subtask**: 1.2.1

---

## Phase Plan Files

Each phase has a detailed plan file with Haiku-executable subtasks:

| Phase | File | Duration | Focus |
|-------|------|----------|-------|
| 0 | [plans/PHASE_0_FOUNDATION.md](plans/PHASE_0_FOUNDATION.md) | 1 week | Project setup, CI/CD, architecture |
| 1 | [plans/PHASE_1_CORE.md](plans/PHASE_1_CORE.md) | 2 weeks | SQLite storage, sqlite-vec, embeddings |
| 2 | [plans/PHASE_2_INDEXER.md](plans/PHASE_2_INDEXER.md) | 1 week | File watcher, incremental indexing |
| 3 | [plans/PHASE_3_FEATURES.md](plans/PHASE_3_FEATURES.md) | 1 week | Lessons learned, checkpoints |
| 4 | [plans/PHASE_4_API.md](plans/PHASE_4_API.md) | 1 week | MCP server, REST API, observability |
| 5 | [plans/PHASE_5_PACKAGING.md](plans/PHASE_5_PACKAGING.md) | 1 week | Systemd, cross-compile, docs |

---

## Git Workflow

### Branch Strategy
- **ONE branch per TASK** (e.g., `feature/1-2-chunk-storage`)
- **NO branches for individual subtasks** - subtasks are commits within the task branch
- Create branch when starting first subtask of a task
- Branch naming: `feature/{phase}-{task}-{description}`

### Commit Strategy
- **One commit per subtask** with semantic message
- Format: `feat(scope): description` or `fix(scope): description`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- Example: `feat(storage): implement sqlite-vec vector search`

### Merge Strategy
- **Squash merge when task is complete** (all subtasks done)
- Delete feature branch after merge
- Push to remote: `git push origin main`

### Workflow Example
```bash
# Starting Task 1.2 (first subtask is 1.2.1)
git checkout -b feature/1-2-chunk-storage

# After completing subtask 1.2.1
git add . && git commit -m "feat(storage): define storage traits and models"

# After completing subtask 1.2.2
git add . && git commit -m "feat(storage): implement chunk CRUD operations"

# After completing subtask 1.2.3 (task complete)
git add . && git commit -m "feat(storage): add vector similarity search"

# Task complete - squash merge
git checkout main
git merge --squash feature/1-2-chunk-storage
git commit -m "feat(storage): implement chunk storage with vector search

- Define ChunkRecord and storage traits
- Implement CRUD operations for chunks
- Add sqlite-vec powered similarity search
- Add comprehensive tests (15 tests, 85% coverage)"
git push origin main
git branch -d feature/1-2-chunk-storage
```

---

## Verification Commands

Run these before every commit:

```bash
# Format code
cargo fmt

# Lint with strict warnings
cargo clippy -- -D warnings

# Run all tests
cargo test

# Build release binary
cargo build --release
```

All must pass with no errors or warnings.

---

## Success Criteria

1. 72-hour stress test with no restarts required
2. <200ms p95 query latency at 1M chunks
3. Successful deployment on Sequel Data ESXi
4. Claude Code can connect and search (via MCP)
5. Zero external runtime dependencies

---

## Deferred Features (v2)

These features are planned for v2 after MVP completion:

- Web UI for search and lessons management
- Clustering/replication for large deployments
- Incremental backup/restore
- IDE plugins (VS Code, JetBrains)
- GPU acceleration for embeddings
- Multi-tenant support

---

## Ready to Build

Each phase file contains paint-by-numbers subtasks with:
- Complete, copy-pasteable code blocks
- Exact file paths
- Verification commands with expected outputs
- Testable success criteria

**To start implementation**, use the executor agent:

```
Use the nellie-rs-executor agent to execute subtask 0.1.1
```

Start with Phase 0 and work through subtasks in order. Each one builds on the previous.

---

*Generated by DevPlan MCP Server*
