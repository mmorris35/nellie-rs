# Phase 6: Wire up Watcher/Indexer (Issue #13 - CRITICAL)

**Priority**: 🔴 CRITICAL - Core feature was broken
**Goal**: Start file watcher and indexer on server startup so code actually gets indexed
**GitHub Issue**: https://github.com/mmorris35/nellie-rs/issues/13
**Status**: ✅ COMPLETE (2026-02-04)

---

## Problem Summary

The watcher module code existed but was never started:
- `FileWatcher` exists in `src/watcher/watcher.rs`
- `EventHandler` exists in `src/watcher/handler.rs`
- `Indexer` exists in `src/watcher/indexer.rs`

But these were never wired up to server startup. Result: `search_code` always returned empty.

---

## Task 6.1: Wire up Watcher Pipeline to Server Startup

**Git Branch**: `fix/13-watcher-startup` (merged to main)
**Commit**: 890dc00

---

### Subtask 6.1.1: Add watcher state and startup to App ✅

**Goal**: Modify `App` to start the watcher/indexer pipeline when watch directories are specified.

**Files Modified**:
- `src/server/app.rs`

**Changes**:

1. Added imports:
```rust
use crate::watcher::{
    EventHandler, FileWatcher, HandlerConfig, Indexer, WatcherConfig, WatcherStats,
};
use tokio::sync::mpsc;
```

2. Added `watch_dirs` field to `ServerConfig`:
```rust
pub struct ServerConfig {
    // ... existing fields ...
    /// Directories to watch for code changes
    pub watch_dirs: Vec<std::path::PathBuf>,
}
```

3. Implemented `App::start_watcher()`:
```rust
/// Start the file watcher and indexer pipeline.
async fn start_watcher(
    &self,
    watch_dirs: Vec<std::path::PathBuf>,
) -> Result<Option<(
    tokio::task::JoinHandle<()>,
    tokio::task::JoinHandle<()>,
)>>
```

4. Implemented `App::initial_scan()`:
```rust
/// Perform initial scan of a directory.
async fn initial_scan(
    &self,
    dir: &std::path::Path,
    index_tx: &mpsc::Sender<crate::watcher::IndexRequest>,
) -> Result<()>
```

**Success Criteria**: ✅ All met
- [x] `ServerConfig` has `watch_dirs` field
- [x] `App::start_watcher()` method exists
- [x] `App::initial_scan()` method exists

---

### Subtask 6.1.2: Add walkdir dependency and fix module access ✅

**Goal**: Add walkdir crate for directory traversal and fix module visibility.

**Files Modified**:
- `Cargo.toml`
- `src/watcher/mod.rs`

**Changes**:

1. Added to `Cargo.toml`:
```toml
walkdir = "2"
```

2. Updated `src/watcher/mod.rs` to re-export `IndexRequest`:
```rust
pub use handler::{EventHandler, HandlerConfig, IndexRequest, WatcherStats, WatcherStatsSnapshot};
```

**Success Criteria**: ✅ All met
- [x] `walkdir` added to Cargo.toml
- [x] `IndexRequest` re-exported from watcher module
- [x] `cargo check` passes

---

### Subtask 6.1.3: Update main.rs to start watcher with server ✅

**Goal**: Pass watch directories to App and start watcher pipeline.

**Files Modified**:
- `src/main.rs`

**Changes**:

Updated `serve_command()` to:
1. Pass `watch_dirs` to `ServerConfig`
2. Call `app.start_watcher()` before `app.run()`
3. Store watcher handles to prevent premature drop

```rust
// Create server config
let server_config = ServerConfig {
    // ... other fields ...
    watch_dirs: args.watch,
};

// Create app
let app = App::new(server_config.clone(), db).await?;

// Start watcher if directories specified
let _watcher_handles = app.start_watcher(server_config.watch_dirs.clone()).await?;

// Run server (blocks until shutdown)
app.run().await
```

**Success Criteria**: ✅ All met
- [x] `serve_command` passes `watch_dirs` to `ServerConfig`
- [x] `app.start_watcher()` called before `app.run()`
- [x] Watcher handles stored to prevent drop
- [x] `cargo build --release` succeeds

---

### Subtask 6.1.4: Add McpState accessor methods ✅

**Goal**: Add methods to `McpState` to access database and embedding service for indexer.

**Files Modified**:
- `src/server/mcp.rs`

**Changes**:

```rust
impl McpState {
    /// Get the database.
    #[must_use]
    pub const fn db(&self) -> &Database {
        &self.db
    }

    /// Get the embedding service if available.
    #[must_use]
    pub fn embedding_service(&self) -> Option<EmbeddingService> {
        self.embedding_service.clone()
    }
}
```

**Success Criteria**: ✅ All met
- [x] `McpState::db()` returns reference to database
- [x] `McpState::embedding_service()` returns optional service
- [x] `cargo check` passes

---

### Subtask 6.1.5: Write integration tests for watcher startup ✅

**Goal**: Add tests verifying watcher indexes files on startup.

**Files Created**:
- `tests/watcher_integration.rs` (174 lines)

**Tests Added**:
1. `test_initial_scan_indexes_files` - Verifies files are indexed on startup
2. `test_file_filter_excludes_binaries` - Verifies non-code files are filtered
3. `test_gitignore_patterns` - Verifies .gitignore patterns are respected
4. `test_watcher_stats_tracking` - Verifies stats are properly tracked
5. `test_event_batch_operations` - Verifies EventBatch operations

**Success Criteria**: ✅ All met
- [x] Test file created at `tests/watcher_integration.rs`
- [x] All 5 integration tests pass
- [x] All existing tests still pass

---

### Subtask 6.1.6: Final verification and commit ✅

**Verification Results**:
- `cargo fmt`: ✅ Clean
- `cargo clippy -- -D warnings`: ✅ Clean (fixed 4 warnings)
- `cargo test`: ✅ 249 tests passing
- `cargo build --release`: ✅ 9.4MB binary

**Git Commit**:
```
fix(watcher): Wire up file watcher and indexer to server startup (Issue #13)

CRITICAL FIX: Core feature was broken - code was never indexed.

- Add watch_dirs to ServerConfig
- Implement App::start_watcher() to create watcher pipeline
- Implement App::initial_scan() for startup indexing
- Add McpState accessor methods for db/embeddings
- Start watcher before server when --watch specified
- Add walkdir dependency for directory traversal
- Add integration tests for watcher functionality

Fixes #13
```

---

## Completion Notes

**Implementation Summary**:
- Added `watch_dirs` field to `ServerConfig`
- Implemented `App::start_watcher()` to create and start the watcher/indexer pipeline
- Implemented `App::initial_scan()` to perform directory scanning on startup
- Added `McpState::db()` and `McpState::embedding_service()` accessors
- Added walkdir dependency for directory traversal
- Added 5 comprehensive integration tests

**Files Created**:
- `tests/watcher_integration.rs` (174 lines)

**Files Modified**:
- `src/server/app.rs` (+140 lines)
- `src/main.rs` (+14 lines)
- `src/server/mcp.rs` (+12 lines)
- `src/watcher/mod.rs` (+3 lines)
- `Cargo.toml` (+1 line)

**Tests**: 249 total (232 lib + 9 main + 5 watcher integration + 3 other)

**Build**: ✅ Success
- Binary: 9.4MB release build
- Clippy: Clean
- Format: Clean

**Verified Working**:
```
INFO nellie::server::app: Starting file watcher watch_dirs=["/tmp/test"]
INFO nellie::watcher::indexer: Indexer started
INFO nellie::watcher::watcher: Watching directory path=/tmp/test
INFO nellie::server::app: Performing initial scan of watch directories
INFO nellie::server::app: Initial scan complete dir=/tmp/test files=1
```

---

## Impact

- **Issue #13 FIXED**: File watcher and indexer now properly wire up on server startup
- **Core Feature Enabled**: `search_code` now returns indexed results
- **Incremental Indexing**: File changes trigger re-indexing
- **Graceful Degradation**: Works without embeddings (useful for testing)
- **No Regressions**: All existing tests continue to pass

---

*Phase 6 completed by Claude Opus 4.5 - 2026-02-04*
