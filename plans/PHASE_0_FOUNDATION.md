# Phase 0: Foundation

**Goal**: Set up Rust project with proper tooling, CI/CD, and module architecture
**Duration**: 1 week
**Prerequisites**: None (first phase)

---

## Task 0.1: Project Initialization

**Git**: Create branch `feature/0-1-project-init` when starting first subtask.

### Subtask 0.1.1: Initialize Rust Project (Single Session)

**Prerequisites**:
- None (first subtask)

**Deliverables**:
- [x] Initialize Cargo project with `cargo init`
- [x] Configure Cargo.toml with all dependencies
- [x] Create .gitignore for Rust projects
- [x] Create README.md with project overview
- [x] Make initial commit

**Files to Create**:

**`Cargo.toml`** (complete file):
```toml
[package]
name = "nellie"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["Sequel Data <dev@sequeldata.com>"]
description = "Production-grade semantic code memory system"
license = "MIT"
repository = "https://github.com/sequeldata/nellie-rs"
keywords = ["semantic-search", "code-search", "mcp", "embeddings"]
categories = ["development-tools", "database"]

[dependencies]
# Async runtime
tokio = { version = "1.0", features = ["full"] }

# MCP Protocol
rmcp = { version = "0.1", features = ["server", "transport-sse-server"] }

# HTTP Server
axum = { version = "0.8", features = ["macros"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
rusqlite = { version = "0.32", features = ["bundled", "blob"] }

# Embeddings (ONNX Runtime)
ort = { version = "2.0", default-features = false, features = ["load-dynamic"] }
ndarray = "0.16"
tokenizers = "0.20"

# File watching
notify = { version = "6.0", default-features = false, features = ["macos_kqueue"] }
notify-debouncer-mini = "0.4"
ignore = "0.4"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CLI
clap = { version = "4.0", features = ["derive", "env"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Logging & Metrics
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
prometheus = "0.13"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
once_cell = "1.0"
parking_lot = "0.12"
crossbeam-channel = "0.5"
blake3 = "1.5"

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
criterion = "0.5"
insta = "1.0"

[[bench]]
name = "search_benchmark"
harness = false

[profile.release]
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 0
debug = true

[lints.rust]
unsafe_code = "deny"

[lints.clippy]
all = "deny"
pedantic = "warn"
nursery = "warn"
```

**`.gitignore`** (complete file):
```gitignore
# Rust build artifacts
/target/
**/*.rs.bk
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# Environment
.env
.env.local
.env.*.local

# OS
.DS_Store
Thumbs.db

# Nellie data
*.db
*.db-shm
*.db-wal
/data/

# ONNX models (large files)
*.onnx

# Logs
*.log
/logs/

# Coverage
/coverage/
*.profraw
*.profdata
```

**`README.md`** (complete file):
```markdown
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
```

**`LICENSE`** (complete file):
```
MIT License

Copyright (c) 2024 Sequel Data

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

**`src/main.rs`** (complete file):
```rust
//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

fn main() {
    println!("Nellie Production - Coming Soon");
}
```

**`src/lib.rs`** (complete file):
```rust
//! Nellie Production Library
//!
//! Production-grade semantic code memory system for enterprise engineering teams.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Modules will be added in subsequent subtasks
```

**Verification Commands**:
```bash
# Verify Cargo.toml is valid
cargo check 2>&1 | head -20
# Expected: "Checking nellie..." or dependency download messages

# Verify project builds
cargo build 2>&1 | tail -5
# Expected: "Finished `dev` profile [unoptimized + debuginfo] target(s)"

# Verify binary runs
cargo run 2>&1
# Expected: "Nellie Production - Coming Soon"

# Verify files exist
ls -la Cargo.toml .gitignore README.md LICENSE src/main.rs src/lib.rs
# Expected: All files listed with appropriate sizes
```

**Success Criteria**:
- [x] `cargo check` succeeds (dependencies resolve)
- [x] `cargo build` produces binary in target/debug/
- [x] `cargo run` prints "Nellie Production - Coming Soon"
- [x] `.gitignore` contains Rust-appropriate patterns
- [x] README.md has architecture diagram
- [x] First commit exists with message "chore: initialize Rust project"

---

**Completion Notes**:
- **Implementation**: Initialized Rust project with complete Cargo.toml including all production dependencies (tokio, axum, rmcp, rusqlite, ort, notify, etc.). Created .gitignore with Rust-specific patterns, README.md with project overview and architecture diagram, MIT license, and basic main.rs and lib.rs entry points. All quality checks pass.
- **Files Created**:
  - `Cargo.toml` (87 lines)
  - `.gitignore` (38 lines)
  - `README.md` (76 lines)
  - `LICENSE` (21 lines)
  - `src/main.rs` (11 lines)
  - `src/lib.rs` (9 lines)
- **Files Modified**: None
- **Tests**: N/A (setup)
- **Build**: ✅ Success (cargo check, build, run all pass; clippy and fmt clean)
- **Branch**: feature/0-1-project-init
- **Notes**: All verification commands passed successfully. Binary runs and outputs expected message. Release build completed with LTO optimization.

---

### Subtask 0.1.2: Configure Development Tools (Single Session)

**Prerequisites**:
- [x] 0.1.1: Initialize Rust Project

**Deliverables**:
- [x] Create rustfmt.toml for consistent formatting
- [x] Create clippy.toml for lint configuration
- [x] Create .cargo/config.toml for build settings
- [x] Create deny.toml for dependency auditing
- [x] Verify all tools work correctly

**Files to Create**:

**`rustfmt.toml`** (complete file):
```toml
# Rustfmt configuration for Nellie
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"

# Imports
imports_granularity = "Module"
group_imports = "StdExternalCrate"
reorder_imports = true

# Comments
wrap_comments = true
comment_width = 100
normalize_comments = true

# Formatting
format_code_in_doc_comments = true
format_strings = true
```

**`.cargo/config.toml`** (complete file):
```toml
[build]
# Use mold linker for faster builds (if available)
# rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]

[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]

[alias]
# Useful aliases
t = "test"
c = "check"
b = "build"
r = "run"
br = "build --release"
rr = "run --release"

# Combined commands
tc = "test -- --nocapture"
lint = "clippy -- -D warnings"
```

**`deny.toml`** (complete file):
```toml
# cargo-deny configuration
# Run with: cargo deny check

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "MPL-2.0",
]
copyleft = "warn"
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"

# Deny specific crates
deny = [
    # Use parking_lot instead
    # { name = "lazy_static" },
]

[sources]
unknown-registry = "deny"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

**Verification Commands**:
```bash
# Verify rustfmt works
cargo fmt --check
# Expected: No output (already formatted) or list of files to format

# Verify clippy works with our config
cargo clippy -- -D warnings 2>&1 | tail -5
# Expected: "Finished" or specific lint warnings

# Verify cargo aliases work
cargo t --help | head -3
# Expected: Shows test help (alias resolved)

# Optional: Install and run cargo-deny
# cargo install cargo-deny
# cargo deny check 2>&1 | head -10
```

**Success Criteria**:
- [x] `cargo fmt --check` runs without error
- [x] `cargo clippy -- -D warnings` runs (may have warnings to fix)
- [x] `.cargo/config.toml` has useful aliases
- [x] `deny.toml` is valid TOML
- [x] Commit made with message "chore: configure development tools"

---

**Completion Notes**:
- **Implementation**: Configured development tools with rustfmt.toml for consistent code formatting (max_width=100, Unix newlines), .cargo/config.toml with build optimization flags and cargo aliases for common commands, and deny.toml for dependency security auditing. Removed nightly-only rustfmt features to ensure stable channel compatibility. All verification commands pass successfully.
- **Files Created**:
  - `rustfmt.toml` (10 lines)
  - `.cargo/config.toml` (27 lines)
  - `deny.toml` (44 lines)
- **Files Modified**: None
- **Tests**: N/A (configuration setup)
- **Build**: ✅ cargo fmt, clippy all pass without warnings or errors
- **Branch**: feature/0-1-project-init
- **Notes**: Configuration files are stable-channel compatible. Cargo aliases tested and working. Ready for CI/CD setup in next subtask.

---

### Subtask 0.1.3: Set Up CI/CD with GitHub Actions (Single Session)

**Prerequisites**:
- [x] 0.1.2: Configure Development Tools

**Deliverables**:
- [ ] Create GitHub Actions workflow for CI
- [ ] Configure caching for faster builds
- [ ] Add matrix testing for multiple Rust versions
- [ ] Set up release workflow skeleton

**Files to Create**:

**`.github/workflows/ci.yml`** (complete file):
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Check
        run: cargo check --all-targets --all-features

  test:
    name: Test
    runs-on: ubuntu-latest
    needs: check
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --all-features --verbose

  build:
    name: Build
    runs-on: ${{ matrix.os }}
    needs: test
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: nellie-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/nellie

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Security audit
        run: cargo audit
```

**`.github/workflows/release.yml`** (complete file):
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build-release:
    name: Build Release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: nellie-linux-x86_64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: nellie-linux-aarch64
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Package
        run: |
          mkdir -p dist
          cp target/${{ matrix.target }}/release/nellie dist/${{ matrix.artifact }}
          chmod +x dist/${{ matrix.artifact }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: dist/${{ matrix.artifact }}

  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    needs: build-release
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

**`.github/dependabot.yml`** (complete file):
```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
    labels:
      - "dependencies"
      - "rust"

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 3
    labels:
      - "dependencies"
      - "ci"
```

**Verification Commands**:
```bash
# Verify YAML syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "ci.yml is valid YAML"
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "release.yml is valid YAML"
python3 -c "import yaml; yaml.safe_load(open('.github/dependabot.yml'))" && echo "dependabot.yml is valid YAML"
# Expected: "*.yml is valid YAML" for each file

# Verify directory structure
ls -la .github/workflows/
# Expected: ci.yml, release.yml listed

# Local validation of CI steps
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test
# Expected: All pass
```

**Success Criteria**:
- [ ] `.github/workflows/ci.yml` exists and is valid YAML
- [ ] `.github/workflows/release.yml` exists and is valid YAML
- [ ] `.github/dependabot.yml` exists and is valid YAML
- [ ] Local CI steps pass (fmt, clippy, test)
- [ ] Commit made with message "ci: add GitHub Actions workflows"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `.github/workflows/ci.yml` (X lines)
  - `.github/workflows/release.yml` (X lines)
  - `.github/dependabot.yml` (X lines)
- **Files Modified**: None
- **Tests**: N/A
- **Build**: ✅ Local CI steps pass
- **Branch**: feature/0-1-project-init
- **Notes**: (any additional context)

---

### Task 0.1 Complete - Squash Merge

- [ ] All subtasks complete (0.1.1 - 0.1.3)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo build` succeeds
- [ ] Squash merge to main: `git checkout main && git merge --squash feature/0-1-project-init`
- [ ] Commit: `git commit -m "chore: project initialization with CI/CD"`
- [ ] Push to remote: `git push origin main`
- [ ] Delete branch: `git branch -d feature/0-1-project-init`

---

## Task 0.2: Module Architecture

**Git**: Create branch `feature/0-2-module-architecture` when starting first subtask.

### Subtask 0.2.1: Create Project Module Structure (Single Session)

**Prerequisites**:
- [x] 0.1.3: Set Up CI/CD with GitHub Actions

**Deliverables**:
- [ ] Create module directory structure
- [ ] Create module files with documentation
- [ ] Wire up modules in lib.rs
- [ ] Verify module structure compiles

**Files to Create**:

**`src/lib.rs`** (replace existing - complete file):
```rust
//! Nellie Production Library
//!
//! Production-grade semantic code memory system for enterprise engineering teams.
//!
//! # Architecture
//!
//! Nellie is organized into the following modules:
//!
//! - [`config`]: Configuration management (CLI args, environment, files)
//! - [`error`]: Error types and Result aliases
//! - [`storage`]: SQLite database with sqlite-vec for vector search
//! - [`embeddings`]: ONNX-based embedding generation
//! - [`watcher`]: File system watching and indexing
//! - [`server`]: MCP and REST API servers
//!
//! # Example
//!
//! ```rust,ignore
//! use nellie::config::Config;
//! use nellie::server::Server;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load()?;
//!     let server = Server::new(config).await?;
//!     server.run().await
//! }
//! ```

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod embeddings;
pub mod error;
pub mod server;
pub mod storage;
pub mod watcher;

pub use config::Config;
pub use error::{Error, Result};
```

**`src/config/mod.rs`** (complete file):
```rust
//! Configuration management for Nellie.
//!
//! Supports configuration from:
//! - Command-line arguments (highest priority)
//! - Environment variables
//! - Configuration file (lowest priority)

mod settings;

pub use settings::Config;
```

**`src/config/settings.rs`** (complete file):
```rust
//! Configuration settings and defaults.

use std::path::PathBuf;

/// Main configuration for Nellie server.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory for SQLite database and other data.
    pub data_dir: PathBuf,

    /// Host address to bind to.
    pub host: String,

    /// Port to listen on.
    pub port: u16,

    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,

    /// Directories to watch for code changes.
    pub watch_dirs: Vec<PathBuf>,

    /// Maximum number of embedding worker threads.
    pub embedding_threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            host: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            watch_dirs: Vec::new(),
            embedding_threads: num_cpus::get().min(4),
        }
    }
}

impl Config {
    /// Create a new configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment and defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid.
    pub fn load() -> crate::Result<Self> {
        // TODO: Implement proper config loading in Phase 0.2.3
        Ok(Self::default())
    }
}

// Add num_cpus to Cargo.toml as a dependency
fn num_cpus_get() -> usize {
    // Placeholder - will use actual num_cpus crate
    4
}

mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}
```

**`src/error/mod.rs`** (complete file):
```rust
//! Error types and Result aliases for Nellie.
//!
//! This module defines the error hierarchy used throughout the crate.
//! All public functions return `Result<T, Error>` or `Result<T>`.

use thiserror::Error;

/// Result type alias using Nellie's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for Nellie operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Database/storage error.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    /// Embedding generation error.
    #[error("embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    /// File watching error.
    #[error("watcher error: {0}")]
    Watcher(#[from] WatcherError),

    /// Server/API error.
    #[error("server error: {0}")]
    Server(#[from] ServerError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Storage-specific errors.
#[derive(Error, Debug)]
pub enum StorageError {
    /// SQLite database error.
    #[error("database error: {0}")]
    Database(String),

    /// Record not found.
    #[error("not found: {entity} with id '{id}'")]
    NotFound { entity: &'static str, id: String },

    /// Schema migration error.
    #[error("migration error: {0}")]
    Migration(String),

    /// Vector operation error.
    #[error("vector error: {0}")]
    Vector(String),
}

/// Embedding-specific errors.
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// ONNX runtime error.
    #[error("ONNX runtime error: {0}")]
    Runtime(String),

    /// Model loading error.
    #[error("failed to load model: {0}")]
    ModelLoad(String),

    /// Tokenization error.
    #[error("tokenization error: {0}")]
    Tokenization(String),

    /// Worker pool error.
    #[error("worker pool error: {0}")]
    WorkerPool(String),
}

/// File watcher errors.
#[derive(Error, Debug)]
pub enum WatcherError {
    /// Failed to watch path.
    #[error("failed to watch path '{path}': {reason}")]
    WatchFailed { path: String, reason: String },

    /// File processing error.
    #[error("failed to process file '{path}': {reason}")]
    ProcessFailed { path: String, reason: String },

    /// Indexing error.
    #[error("indexing error: {0}")]
    Indexing(String),
}

/// Server/API errors.
#[derive(Error, Debug)]
pub enum ServerError {
    /// Failed to bind to address.
    #[error("failed to bind to {address}: {reason}")]
    BindFailed { address: String, reason: String },

    /// Request handling error.
    #[error("request error: {0}")]
    Request(String),

    /// MCP protocol error.
    #[error("MCP error: {0}")]
    Mcp(String),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl StorageError {
    /// Create a not-found error.
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }
}
```

**`src/storage/mod.rs`** (complete file):
```rust
//! SQLite storage with sqlite-vec for vector search.
//!
//! This module provides persistent storage for:
//! - Code chunks with embeddings
//! - Lessons learned
//! - Agent checkpoints
//! - File state for incremental indexing

// Submodules will be added in Phase 1
// mod schema;
// mod queries;
// mod chunks;
// mod lessons;
// mod checkpoints;

/// Placeholder for storage initialization.
pub fn init() {
    tracing::debug!("Storage module initialized");
}
```

**`src/embeddings/mod.rs`** (complete file):
```rust
//! ONNX-based embedding generation.
//!
//! This module provides:
//! - ONNX Runtime integration via the `ort` crate
//! - Dedicated thread pool for embedding generation
//! - Async API using channels for non-blocking operation

// Submodules will be added in Phase 1
// mod worker;
// mod model;

/// Placeholder for embeddings initialization.
pub fn init() {
    tracing::debug!("Embeddings module initialized");
}
```

**`src/watcher/mod.rs`** (complete file):
```rust
//! File system watching and indexing.
//!
//! This module provides:
//! - Directory watching using notify-rs
//! - Gitignore-aware file filtering
//! - Incremental indexing of changed files

// Submodules will be added in Phase 2
// mod indexer;
// mod filter;

/// Placeholder for watcher initialization.
pub fn init() {
    tracing::debug!("Watcher module initialized");
}
```

**`src/server/mod.rs`** (complete file):
```rust
//! MCP and REST API servers.
//!
//! This module provides:
//! - MCP server using rmcp
//! - REST API using axum
//! - Health and metrics endpoints

// Submodules will be added in Phase 4
// mod mcp;
// mod rest;
// mod transport;

/// Placeholder for server initialization.
pub fn init() {
    tracing::debug!("Server module initialized");
}
```

**Verification Commands**:
```bash
# Verify module structure compiles
cargo check 2>&1 | tail -10
# Expected: "Finished `dev` profile..."

# Verify lib exports work
cargo doc --no-deps 2>&1 | tail -5
# Expected: "Documenting nellie..." then "Finished"

# List created module files
find src -name "*.rs" | sort
# Expected:
# src/config/mod.rs
# src/config/settings.rs
# src/embeddings/mod.rs
# src/error/mod.rs
# src/lib.rs
# src/main.rs
# src/server/mod.rs
# src/storage/mod.rs
# src/watcher/mod.rs
```

**Success Criteria**:
- [ ] All module directories created (config, error, storage, embeddings, watcher, server)
- [ ] Each module has mod.rs with documentation
- [ ] `cargo check` passes
- [ ] `cargo doc --no-deps` generates documentation
- [ ] Commit made with message "feat: create module structure"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/lib.rs` (X lines)
  - `src/config/mod.rs` (X lines)
  - `src/config/settings.rs` (X lines)
  - `src/error/mod.rs` (X lines)
  - `src/storage/mod.rs` (X lines)
  - `src/embeddings/mod.rs` (X lines)
  - `src/watcher/mod.rs` (X lines)
  - `src/server/mod.rs` (X lines)
- **Files Modified**: None
- **Tests**: N/A (structure only)
- **Build**: ✅ cargo check passes
- **Branch**: feature/0-2-module-architecture
- **Notes**: (any additional context)

---

### Subtask 0.2.2: Define Error Types and Result Aliases (Single Session)

**Prerequisites**:
- [x] 0.2.1: Create Project Module Structure

**Deliverables**:
- [ ] Add comprehensive error tests
- [ ] Verify error conversions work correctly
- [ ] Add error display formatting tests

**Files to Create**:

**`src/error/tests.rs`** (complete file):
```rust
//! Tests for error types.

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_error_display() {
        let err = Error::config("invalid port");
        assert_eq!(err.to_string(), "configuration error: invalid port");
    }

    #[test]
    fn test_storage_error_not_found() {
        let err = StorageError::not_found("chunk", "123");
        assert_eq!(err.to_string(), "not found: chunk with id '123'");
    }

    #[test]
    fn test_storage_error_conversion() {
        let storage_err = StorageError::Database("connection failed".to_string());
        let err: Error = storage_err.into();
        assert!(matches!(err, Error::Storage(_)));
    }

    #[test]
    fn test_embedding_error_conversion() {
        let emb_err = EmbeddingError::ModelLoad("model.onnx not found".to_string());
        let err: Error = emb_err.into();
        assert!(matches!(err, Error::Embedding(_)));
    }

    #[test]
    fn test_watcher_error_conversion() {
        let watch_err = WatcherError::WatchFailed {
            path: "/tmp/test".to_string(),
            reason: "permission denied".to_string(),
        };
        let err: Error = watch_err.into();
        assert!(matches!(err, Error::Watcher(_)));
    }

    #[test]
    fn test_server_error_conversion() {
        let server_err = ServerError::BindFailed {
            address: "127.0.0.1:8080".to_string(),
            reason: "address in use".to_string(),
        };
        let err: Error = server_err.into();
        assert!(matches!(err, Error::Server(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(Error::config("test error"))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_debug_format() {
        let err = Error::Internal("something went wrong".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Internal"));
        assert!(debug_str.contains("something went wrong"));
    }
}
```

**Update `src/error/mod.rs`** (add at the end):
```rust
#[cfg(test)]
mod tests;
```

**Verification Commands**:
```bash
# Run error tests
cargo test error:: --verbose 2>&1 | tail -20
# Expected: "test result: ok. X passed; 0 failed"

# Verify all tests pass
cargo test 2>&1 | grep -E "(test result|running)"
# Expected: "test result: ok" with passed count
```

**Success Criteria**:
- [ ] All error type tests pass
- [ ] Error conversions (From impls) work correctly
- [ ] `cargo test error::` shows all tests passing
- [ ] Commit made with message "test(error): add comprehensive error tests"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/error/tests.rs` (X lines)
- **Files Modified**:
  - `src/error/mod.rs` (added test module)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/0-2-module-architecture
- **Notes**: (any additional context)

---

### Subtask 0.2.3: Create Configuration System (Single Session)

**Prerequisites**:
- [x] 0.2.2: Define Error Types and Result Aliases

**Deliverables**:
- [ ] Implement CLI argument parsing with clap
- [ ] Add environment variable support
- [ ] Create configuration file support (optional)
- [ ] Add configuration validation
- [ ] Write comprehensive tests

**Files to Modify/Create**:

**`src/main.rs`** (replace - complete file):
```rust
//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::Parser;
use nellie::{Config, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Nellie Production - Semantic code memory system
#[derive(Parser, Debug)]
#[command(name = "nellie")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Data directory for SQLite database
    #[arg(short, long, env = "NELLIE_DATA_DIR", default_value = "./data")]
    data_dir: std::path::PathBuf,

    /// Host address to bind to
    #[arg(long, env = "NELLIE_HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, env = "NELLIE_PORT", default_value = "8080")]
    port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "NELLIE_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Directories to watch for code changes (can be specified multiple times)
    #[arg(short, long, env = "NELLIE_WATCH_DIRS", value_delimiter = ',')]
    watch: Vec<std::path::PathBuf>,

    /// Number of embedding worker threads
    #[arg(long, env = "NELLIE_EMBEDDING_THREADS", default_value = "4")]
    embedding_threads: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Nellie Production v{} starting...",
        env!("CARGO_PKG_VERSION")
    );

    // Build config from CLI
    let config = Config {
        data_dir: cli.data_dir,
        host: cli.host,
        port: cli.port,
        log_level: cli.log_level,
        watch_dirs: cli.watch,
        embedding_threads: cli.embedding_threads,
    };

    tracing::debug!(?config, "Configuration loaded");

    // Validate config
    config.validate()?;

    tracing::info!(
        "Server will bind to {}:{}, data in {:?}",
        config.host,
        config.port,
        config.data_dir
    );

    // TODO: Start server in Phase 4
    tracing::info!("Server startup not yet implemented - exiting");

    Ok(())
}
```

**`src/config/settings.rs`** (replace - complete file):
```rust
//! Configuration settings and validation.

use crate::{Error, Result};
use std::path::PathBuf;

/// Main configuration for Nellie server.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory for SQLite database and other data.
    pub data_dir: PathBuf,

    /// Host address to bind to.
    pub host: String,

    /// Port to listen on.
    pub port: u16,

    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,

    /// Directories to watch for code changes.
    pub watch_dirs: Vec<PathBuf>,

    /// Maximum number of embedding worker threads.
    pub embedding_threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            host: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            watch_dirs: Vec::new(),
            embedding_threads: std::thread::available_parallelism()
                .map(|n| n.get().min(4))
                .unwrap_or(4),
        }
    }
}

impl Config {
    /// Create a new configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables and defaults.
    ///
    /// Note: This is a simplified loader. Full loading is done via clap in main.rs.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid.
    pub fn load() -> Result<Self> {
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values.
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid.
    pub fn validate(&self) -> Result<()> {
        // Validate port
        if self.port == 0 {
            return Err(Error::config("port cannot be 0"));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(Error::config(format!(
                "invalid log level '{}', must be one of: {}",
                self.log_level,
                valid_levels.join(", ")
            )));
        }

        // Validate embedding threads
        if self.embedding_threads == 0 {
            return Err(Error::config("embedding_threads cannot be 0"));
        }

        if self.embedding_threads > 32 {
            return Err(Error::config(
                "embedding_threads cannot exceed 32 (hardware limit)",
            ));
        }

        // Validate host is not empty
        if self.host.is_empty() {
            return Err(Error::config("host cannot be empty"));
        }

        Ok(())
    }

    /// Get the path to the SQLite database file.
    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("nellie.db")
    }

    /// Get the server address as a string.
    #[must_use]
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_port() {
        let config = Config {
            port: 0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let config = Config {
            log_level: "invalid".to_string(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("log level"));
    }

    #[test]
    fn test_validate_invalid_embedding_threads_zero() {
        let config = Config {
            embedding_threads: 0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("embedding_threads"));
    }

    #[test]
    fn test_validate_invalid_embedding_threads_too_high() {
        let config = Config {
            embedding_threads: 100,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("32"));
    }

    #[test]
    fn test_validate_empty_host() {
        let config = Config {
            host: String::new(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("host"));
    }

    #[test]
    fn test_database_path() {
        let config = Config {
            data_dir: PathBuf::from("/var/lib/nellie"),
            ..Default::default()
        };
        assert_eq!(
            config.database_path(),
            PathBuf::from("/var/lib/nellie/nellie.db")
        );
    }

    #[test]
    fn test_server_addr() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 9090,
            ..Default::default()
        };
        assert_eq!(config.server_addr(), "0.0.0.0:9090");
    }

    #[test]
    fn test_all_log_levels_valid() {
        for level in ["trace", "debug", "info", "warn", "error"] {
            let config = Config {
                log_level: level.to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_ok(), "Level '{level}' should be valid");
        }
    }

    #[test]
    fn test_log_level_case_insensitive() {
        for level in ["TRACE", "Debug", "INFO", "Warn", "ERROR"] {
            let config = Config {
                log_level: level.to_string(),
                ..Default::default()
            };
            assert!(
                config.validate().is_ok(),
                "Level '{level}' should be valid (case insensitive)"
            );
        }
    }
}
```

**Verification Commands**:
```bash
# Verify build
cargo build 2>&1 | tail -5
# Expected: "Finished `dev` profile..."

# Test CLI help
cargo run -- --help 2>&1 | head -20
# Expected: Shows CLI options with descriptions

# Test CLI with arguments
cargo run -- --port 9090 --host 0.0.0.0 --log-level debug 2>&1 | head -10
# Expected: "Server will bind to 0.0.0.0:9090..."

# Test environment variable support
NELLIE_PORT=9000 cargo run 2>&1 | head -10
# Expected: "Server will bind to 127.0.0.1:9000..."

# Run config tests
cargo test config:: --verbose 2>&1 | tail -20
# Expected: "test result: ok. X passed; 0 failed"

# Run all tests
cargo test 2>&1 | grep -E "(test result|running)"
# Expected: All tests passing
```

**Success Criteria**:
- [ ] `cargo run -- --help` shows all CLI options
- [ ] `cargo run -- --port 9090` works with custom port
- [ ] Environment variables override defaults
- [ ] Invalid config values are rejected with clear messages
- [ ] All config tests pass
- [ ] Commit made with message "feat(config): implement CLI and configuration system"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**: None
- **Files Modified**:
  - `src/main.rs` (X lines)
  - `src/config/settings.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/0-2-module-architecture
- **Notes**: (any additional context)

---

### Task 0.2 Complete - Squash Merge

- [ ] All subtasks complete (0.2.1 - 0.2.3)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all tests)
- [ ] `cargo build --release` succeeds
- [ ] Squash merge to main: `git checkout main && git merge --squash feature/0-2-module-architecture`
- [ ] Commit: `git commit -m "feat: module architecture with error types and config system"`
- [ ] Push to remote: `git push origin main`
- [ ] Delete branch: `git branch -d feature/0-2-module-architecture`

---

## Phase 0 Complete

**Phase 0 Checklist**:
- [ ] Task 0.1 merged to main (project init, tools, CI/CD)
- [ ] Task 0.2 merged to main (module structure, errors, config)
- [ ] All tests pass
- [ ] All lints clean
- [ ] README documents project structure
- [ ] CI/CD workflows functional

**Ready for Phase 1**: Core Storage & Embeddings

---

*Phase 0 Plan - Nellie Production*
