# CLAUDE.md - Project Rules for Nellie Production

> This document defines HOW Claude Code should work on Nellie Production (Rust rewrite).
> Read at the start of every session to maintain consistency.

## Core Operating Principles

### 1. Single Session Execution
- ✅ Complete the ENTIRE subtask in this session
- ✅ End every session with a git commit
- ❌ If blocked, document why and mark as BLOCKED

### 2. Read Before Acting
**Every session must begin with:**
1. Read DEVELOPMENT_PLAN.md completely
2. Locate the specific subtask ID from the prompt
3. Verify prerequisites are marked `[x]` complete
4. Read completion notes from prerequisites for context

### 3. File Management

**Project Structure:**
```
nellie-rs/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── server/              # MCP server implementation
│   │   ├── mod.rs
│   │   ├── tools.rs         # MCP tool handlers
│   │   └── transport.rs     # HTTP+SSE transport
│   ├── storage/             # SQLite + sqlite-vec
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── queries.rs
│   ├── embeddings/          # ONNX embedding worker
│   │   ├── mod.rs
│   │   └── worker.rs
│   ├── watcher/             # File watching daemon
│   │   ├── mod.rs
│   │   └── indexer.rs
│   └── config.rs            # Configuration
├── tests/
│   ├── integration/         # Integration tests
│   └── common/              # Test utilities
├── benches/                 # Benchmarks
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CLAUDE.md                # This file
├── PROJECT_BRIEF.md         # Requirements
└── DEVELOPMENT_PLAN.md      # Development roadmap
```

**Creating Files:**
- Use exact paths specified in subtask
- Add proper module documentation (`//!` for modules, `///` for items)
- Include full type annotations

**Modifying Files:**
- Only modify files listed in subtask
- Preserve existing functionality
- Update related tests

### 4. Testing Requirements

**Unit Tests:**
- Write tests for EVERY new function/struct
- Use `#[cfg(test)]` module at bottom of each file
- Integration tests in `tests/` directory
- Target coverage: 80%+

**Running Tests:**
```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name

# Integration tests only
cargo test --test '*'
```

**Before Every Commit:**
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo build --release` succeeds

### 5. Completion Protocol

**When a subtask is complete:**

1. **Update DEVELOPMENT_PLAN.md** with completion notes:
```markdown
**Completion Notes:**
- **Implementation**: Brief description of what was built
- **Files Created**:
  - `src/storage/mod.rs` (234 lines)
  - `tests/integration/storage_test.rs` (156 lines)
- **Files Modified**:
  - `src/lib.rs` (added storage module)
- **Tests**: 12 unit tests, 3 integration tests
- **Build**: ✅ Success (tests pass, clippy clean, fmt clean)
- **Branch**: feature/1-2-storage
- **Notes**: Any deviations, issues, or future work
```

2. **Check all checkboxes** in the subtask (change `[ ]` to `[x]`)

3. **Git commit** with semantic message:
```bash
git add .
git commit -m "feat(storage): Implement SQLite storage layer

- Add sqlite-vec for vector similarity
- Create schema with migrations
- Implement CRUD for chunks, lessons, checkpoints
- Add comprehensive tests"
```

4. **Report completion** with summary

### 6. Technology Stack

| Component | Library | Version |
|-----------|---------|---------|
| MCP Protocol | rmcp | 0.8+ |
| HTTP Server | axum | 0.8+ |
| Async Runtime | tokio | 1.0+ |
| Vector Storage | sqlite-vec | latest |
| Embeddings | ort | 2.0+ |
| File Watching | notify | 6.0+ |
| Serialization | serde | 1.0+ |
| CLI | clap | 4.0+ |
| Error Handling | thiserror, anyhow | latest |
| Logging | tracing | latest |

**Cargo Commands:**
```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run debug
cargo run --release      # Run release
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
cargo doc --open         # Documentation
```

### 7. Error Handling

**Rust Patterns:**
- Use `thiserror` for library errors
- Use `anyhow` for application errors
- Propagate errors with `?` operator
- Never panic in library code

**If you encounter a blocking error:**
1. Update DEVELOPMENT_PLAN.md with BLOCKED status
2. Do NOT mark subtask complete
3. Do NOT commit broken code
4. Report immediately

### 8. Code Quality Standards

**Rust Style:**
- Follow Rust API Guidelines
- All public items have documentation
- No `unwrap()` in production code (use `expect()` or proper error handling)
- No `unsafe` without explicit justification and safety comments
- Max line length: 100 characters

**Required Attributes:**
```rust
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
```

**Prohibited:**
- `println!()` in library code (use `tracing`)
- Blocking calls on async runtime
- `panic!()` for recoverable errors

### 9. Async Best Practices

**Tokio Patterns:**
- Use `#[tokio::main]` for entry point
- Spawn blocking work with `tokio::task::spawn_blocking`
- Use channels for cross-thread communication
- Always handle task join errors

**Embedding Worker:**
- Embeddings MUST run in dedicated thread pool
- Never block async runtime with embedding calls
- Use bounded channels for backpressure

### 10. Build Verification

**Before marking subtask complete:**

```bash
# Format
cargo fmt

# Lint
cargo clippy -- -D warnings

# Test
cargo test

# Build release
cargo build --release
```

**All must pass with no errors or warnings.**

## Checklist: Starting a New Session

- [ ] Read DEVELOPMENT_PLAN.md completely
- [ ] Locate subtask ID from prompt
- [ ] Verify prerequisites marked `[x]`
- [ ] Read prerequisite completion notes
- [ ] Understand success criteria
- [ ] Ready to code!

## Checklist: Ending a Session

- [ ] All subtask checkboxes checked
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean
- [ ] Completion notes written
- [ ] Git commit with semantic message
- [ ] User notified

---

**Version**: 1.0
**Last Updated**: 2026-02-02
**Project**: Nellie Production (Rust)
