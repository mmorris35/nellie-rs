# Nellie-RS Development Checkpoint

**Date**: 2026-02-03
**Session**: Full MVP implementation + post-MVP fixes
**Agent**: Claude Opus 4.5

---

## Project Status: MVP COMPLETE + Critical Bug Outstanding

Nellie Production (Rust rewrite) has completed all 5 development phases with 244 tests passing. However, a critical bug prevents the core feature from working.

---

## Completed Work

### Phase 0-5: Full MVP Implementation ✅

All phases complete and merged to main:
- **Phase 0**: Foundation (project setup)
- **Phase 1**: Core Infrastructure (SQLite + sqlite-vec, ONNX embeddings)
- **Phase 2**: Indexer (file watcher, chunker, indexer pipeline)
- **Phase 3**: Features (search, lessons, checkpoints)
- **Phase 4**: MCP & REST API (11 tools, health/metrics endpoints)
- **Phase 5**: Packaging & Documentation (CLI, systemd, cross-compilation)

### Post-MVP Fixes ✅

| Issue | Status | Commit |
|-------|--------|--------|
| #9: macOS deployment | ✅ COMPLETE | b52cc94 |
| #10: EmbeddingService init | ✅ COMPLETE | 5c79109 |
| sqlite-vec initialization | ✅ COMPLETE | 82ba25d |
| API key authentication | ✅ COMPLETE | 517b7da |
| Benchmarks | ✅ COMPLETE | 57bc3a2 |
| Missing MCP tools (3) | ✅ COMPLETE | 5bf6f9e |
| CLI stubs removed | ✅ COMPLETE | c6f29c9 |

### Test & Build Status ✅

- **Tests**: 244 passing (232 lib + 9 CLI + 3 integration)
- **Clippy**: Clean (no warnings)
- **Binary**: 6.3MB release build
- **Benchmarks**: vector_search @ 10K chunks = 37ms (under 200ms target)

---

## Outstanding Critical Issue

### Issue #13: Watcher/Indexer Not Started 🔴

**Status**: NOT IMPLEMENTED
**Priority**: CRITICAL
**GitHub**: https://github.com/mmorris35/nellie-rs/issues/13

**Problem**: The file watcher/indexer daemon is never started. The code exists in `src/watcher/` but is not wired up to server startup.

**Impact**:
- `search_code` always returns empty results
- `get_status` shows 0 files, 0 chunks
- `trigger_reindex` clears DB but nothing re-indexes
- **Nellie cannot index or search code**

**Required Fix**:
1. Start watcher on server startup
2. Do initial full scan of watch directories
3. Continue watching for file changes
4. Index files as they are discovered/changed

**Files to Modify**:
- `src/main.rs` - Start watcher alongside server
- `src/server/app.rs` - Integrate watcher state

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                        Nellie Production                        │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   MCP API    │    │   Embedding  │    │   File Watcher   │  │
│  │   (11 tools) │───▶│   Worker     │    │   (NOT STARTED)  │  │
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

---

## MCP Tools (11 total)

| Tool | Status | Notes |
|------|--------|-------|
| search_code | ⚠️ | Returns empty (no indexed data) |
| search_lessons | ✅ | Works |
| search_checkpoints | ✅ | Works |
| list_lessons | ✅ | Works |
| add_lesson | ✅ | Works |
| delete_lesson | ✅ | Works |
| add_checkpoint | ✅ | Works |
| get_recent_checkpoints | ✅ | Works |
| get_agent_status | ✅ | Works |
| trigger_reindex | ⚠️ | Clears DB but doesn't re-index |
| get_status | ⚠️ | Shows 0 chunks (nothing indexed) |

---

## File Structure

```
nellie-rs/
├── src/
│   ├── main.rs              # CLI + server startup
│   ├── lib.rs               # Library root
│   ├── config/              # Configuration
│   ├── error/               # Error types
│   ├── storage/             # SQLite + sqlite-vec (WORKING)
│   ├── embeddings/          # ONNX embeddings (WORKING)
│   ├── watcher/             # File watcher (EXISTS, NOT STARTED)
│   └── server/              # MCP + REST API (WORKING)
├── tests/                   # Integration tests
├── benches/                 # Criterion benchmarks
├── packaging/
│   ├── macos/               # launchd + migration scripts
│   ├── nellie.service       # systemd service
│   └── nellie.conf          # Default config
├── docs/
│   └── OPERATOR_GUIDE.md    # Operations documentation
├── CLAUDE.md                # Development guidelines
├── PROJECT_BRIEF.md         # Requirements
├── DEVELOPMENT_PLAN.md      # Implementation plan
└── README.md                # User documentation
```

---

## Git Status

**Branch**: main
**Latest Commit**: 783e7ef
**Remote**: https://github.com/mmorris35/nellie-rs.git

Recent commits:
```
783e7ef docs: Update Issue #10 status to COMPLETED
5c79109 fix(embeddings): Initialize EmbeddingService on server startup
d6ccb97 docs: Add post-MVP implementation records for Issues #9 and #10
b52cc94 feat(packaging): Add macOS deployment for Mac Mini (Issue #9)
82ba25d fix(storage): Initialize sqlite-vec BEFORE creating database connections
```

---

## Next Steps

1. **CRITICAL**: Implement Issue #13 - Wire up watcher/indexer to server startup
2. Deploy to Mac Mini (mini-dev-server) for testing
3. Run migration from Python Nellie
4. Perform 72-hour stress test
5. Verify <200ms p95 latency at 1M chunks

---

## Resume Instructions

To continue this work:

```
# Implement Issue #13
Use the nellie-rs-executor agent to implement Issue #13: Wire up watcher/indexer to server startup

# The fix requires:
1. Start watcher on server startup in src/main.rs
2. Pass watch directories from CLI args
3. Integrate with embedding service for indexing
4. Initial full scan + continuous file watching
```

---

*Checkpoint created by Claude Opus 4.5*
