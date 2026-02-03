# Phase 4: MCP & REST API

**Goal**: Implement MCP server and REST API with health/metrics endpoints
**Duration**: 1 week
**Prerequisites**: Phase 3 complete

---

## Task 4.1: MCP Server

**Git**: Create branch `feature/4-1-mcp-server` when starting first subtask.

### Subtask 4.1.1: Set Up rmcp Server with Axum Transport (Single Session)

**Prerequisites**:
- [x] 3.2.1: Implement Checkpoint Storage

**Deliverables**:
- [ ] Create MCP server using rmcp crate
- [ ] Configure HTTP+SSE transport
- [ ] Set up tool registration framework
- [ ] Write basic server tests

**Files to Create**:

**`src/server/mcp.rs`** (complete file):
```rust
//! MCP server implementation using rmcp.

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use rmcp::{
    model::{Tool, ToolInfo},
    server::{Server as McpServer, ServerConfig},
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::storage::Database;
use crate::Result;

/// MCP server state.
pub struct McpState {
    pub db: Database,
    // Add embeddings service when ready
}

impl McpState {
    /// Create new MCP state.
    #[must_use]
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

/// Tool definitions for Nellie.
pub fn get_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "search_code".to_string(),
            description: Some("Search indexed code repositories for relevant code snippets".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query to search for relevant code"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)",
                        "default": 10
                    },
                    "language": {
                        "type": "string",
                        "description": "Filter by programming language"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolInfo {
            name: "search_lessons".to_string(),
            description: Some("Search previously recorded lessons learned".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query to search lessons"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum lessons to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        },
        ToolInfo {
            name: "add_lesson".to_string(),
            description: Some("Record a lesson learned during development".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Brief title for the lesson"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full description of the lesson learned"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags for categorization"
                    },
                    "severity": {
                        "type": "string",
                        "enum": ["critical", "warning", "info"],
                        "description": "Importance level (default: info)"
                    }
                },
                "required": ["title", "content", "tags"]
            }),
        },
        ToolInfo {
            name: "add_checkpoint".to_string(),
            description: Some("Store an agent checkpoint for context recovery".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent identifier"
                    },
                    "working_on": {
                        "type": "string",
                        "description": "Current task description"
                    },
                    "state": {
                        "type": "object",
                        "description": "State object to persist"
                    }
                },
                "required": ["agent", "working_on", "state"]
            }),
        },
        ToolInfo {
            name: "get_recent_checkpoints".to_string(),
            description: Some("Retrieve recent checkpoints for an agent".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent identifier"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum checkpoints to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["agent"]
            }),
        },
        ToolInfo {
            name: "get_status".to_string(),
            description: Some("Get Nellie server status and statistics".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Create MCP router.
pub fn create_mcp_router(state: Arc<McpState>) -> Router {
    Router::new()
        .route("/mcp/tools", get(list_tools))
        .route("/mcp/invoke", post(invoke_tool))
        .with_state(state)
}

/// List available tools.
async fn list_tools() -> Json<Vec<ToolInfo>> {
    Json(get_tools())
}

/// Tool invocation request.
#[derive(Debug, Deserialize)]
pub struct ToolRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool invocation response.
#[derive(Debug, Serialize)]
pub struct ToolResponse {
    pub content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Invoke a tool.
async fn invoke_tool(
    State(state): State<Arc<McpState>>,
    Json(request): Json<ToolRequest>,
) -> Json<ToolResponse> {
    let result = match request.name.as_str() {
        "search_code" => handle_search_code(&state, request.arguments).await,
        "search_lessons" => handle_search_lessons(&state, request.arguments).await,
        "add_lesson" => handle_add_lesson(&state, request.arguments).await,
        "add_checkpoint" => handle_add_checkpoint(&state, request.arguments).await,
        "get_recent_checkpoints" => handle_get_checkpoints(&state, request.arguments).await,
        "get_status" => handle_get_status(&state).await,
        _ => Err(format!("Unknown tool: {}", request.name)),
    };

    match result {
        Ok(content) => Json(ToolResponse {
            content,
            error: None,
        }),
        Err(e) => Json(ToolResponse {
            content: serde_json::Value::Null,
            error: Some(e),
        }),
    }
}

// Tool handlers

async fn handle_search_code(
    state: &McpState,
    args: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;

    // TODO: Implement actual search with embeddings
    // For now, return placeholder
    Ok(serde_json::json!({
        "results": [],
        "query": query,
        "limit": limit,
        "message": "Search not yet implemented - embeddings required"
    }))
}

async fn handle_search_lessons(
    state: &McpState,
    args: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    let lessons = state
        .db
        .with_conn(|conn| crate::storage::search_lessons_by_text(conn, query, limit))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(&lessons).unwrap_or_default())
}

async fn handle_add_lesson(
    state: &McpState,
    args: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let title = args["title"].as_str().ok_or("title is required")?;
    let content = args["content"].as_str().ok_or("content is required")?;
    let tags: Vec<String> = args["tags"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let severity = args["severity"].as_str().unwrap_or("info");

    let lesson = crate::storage::LessonRecord::new(title, content, tags)
        .with_severity(severity);
    let id = lesson.id.clone();

    state
        .db
        .with_conn(|conn| crate::storage::insert_lesson(conn, &lesson))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": id,
        "message": "Lesson recorded successfully"
    }))
}

async fn handle_add_checkpoint(
    state: &McpState,
    args: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let agent = args["agent"].as_str().ok_or("agent is required")?;
    let working_on = args["working_on"].as_str().ok_or("working_on is required")?;
    let checkpoint_state = args["state"].clone();

    let checkpoint = crate::storage::CheckpointRecord::new(agent, working_on, checkpoint_state);
    let id = checkpoint.id.clone();

    state
        .db
        .with_conn(|conn| crate::storage::insert_checkpoint(conn, &checkpoint))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": id,
        "message": "Checkpoint saved successfully"
    }))
}

async fn handle_get_checkpoints(
    state: &McpState,
    args: serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let agent = args["agent"].as_str().ok_or("agent is required")?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    let checkpoints = state
        .db
        .with_conn(|conn| crate::storage::get_recent_checkpoints(conn, agent, limit))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(&checkpoints).unwrap_or_default())
}

async fn handle_get_status(
    state: &McpState,
) -> std::result::Result<serde_json::Value, String> {
    let chunk_count = state
        .db
        .with_conn(|conn| crate::storage::count_chunks(conn))
        .unwrap_or(0);

    let lesson_count = state
        .db
        .with_conn(|conn| crate::storage::count_lessons(conn))
        .unwrap_or(0);

    let file_count = state
        .db
        .with_conn(|conn| crate::storage::count_tracked_files(conn))
        .unwrap_or(0);

    Ok(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "stats": {
            "chunks": chunk_count,
            "lessons": lesson_count,
            "files": file_count
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_defined() {
        let tools = get_tools();
        assert!(tools.len() >= 5);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"search_code"));
        assert!(names.contains(&"search_lessons"));
        assert!(names.contains(&"add_lesson"));
        assert!(names.contains(&"add_checkpoint"));
        assert!(names.contains(&"get_status"));
    }

    #[tokio::test]
    async fn test_list_tools_endpoint() {
        let tools = list_tools().await;
        assert!(!tools.0.is_empty());
    }
}
```

**Update `src/server/mod.rs`** (replace - complete file):
```rust
//! MCP and REST API servers.
//!
//! This module provides:
//! - MCP server using rmcp
//! - REST API using axum
//! - Health and metrics endpoints

mod mcp;

pub use mcp::{create_mcp_router, get_tools, McpState, ToolRequest, ToolResponse};

/// Initialize server module.
pub fn init() {
    tracing::debug!("Server module initialized");
}
```

**Verification Commands**:
```bash
cargo check 2>&1 | tail -10
# Expected: "Finished `dev` profile..."

cargo test server:: --verbose 2>&1 | tail -20
# Expected: "test result: ok. 2 passed; 0 failed"
```

**Success Criteria**:
- [ ] MCP tools defined
- [ ] Router compiles
- [ ] Tool handlers compile
- [ ] All server tests pass
- [ ] Commit made with message "feat(server): set up MCP server with tool handlers"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/server/mcp.rs` (X lines)
- **Files Modified**:
  - `src/server/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/4-1-mcp-server
- **Notes**: (any additional context)

---

### Task 4.1 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] All tests pass
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Task 4.2: REST API & Observability

**Git**: Create branch `feature/4-2-rest-api` when starting first subtask.

### Subtask 4.2.1: Create REST Health and Metrics Endpoints (Single Session)

**Prerequisites**:
- [x] 4.1.1: Set Up rmcp Server

**Deliverables**:
- [ ] Create health check endpoint
- [ ] Create Prometheus metrics endpoint
- [ ] Add basic metrics collection
- [ ] Write endpoint tests

**Files to Create**:

**`src/server/rest.rs`** (complete file):
```rust
//! REST API endpoints.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::Serialize;

use super::mcp::McpState;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
}

/// Create REST API router.
pub fn create_rest_router(state: Arc<McpState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .route("/api/v1/status", get(status))
        .with_state(state)
}

/// Health check endpoint.
async fn health_check(State(state): State<Arc<McpState>>) -> impl IntoResponse {
    let db_status = match state.db.health_check() {
        Ok(()) => "ok",
        Err(_) => "error",
    };

    let response = HealthResponse {
        status: if db_status == "ok" { "healthy" } else { "unhealthy" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status.to_string(),
    };

    if db_status == "ok" {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

/// Prometheus metrics endpoint.
async fn metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    if encoder.encode(&metric_families, &mut buffer).is_ok() {
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            buffer,
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            b"Failed to encode metrics".to_vec(),
        )
    }
}

/// Status endpoint with statistics.
async fn status(State(state): State<Arc<McpState>>) -> impl IntoResponse {
    let chunk_count = state
        .db
        .with_conn(|conn| crate::storage::count_chunks(conn))
        .unwrap_or(0);

    let lesson_count = state
        .db
        .with_conn(|conn| crate::storage::count_lessons(conn))
        .unwrap_or(0);

    let file_count = state
        .db
        .with_conn(|conn| crate::storage::count_tracked_files(conn))
        .unwrap_or(0);

    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "stats": {
            "indexed_chunks": chunk_count,
            "lessons": lesson_count,
            "tracked_files": file_count
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{migrate, Database};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn create_test_state() -> Arc<McpState> {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();
        Arc::new(McpState::new(db))
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = create_test_state();
        let app = create_rest_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics() {
        let state = create_test_state();
        let app = create_rest_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status() {
        let state = create_test_state();
        let app = create_rest_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

**`src/server/metrics.rs`** (complete file):
```rust
//! Prometheus metrics definitions.

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};

/// Total chunks indexed.
pub static CHUNKS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_chunks_total", "Total number of indexed code chunks").unwrap()
});

/// Total lessons stored.
pub static LESSONS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_lessons_total", "Total number of lessons stored").unwrap()
});

/// Total files tracked.
pub static FILES_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("nellie_files_total", "Total number of tracked files").unwrap()
});

/// Request latency histogram.
pub static REQUEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "nellie_request_duration_seconds",
        "Request latency in seconds",
        &["endpoint", "method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .unwrap()
});

/// Request counter.
pub static REQUEST_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nellie_requests_total",
        "Total number of requests",
        &["endpoint", "method", "status"]
    )
    .unwrap()
});

/// Embedding queue depth.
pub static EMBEDDING_QUEUE_DEPTH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "nellie_embedding_queue_depth",
        "Number of items waiting for embedding"
    )
    .unwrap()
});

/// Initialize all metrics (call once at startup).
pub fn init_metrics() {
    // Access lazy statics to register them
    let _ = &*CHUNKS_TOTAL;
    let _ = &*LESSONS_TOTAL;
    let _ = &*FILES_TOTAL;
    let _ = &*REQUEST_LATENCY;
    let _ = &*REQUEST_COUNT;
    let _ = &*EMBEDDING_QUEUE_DEPTH;

    tracing::debug!("Prometheus metrics initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_init() {
        init_metrics();

        CHUNKS_TOTAL.set(100);
        assert_eq!(CHUNKS_TOTAL.get(), 100);

        LESSONS_TOTAL.set(50);
        assert_eq!(LESSONS_TOTAL.get(), 50);
    }
}
```

**Update `src/server/mod.rs`** - add:
```rust
mod metrics;
mod rest;

pub use metrics::{init_metrics, CHUNKS_TOTAL, EMBEDDING_QUEUE_DEPTH, FILES_TOTAL, LESSONS_TOTAL};
pub use rest::{create_rest_router, HealthResponse};
```

**Verification Commands**:
```bash
cargo test server:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. X passed; 0 failed"
```

**Success Criteria**:
- [ ] Health endpoint returns status
- [ ] Metrics endpoint returns Prometheus format
- [ ] Status endpoint returns stats
- [ ] All REST tests pass
- [ ] Commit made with message "feat(server): add REST health and metrics endpoints"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/server/rest.rs` (X lines)
  - `src/server/metrics.rs` (X lines)
- **Files Modified**:
  - `src/server/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/4-2-rest-api
- **Notes**: (any additional context)

---

### Subtask 4.2.2: Implement Graceful Shutdown (Single Session)

**Prerequisites**:
- [x] 4.2.1: Create REST Health and Metrics Endpoints

**Deliverables**:
- [ ] Add signal handling for SIGTERM/SIGINT
- [ ] Implement graceful shutdown for server
- [ ] Add shutdown timeout
- [ ] Update main.rs with full server startup

**Files to Create**:

**`src/server/app.rs`** (complete file):
```rust
//! Main application server.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::mcp::{create_mcp_router, McpState};
use super::rest::create_rest_router;
use crate::storage::Database;
use crate::Result;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

/// Application server.
pub struct App {
    config: ServerConfig,
    state: Arc<McpState>,
}

impl App {
    /// Create a new application.
    #[must_use]
    pub fn new(config: ServerConfig, db: Database) -> Self {
        let state = Arc::new(McpState::new(db));
        Self { config, state }
    }

    /// Build the router with all endpoints.
    fn router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .merge(create_mcp_router(Arc::clone(&self.state)))
            .merge(create_rest_router(Arc::clone(&self.state)))
            .layer(TraceLayer::new_for_http())
            .layer(cors)
    }

    /// Run the server until shutdown signal.
    ///
    /// # Errors
    ///
    /// Returns an error if the server cannot start.
    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| crate::Error::config(format!("invalid address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| crate::error::ServerError::BindFailed {
                address: addr.to_string(),
                reason: e.to_string(),
            })?;

        tracing::info!(%addr, "Server listening");

        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| crate::error::ServerError::Request(e.to_string()))?;

        tracing::info!("Server shut down gracefully");
        Ok(())
    }
}

/// Wait for shutdown signal.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_app_creation() {
        let config = ServerConfig::default();
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();
        // Router exists - can't easily test more without running server
        assert!(true);
    }
}
```

**Update `src/server/mod.rs`** - add:
```rust
mod app;

pub use app::{App, ServerConfig};
```

**Update `src/main.rs`** (replace - complete file):
```rust
//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::Parser;
use nellie::server::{init_metrics, App, ServerConfig};
use nellie::storage::{init_storage, Database};
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

    /// Directories to watch for code changes
    #[arg(short, long, env = "NELLIE_WATCH_DIRS", value_delimiter = ',')]
    watch: Vec<std::path::PathBuf>,

    /// Number of embedding worker threads
    #[arg(long, env = "NELLIE_EMBEDDING_THREADS", default_value = "4")]
    embedding_threads: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
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

    // Build config
    let config = Config {
        data_dir: cli.data_dir.clone(),
        host: cli.host.clone(),
        port: cli.port,
        log_level: cli.log_level,
        watch_dirs: cli.watch,
        embedding_threads: cli.embedding_threads,
    };

    config.validate()?;

    // Initialize database
    let db = Database::open(config.database_path())?;
    init_storage(&db)?;

    // Initialize metrics
    init_metrics();

    // Create and run server
    let server_config = ServerConfig {
        host: config.host,
        port: config.port,
        ..Default::default()
    };

    let app = App::new(server_config, db);
    app.run().await
}
```

**Verification Commands**:
```bash
# Build and verify
cargo build 2>&1 | tail -5
# Expected: "Finished `dev` profile..."

# Run server briefly (Ctrl+C to stop)
timeout 3 cargo run -- --port 9999 2>&1 || true
# Expected: "Server listening" then timeout

# Run all tests
cargo test 2>&1 | grep "test result"
# Expected: "test result: ok"
```

**Success Criteria**:
- [ ] Server starts and listens
- [ ] Graceful shutdown on Ctrl+C
- [ ] All components integrated
- [ ] All tests pass
- [ ] Commit made with message "feat(server): implement graceful shutdown and full server startup"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/server/app.rs` (X lines)
- **Files Modified**:
  - `src/server/mod.rs` (X lines)
  - `src/main.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/4-2-rest-api
- **Notes**: (any additional context)

---

### Task 4.2 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Server starts and responds to health checks
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Phase 4 Complete

**Phase 4 Checklist**:
- [ ] Task 4.1 merged (MCP server + tools)
- [ ] Task 4.2 merged (REST API + metrics + graceful shutdown)
- [ ] All tests pass (80+ tests)
- [ ] Server functional with all endpoints

**Ready for Phase 5**: Packaging & Documentation

---

*Phase 4 Plan - Nellie Production*
