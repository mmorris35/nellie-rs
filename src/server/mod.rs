//! MCP and REST API servers.
//!
//! This module provides:
//! - MCP server using rmcp
//! - REST API using axum
//! - Health and metrics endpoints
//! - Graceful shutdown coordination
//! - Structured logging and tracing observability

mod app;
mod mcp;
mod metrics;
pub mod observability;
mod rest;

pub use app::{App, ServerConfig};
pub use mcp::{create_mcp_router, get_tools, McpState, ToolRequest, ToolResponse};
pub use metrics::{init_metrics, CHUNKS_TOTAL, EMBEDDING_QUEUE_DEPTH, FILES_TOTAL, LESSONS_TOTAL};
pub use observability::init_tracing;
pub use rest::{create_rest_router, HealthResponse};

/// Initialize server module.
pub fn init() {
    ::tracing::debug!("Server module initialized");
}
