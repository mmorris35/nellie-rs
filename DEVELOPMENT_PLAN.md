# Nellie Production - Development Plan

## How to Use This Plan

**For Claude Code**: Read this plan, find the subtask ID from the prompt, complete ALL checkboxes, update completion notes, commit.

**For You**: Use the executor agent to implement subtasks:
```
Use the nellie-rs-executor agent to execute subtask X.Y.Z
```

---

## Project Overview

**Project Name**: Nellie Production
**Goal**: Production-grade semantic code memory system for enterprise engineering teams
**Language**: Rust
**Timeline**: 7 weeks

**MVP Features**:
1. MCP server with semantic code search
2. Lessons learned storage and retrieval
3. Agent checkpoint system
4. File watcher with incremental indexing
5. Health and observability endpoints

---

## Technology Stack

- **Language**: Rust (2021 edition)
- **MCP Protocol**: rmcp 0.8+
- **HTTP Server**: axum 0.8+
- **Async Runtime**: tokio 1.0+
- **Vector Storage**: SQLite + sqlite-vec
- **Embeddings**: ort (ONNX Runtime) 2.0+
- **File Watching**: notify 6.0+
- **CLI**: clap 4.0+
- **Testing**: cargo test + criterion (benchmarks)
- **CI/CD**: GitHub Actions

---

## Progress Tracking

### Phase 0: Foundation
- [ ] 0.1.1: Initialize Rust project
- [ ] 0.1.2: Configure CI/CD
- [ ] 0.1.3: Add development tooling

### Phase 1: Storage Layer
- [ ] 1.1.1: SQLite schema and migrations
- [ ] 1.1.2: sqlite-vec integration
- [ ] 1.1.3: CRUD operations for all entities

### Phase 2: Embedding Worker
- [ ] 2.1.1: ONNX Runtime setup
- [ ] 2.1.2: Embedding worker thread pool
- [ ] 2.1.3: Batch embedding API

### Phase 3: MCP Server
- [ ] 3.1.1: rmcp server setup
- [ ] 3.1.2: Tool implementations
- [ ] 3.1.3: HTTP+SSE transport with graceful disconnect

### Phase 4: File Watcher
- [ ] 4.1.1: Directory watching with notify
- [ ] 4.1.2: Incremental indexing logic
- [ ] 4.1.3: Gitignore-aware filtering

### Phase 5: Integration & Hardening
- [ ] 5.1.1: End-to-end integration tests
- [ ] 5.1.2: Stress testing and benchmarks
- [ ] 5.1.3: Health checks and metrics

### Phase 6: Packaging
- [ ] 6.1.1: Release binary builds
- [ ] 6.1.2: Systemd service files
- [ ] 6.1.3: Documentation and README

**Current**: Phase 0
**Next**: 0.1.1

---

## Phase 0: Foundation

**Goal**: Set up Rust project with CI/CD and development tooling
**Duration**: 3-5 days

### Task 0.1: Project Setup

**Git**: Create branch `feature/0-1-project-setup`

---

**Subtask 0.1.1: Initialize Rust Project**

**Prerequisites**: None

**Deliverables**:
- [ ] Run `cargo init --name nellie`
- [ ] Configure Cargo.toml with workspace metadata
- [ ] Add initial dependencies to Cargo.toml
- [ ] Create src/lib.rs with module structure
- [ ] Create src/main.rs with clap CLI skeleton
- [ ] Add .gitignore for Rust
- [ ] Update README.md with project description

**Cargo.toml dependencies**:
```toml
[package]
name = "nellie"
version = "0.1.0"
edition = "2021"
description = "Production-grade semantic code memory system"
license = "MIT"
repository = "https://github.com/mmorris35/nellie-rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# MCP Protocol
rmcp = { version = "0.8", features = ["server"] }

# HTTP Server  
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Storage
rusqlite = { version = "0.32", features = ["bundled"] }
# sqlite-vec = "0.1"  # Add when implementing storage

# Embeddings
# ort = "2"  # Add when implementing embeddings

# File watching
notify = "6"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CLI
clap = { version = "4", features = ["derive"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
criterion = "0.5"

[[bench]]
name = "search_benchmark"
harness = false

[profile.release]
lto = true
codegen-units = 1
strip = true
```

**src/lib.rs skeleton**:
```rust
//! Nellie - Production-grade semantic code memory system

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod embeddings;
pub mod server;
pub mod storage;
pub mod watcher;

pub use config::Config;
```

**src/main.rs skeleton**:
```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "nellie")]
#[command(about = "Production-grade semantic code memory system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8765")]
        port: u16,
        
        /// Data directory
        #[arg(short, long, default_value = "./data")]
        data_dir: String,
    },
    /// Index a directory
    Index {
        /// Directory to index
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "nellie=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, data_dir } => {
            tracing::info!("Starting Nellie server on port {}", port);
            // TODO: Implement server
            todo!("Implement server")
        }
        Commands::Index { path } => {
            tracing::info!("Indexing directory: {}", path);
            // TODO: Implement indexer
            todo!("Implement indexer")
        }
    }
}
```

**Success Criteria**:
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` has no warnings
- [ ] `cargo run -- --help` shows CLI help
- [ ] Module structure matches CLAUDE.md

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: (list with line counts)
- **Files Modified**: (list)
- **Tests**: N/A (setup)
- **Build**: (pass/fail)
- **Branch**: feature/0-1-project-setup
- **Notes**: (any additional context)

---

**Subtask 0.1.2: Configure CI/CD**

**Prerequisites**:
- [x] 0.1.1: Initialize Rust Project

**Deliverables**:
- [ ] Create .github/workflows/ci.yml
- [ ] Configure build matrix (Linux x86_64, Linux ARM64, macOS ARM64)
- [ ] Add test, clippy, and fmt checks
- [ ] Configure release builds

**.github/workflows/ci.yml**:
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      
      - name: Format check
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy -- -D warnings
      
      - name: Test
        run: cargo test

  build:
    needs: check
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      
      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: nellie-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/nellie
```

**Success Criteria**:
- [ ] CI workflow file exists
- [ ] Workflow passes on push to main

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: (list)
- **Files Modified**: (list)
- **Tests**: N/A
- **Build**: (CI status)
- **Branch**: feature/0-1-project-setup
- **Notes**: (any additional context)

---

**Subtask 0.1.3: Add Development Tooling**

**Prerequisites**:
- [x] 0.1.2: Configure CI/CD

**Deliverables**:
- [ ] Create rust-toolchain.toml pinning Rust version
- [ ] Create .cargo/config.toml with build settings
- [ ] Add clippy.toml with lint configuration
- [ ] Create justfile for common commands
- [ ] Set up pre-commit hooks

**rust-toolchain.toml**:
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

**justfile**:
```just
# Default recipe
default: check

# Run all checks
check: fmt-check lint test

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt --check

# Run clippy
lint:
    cargo clippy -- -D warnings

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Build release
build:
    cargo build --release

# Run the server
serve port="8765":
    cargo run -- serve --port {{port}}

# Clean build artifacts
clean:
    cargo clean

# Generate docs
docs:
    cargo doc --open
```

**Success Criteria**:
- [ ] `just check` runs all checks
- [ ] `just build` produces release binary
- [ ] Rust toolchain pinned

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: (list)
- **Files Modified**: (list)
- **Tests**: N/A
- **Build**: (pass/fail)
- **Branch**: feature/0-1-project-setup
- **Notes**: (any additional context)

---

### Task 0.1 Complete - Squash Merge
- [ ] All subtasks complete (0.1.1 - 0.1.3)
- [ ] All checks pass
- [ ] Squash merge to main: `git checkout main && git merge --squash feature/0-1-project-setup`
- [ ] Push to remote: `git push origin main`
- [ ] Delete feature branch

---

## Phase 1: Storage Layer

**Goal**: Implement SQLite storage with vector similarity search
**Duration**: 1 week

### Task 1.1: SQLite + sqlite-vec

**Git**: Create branch `feature/1-1-storage`

---

**Subtask 1.1.1: SQLite Schema and Migrations**

**Prerequisites**:
- [x] 0.1.3: Add Development Tooling

**Deliverables**:
- [ ] Create src/storage/mod.rs
- [ ] Create src/storage/schema.rs with table definitions
- [ ] Implement schema initialization
- [ ] Add migration support for future updates
- [ ] Write unit tests

**Schema** (from PROJECT_BRIEF.md):
```sql
CREATE TABLE chunks (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,
    file_hash TEXT NOT NULL,
    indexed_at INTEGER NOT NULL
);

CREATE TABLE lessons (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,
    tags TEXT,
    severity TEXT DEFAULT 'info',
    created_at INTEGER NOT NULL
);

CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE file_state (
    path TEXT PRIMARY KEY,
    mtime INTEGER NOT NULL,
    hash TEXT NOT NULL
);

CREATE INDEX idx_chunks_file_path ON chunks(file_path);
CREATE INDEX idx_chunks_file_hash ON chunks(file_hash);
CREATE INDEX idx_lessons_severity ON lessons(severity);
CREATE INDEX idx_checkpoints_agent ON checkpoints(agent);
```

**Success Criteria**:
- [ ] Schema creates all tables
- [ ] Indexes created
- [ ] Unit tests pass
- [ ] Schema is idempotent (safe to run multiple times)

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: (list with line counts)
- **Files Modified**: (list)
- **Tests**: (count)
- **Build**: (pass/fail)
- **Branch**: feature/1-1-storage
- **Notes**: (any additional context)

---

**Subtask 1.1.2: sqlite-vec Integration**

**Prerequisites**:
- [x] 1.1.1: SQLite Schema

**Deliverables**:
- [ ] Add sqlite-vec dependency
- [ ] Create vector table for embeddings
- [ ] Implement similarity search function
- [ ] Benchmark search performance
- [ ] Write unit tests

**Success Criteria**:
- [ ] Vector similarity search works
- [ ] Search returns results sorted by similarity
- [ ] Benchmark shows <100ms for 100K vectors

---

**Completion Notes**: (fill in when complete)

---

**Subtask 1.1.3: CRUD Operations**

**Prerequisites**:
- [x] 1.1.2: sqlite-vec Integration

**Deliverables**:
- [ ] Create src/storage/queries.rs
- [ ] Implement chunk CRUD (create, read, update, delete)
- [ ] Implement lesson CRUD
- [ ] Implement checkpoint CRUD
- [ ] Implement file_state CRUD
- [ ] Write comprehensive tests

**Success Criteria**:
- [ ] All CRUD operations work
- [ ] Tests cover success, failure, edge cases
- [ ] Transactions used where appropriate

---

**Completion Notes**: (fill in when complete)

---

### Task 1.1 Complete - Squash Merge
- [ ] All subtasks complete
- [ ] All tests pass
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete feature branch

---

## Phase 2: Embedding Worker

**Goal**: ONNX-based embedding generation on dedicated thread pool
**Duration**: 1 week

*(Subtasks to be detailed when Phase 1 is complete)*

---

## Phase 3: MCP Server

**Goal**: Full MCP implementation with rmcp
**Duration**: 1.5 weeks

*(Subtasks to be detailed when Phase 2 is complete)*

---

## Phase 4: File Watcher

**Goal**: Incremental file indexing with gitignore support
**Duration**: 1 week

*(Subtasks to be detailed when Phase 3 is complete)*

---

## Phase 5: Integration & Hardening

**Goal**: Stress testing, benchmarks, observability
**Duration**: 1 week

*(Subtasks to be detailed when Phase 4 is complete)*

---

## Phase 6: Packaging

**Goal**: Release builds, systemd, documentation
**Duration**: 0.5 weeks

*(Subtasks to be detailed when Phase 5 is complete)*

---

## Success Criteria (MVP)

1. [ ] 72-hour stress test with no restarts required
2. [ ] <200ms p95 query latency at 1M chunks
3. [ ] Successful deployment on Sequel Data ESXi
4. [ ] Claude Code can connect and search via MCP
5. [ ] Zero external runtime dependencies (single binary)

---

*Generated for Nellie Production (Rust) - 2026-02-02*
