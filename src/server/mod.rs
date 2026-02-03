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
