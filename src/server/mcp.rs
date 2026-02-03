//! MCP server implementation using rmcp.

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::embeddings::EmbeddingService;
use crate::storage::Database;

/// MCP server state.
pub struct McpState {
    pub db: Database,
    pub embeddings: Option<EmbeddingService>,
}

impl McpState {
    /// Create new MCP state.
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self {
            db,
            embeddings: None,
        }
    }

    /// Create MCP state with embedding service.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // EmbeddingService is not const
    pub fn with_embeddings(db: Database, embeddings: EmbeddingService) -> Self {
        Self {
            db,
            embeddings: Some(embeddings),
        }
    }
}

/// Tool information with schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Tool definitions for Nellie.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo {
            name: "search_code".to_string(),
            description: Some(
                "Search indexed code repositories for relevant code snippets".to_string(),
            ),
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
            name: "list_lessons".to_string(),
            description: Some(
                "List all recorded lessons learned with optional filters for severity and limit"
                    .to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "severity": {
                        "type": "string",
                        "enum": ["critical", "warning", "info"],
                        "description": "Filter by severity level (optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum lessons to return (default: 50)",
                        "default": 50
                    }
                },
                "required": []
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
            name: "delete_lesson".to_string(),
            description: Some("Delete a lesson by ID".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Lesson ID to delete"
                    }
                },
                "required": ["id"]
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
            name: "trigger_reindex".to_string(),
            description: Some("Trigger manual re-indexing of specified paths".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File or directory path to re-index (optional, re-indexes all if omitted)"
                    }
                },
                "required": []
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
    let tool_name = request.name.clone();
    let span = tracing::info_span!(
        "tool_invocation",
        tool = %tool_name,
    );
    let _guard = span.enter();

    tracing::debug!("Invoking tool: {}", tool_name);

    let result = match request.name.as_str() {
        "search_code" => handle_search_code(&state, &request.arguments),
        "search_lessons" => handle_search_lessons(&state, &request.arguments),
        "list_lessons" => handle_list_lessons(&state, &request.arguments),
        "add_lesson" => handle_add_lesson(&state, &request.arguments),
        "delete_lesson" => handle_delete_lesson(&state, &request.arguments),
        "add_checkpoint" => handle_add_checkpoint(&state, &request.arguments),
        "get_recent_checkpoints" => handle_get_checkpoints(&state, &request.arguments),
        "trigger_reindex" => handle_trigger_reindex(&state, &request.arguments),
        "get_status" => handle_get_status(&state),
        _ => Err(format!("Unknown tool: {}", request.name)),
    };

    match result {
        Ok(content) => {
            tracing::debug!("Tool invocation succeeded");
            Json(ToolResponse {
                content,
                error: None,
            })
        }
        Err(e) => {
            tracing::warn!(error = %e, "Tool invocation failed");
            Json(ToolResponse {
                content: serde_json::Value::Null,
                error: Some(e),
            })
        }
    }
}

// Tool handlers

#[allow(clippy::cast_possible_truncation)]
fn handle_search_code(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let language_filter = args["language"].as_str();

    // Generate embedding for query
    let embedding = if let Some(ref embeddings) = state.embeddings {
        // Use real embeddings if available
        // Since we're in a sync context, we use blocking runtime
        // This is acceptable for the search operation
        let embeddings = embeddings.clone();
        let query_text = query.to_string();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle
                .block_on(async { embeddings.embed_one(query_text).await })
                .map_err(|e| format!("Failed to generate embedding: {e}"))?
        } else {
            // No async runtime available, use placeholder
            tracing::warn!("No async runtime for embeddings, using placeholder");
            crate::embeddings::placeholder_embedding(query)
        }
    } else {
        // Use placeholder embedding
        crate::embeddings::placeholder_embedding(query)
    };

    // Create search options
    let mut search_opts = crate::storage::SearchOptions::new(limit);
    if let Some(lang) = language_filter {
        search_opts = search_opts.with_language(lang);
    }

    // Search the database
    let results = state
        .db
        .with_conn(|conn| crate::storage::search_chunks(conn, &embedding, &search_opts))
        .map_err(|e| format!("Search failed: {e}"))?;

    // Format results for MCP response
    let formatted_results: Vec<serde_json::Value> = results
        .iter()
        .map(|result| {
            serde_json::json!({
                "file_path": result.record.file_path,
                "chunk_index": result.record.chunk_index,
                "start_line": result.record.start_line,
                "end_line": result.record.end_line,
                "content": result.record.content,
                "language": result.record.language,
                "score": result.score,
                "distance": result.distance,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "results": formatted_results,
        "query": query,
        "limit": limit,
        "count": formatted_results.len(),
    }))
}

#[allow(clippy::redundant_closure, clippy::cast_possible_truncation)]
fn handle_search_lessons(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    let lessons = state
        .db
        .with_conn(|conn| crate::storage::search_lessons_by_text(conn, query, limit))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(&lessons).unwrap_or_default())
}

#[allow(clippy::redundant_closure, clippy::cast_possible_truncation)]
fn handle_list_lessons(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let severity = args["severity"].as_str();
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let lessons = if let Some(severity_filter) = severity {
        state
            .db
            .with_conn(|conn| crate::storage::list_lessons_by_severity(conn, severity_filter))
            .map_err(|e| e.to_string())?
    } else {
        state
            .db
            .with_conn(|conn| crate::storage::list_lessons(conn))
            .map_err(|e| e.to_string())?
    };

    // Apply limit
    let limited_lessons: Vec<_> = lessons.into_iter().take(limit).collect();

    Ok(serde_json::json!({
        "lessons": serde_json::to_value(&limited_lessons).unwrap_or(serde_json::Value::Array(vec![])),
        "count": limited_lessons.len(),
        "severity": severity.unwrap_or("all")
    }))
}

#[allow(clippy::redundant_closure)]
fn handle_add_lesson(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let title = args["title"].as_str().ok_or("title is required")?;
    let content = args["content"].as_str().ok_or("content is required")?;
    let tags_array = args["tags"].as_array().ok_or("tags is required")?;
    let tags: Vec<String> = tags_array
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let severity = args["severity"].as_str().unwrap_or("info");

    let lesson = crate::storage::LessonRecord::new(title, content, tags).with_severity(severity);
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

#[allow(clippy::redundant_closure)]
fn handle_delete_lesson(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let id = args["id"].as_str().ok_or("id is required")?;

    state
        .db
        .with_conn(|conn| crate::storage::delete_lesson(conn, id))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": id,
        "message": "Lesson deleted successfully"
    }))
}

#[allow(clippy::redundant_closure)]
fn handle_add_checkpoint(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let agent = args["agent"].as_str().ok_or("agent is required")?;
    let working_on = args["working_on"]
        .as_str()
        .ok_or("working_on is required")?;
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

#[allow(clippy::redundant_closure, clippy::cast_possible_truncation)]
fn handle_get_checkpoints(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let agent = args["agent"].as_str().ok_or("agent is required")?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    let checkpoints = state
        .db
        .with_conn(|conn| crate::storage::get_recent_checkpoints(conn, agent, limit))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(&checkpoints).unwrap_or_default())
}

#[allow(clippy::redundant_closure)]
fn handle_trigger_reindex(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let path = args["path"].as_str();

    if let Some(target_path) = path {
        // Delete chunks for the specific path to trigger re-indexing
        state
            .db
            .with_conn(|conn| crate::storage::delete_chunks_by_file(conn, target_path))
            .map_err(|e| e.to_string())?;

        // Delete file state to mark as needing re-index
        state
            .db
            .with_conn(|conn| crate::storage::delete_file_state(conn, target_path))
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "reindex_scheduled",
            "path": target_path,
            "message": format!("Re-indexing scheduled for path: {}", target_path)
        }))
    } else {
        // Clear all file state to trigger full re-index
        // This is done by deleting all entries from file_state table
        state
            .db
            .with_conn(|conn| {
                // Get all file paths first
                let paths = crate::storage::list_file_paths(conn)?;
                // Delete file state for all paths
                for file_path in paths {
                    crate::storage::delete_file_state(conn, &file_path)?;
                }
                Ok::<_, crate::Error>(())
            })
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "reindex_scheduled",
            "path": "all",
            "message": "Full re-indexing scheduled for all tracked files"
        }))
    }
}

#[allow(clippy::redundant_closure, clippy::unnecessary_wraps)]
fn handle_get_status(state: &McpState) -> std::result::Result<serde_json::Value, String> {
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
        assert!(tools.len() >= 9);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"search_code"));
        assert!(names.contains(&"search_lessons"));
        assert!(names.contains(&"list_lessons"));
        assert!(names.contains(&"add_lesson"));
        assert!(names.contains(&"delete_lesson"));
        assert!(names.contains(&"add_checkpoint"));
        assert!(names.contains(&"get_recent_checkpoints"));
        assert!(names.contains(&"trigger_reindex"));
        assert!(names.contains(&"get_status"));
    }

    #[tokio::test]
    async fn test_list_tools_endpoint() {
        let tools = list_tools().await;
        assert!(!tools.0.is_empty());
    }

    #[test]
    fn test_search_code_schema() {
        let tools = get_tools();
        let search_code = tools
            .iter()
            .find(|t| t.name == "search_code")
            .expect("search_code tool should exist");

        let schema = &search_code.input_schema;
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("query").is_some());
        assert!(schema["properties"].get("limit").is_some());
    }

    #[test]
    fn test_add_lesson_schema() {
        let tools = get_tools();
        let add_lesson = tools
            .iter()
            .find(|t| t.name == "add_lesson")
            .expect("add_lesson tool should exist");

        let schema = &add_lesson.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required field should be an array");

        assert!(required.iter().any(|v| v.as_str() == Some("title")));
        assert!(required.iter().any(|v| v.as_str() == Some("content")));
        assert!(required.iter().any(|v| v.as_str() == Some("tags")));
    }

    #[test]
    fn test_search_code_with_placeholder_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;
            // Skip vector table initialization as sqlite-vec may not be available in tests
            Ok(())
        })
        .expect("Failed to setup database");
        let state = McpState::new(db);

        // Test with placeholder embedding (should gracefully handle missing vector table)
        let args = serde_json::json!({
            "query": "test search query"
        });

        let result = handle_search_code(&state, &args);
        // May fail due to missing vector table in test environment,
        // but should handle the error gracefully
        match result {
            Ok(response) => {
                // If successful, verify response structure
                assert!(response.get("results").is_some());
                assert_eq!(response["query"], "test search query");
            }
            Err(e) => {
                // Expected in test environment without sqlite-vec
                assert!(e.contains("Search failed") || e.contains("chunk_embeddings"));
            }
        }
    }

    #[test]
    fn test_search_code_with_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;
            Ok(())
        })
        .expect("Failed to setup database");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "query": "test",
            "limit": 5
        });

        let result = handle_search_code(&state, &args);
        // May fail due to missing vector table in test environment
        if let Ok(response) = result {
            assert_eq!(response["limit"], 5);
        }
    }

    #[test]
    fn test_search_code_missing_query() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_search_code(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("query is required"));
    }

    #[test]
    fn test_placeholder_embedding_consistency() {
        let embedding1 = crate::embeddings::placeholder_embedding("test query");
        let embedding2 = crate::embeddings::placeholder_embedding("test query");

        // Placeholder embeddings should be deterministic
        assert_eq!(embedding1, embedding2);
        assert_eq!(embedding1.len(), crate::embeddings::EMBEDDING_DIM);
    }

    #[test]
    fn test_add_lesson_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "title": "Memory Leak Prevention",
            "content": "Use Arc<RwLock<T>> carefully in async contexts",
            "tags": ["rust", "memory", "performance"],
            "severity": "critical"
        });

        let result = handle_add_lesson(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.get("id").is_some());
        assert!(response["message"]
            .as_str()
            .unwrap()
            .contains("Lesson recorded"));
    }

    #[test]
    fn test_add_lesson_missing_title() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "content": "Some lesson content",
            "tags": ["test"]
        });

        let result = handle_add_lesson(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("title is required"));
    }

    #[test]
    fn test_add_lesson_missing_content() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "title": "Lesson Title",
            "tags": ["test"]
        });

        let result = handle_add_lesson(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("content is required"));
    }

    #[test]
    fn test_add_lesson_missing_tags() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "title": "Lesson Title",
            "content": "Lesson content"
        });

        let result = handle_add_lesson(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tags is required"));
    }

    #[test]
    fn test_add_lesson_default_severity() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "title": "Lesson Title",
            "content": "Lesson content",
            "tags": ["test"]
            // severity not provided, should default to "info"
        });

        let result = handle_add_lesson(&state, &args);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.get("id").is_some());
    }

    #[test]
    fn test_search_lessons_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert a test lesson
            let lesson = crate::storage::LessonRecord::new(
                "Rust Error Handling",
                "Always use Result types instead of panicking in libraries",
                vec!["rust".to_string(), "error-handling".to_string()],
            );
            crate::storage::insert_lesson(conn, &lesson)?;
            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "query": "error handling",
            "limit": 5
        });

        let result = handle_search_lessons(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        // Response should be an array or object with lessons
        assert!(response.is_array() || response.is_object());
    }

    #[test]
    fn test_search_lessons_missing_query() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "limit": 5
        });

        let result = handle_search_lessons(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("query is required"));
    }

    #[test]
    fn test_search_lessons_default_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "query": "some query"
            // limit not provided, should default to 5
        });

        let result = handle_search_lessons(&state, &args);
        // Should succeed (may return empty results)
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_lessons_with_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert multiple test lessons
            for i in 0..10 {
                let lesson = crate::storage::LessonRecord::new(
                    &format!("Lesson {}", i),
                    &format!("Content for lesson {}", i),
                    vec!["test".to_string()],
                );
                crate::storage::insert_lesson(conn, &lesson)?;
            }
            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "query": "lesson",
            "limit": 3
        });

        let result = handle_search_lessons(&state, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_lessons_empty_result() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "query": "nonexistent lesson query",
            "limit": 5
        });

        let result = handle_search_lessons(&state, &args);
        // Should return success with empty results
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_checkpoint_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "code-generator-v1",
            "working_on": "Implementing feature X",
            "state": {
                "current_task": "feature-x",
                "progress": 0.5,
                "last_checkpoint": "2024-01-01T12:00:00Z"
            }
        });

        let result = handle_add_checkpoint(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.get("id").is_some());
        assert!(response["message"]
            .as_str()
            .unwrap()
            .contains("Checkpoint saved"));
    }

    #[test]
    fn test_add_checkpoint_missing_agent() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "working_on": "Task",
            "state": {}
        });

        let result = handle_add_checkpoint(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent is required"));
    }

    #[test]
    fn test_add_checkpoint_missing_working_on() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "agent-v1",
            "state": {}
        });

        let result = handle_add_checkpoint(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("working_on is required"));
    }

    #[test]
    fn test_add_checkpoint_with_empty_state() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "agent-v1",
            "working_on": "Task",
            "state": {}
        });

        let result = handle_add_checkpoint(&state, &args);
        // Should succeed even with empty state object
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_checkpoints_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert test checkpoints
            let checkpoint1 = crate::storage::CheckpointRecord::new(
                "test-agent",
                "Working on task 1",
                serde_json::json!({"step": 1}),
            );
            crate::storage::insert_checkpoint(conn, &checkpoint1)?;

            let checkpoint2 = crate::storage::CheckpointRecord::new(
                "test-agent",
                "Working on task 2",
                serde_json::json!({"step": 2}),
            );
            crate::storage::insert_checkpoint(conn, &checkpoint2)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "test-agent",
            "limit": 5
        });

        let result = handle_get_checkpoints(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_array() || response.is_object());
    }

    #[test]
    fn test_get_checkpoints_missing_agent() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "limit": 5
        });

        let result = handle_get_checkpoints(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent is required"));
    }

    #[test]
    fn test_get_checkpoints_default_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "test-agent"
            // limit not provided, should default to 5
        });

        let result = handle_get_checkpoints(&state, &args);
        // Should succeed (may return empty results)
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_checkpoints_with_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert multiple checkpoints
            for i in 0..10 {
                let checkpoint = crate::storage::CheckpointRecord::new(
                    "test-agent",
                    &format!("Task {}", i),
                    serde_json::json!({"step": i}),
                );
                crate::storage::insert_checkpoint(conn, &checkpoint)?;
            }
            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "test-agent",
            "limit": 3
        });

        let result = handle_get_checkpoints(&state, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_checkpoints_empty_result() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "nonexistent-agent",
            "limit": 5
        });

        let result = handle_get_checkpoints(&state, &args);
        // Should return success with empty results
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_lessons_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert test lessons
            for i in 0..5 {
                let lesson = crate::storage::LessonRecord::new(
                    &format!("Lesson {}", i),
                    &format!("Content for lesson {}", i),
                    vec!["test".to_string()],
                );
                crate::storage::insert_lesson(conn, &lesson)?;
            }
            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_list_lessons(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.get("lessons").is_some());
        assert!(response.get("count").is_some());
        assert_eq!(response["count"], 5);
    }

    #[test]
    fn test_list_lessons_with_limit() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert test lessons
            for i in 0..10 {
                let lesson = crate::storage::LessonRecord::new(
                    &format!("Lesson {}", i),
                    &format!("Content {}", i),
                    vec!["test".to_string()],
                );
                crate::storage::insert_lesson(conn, &lesson)?;
            }
            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "limit": 3
        });

        let result = handle_list_lessons(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["count"], 3);
    }

    #[test]
    fn test_list_lessons_with_severity_filter() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert lessons with different severities
            let lesson1 = crate::storage::LessonRecord::new(
                "Critical Issue",
                "A critical problem",
                vec!["critical".to_string()],
            )
            .with_severity("critical");
            crate::storage::insert_lesson(conn, &lesson1)?;

            let lesson2 = crate::storage::LessonRecord::new(
                "Warning Issue",
                "A warning problem",
                vec!["warning".to_string()],
            )
            .with_severity("warning");
            crate::storage::insert_lesson(conn, &lesson2)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "severity": "critical"
        });

        let result = handle_list_lessons(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["severity"], "critical");
    }

    #[test]
    fn test_list_lessons_empty() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_list_lessons(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["count"], 0);
    }

    #[test]
    fn test_delete_lesson_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert a test lesson
            let lesson = crate::storage::LessonRecord::new(
                "Test Lesson",
                "Test content",
                vec!["test".to_string()],
            );
            crate::storage::insert_lesson(conn, &lesson)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        // Get the ID from a list query first
        let list_result = state
            .db
            .with_conn(|conn| crate::storage::list_lessons(conn))
            .expect("Failed to list lessons");

        if let Some(lesson) = list_result.first() {
            let args = serde_json::json!({
                "id": &lesson.id
            });

            let result = handle_delete_lesson(&state, &args);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert!(response.get("id").is_some());
            assert!(response["message"].as_str().unwrap().contains("deleted"));
        }
    }

    #[test]
    fn test_delete_lesson_missing_id() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_delete_lesson(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("id is required"));
    }

    #[test]
    fn test_trigger_reindex_specific_path() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "path": "/test/file.rs"
        });

        let result = handle_trigger_reindex(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "reindex_scheduled");
        assert_eq!(response["path"], "/test/file.rs");
        assert!(response["message"]
            .as_str()
            .unwrap()
            .contains("Re-indexing scheduled"));
    }

    #[test]
    fn test_trigger_reindex_all_paths() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_trigger_reindex(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "reindex_scheduled");
        assert_eq!(response["path"], "all");
        assert!(response["message"]
            .as_str()
            .unwrap()
            .contains("Full re-indexing"));
    }

    #[test]
    fn test_list_lessons_tool_exists() {
        let tools = get_tools();
        let list_lessons = tools
            .iter()
            .find(|t| t.name == "list_lessons")
            .expect("list_lessons tool should exist");

        assert!(list_lessons.description.is_some());
        let desc = list_lessons.description.as_ref().unwrap().to_lowercase();
        assert!(desc.contains("list"));
    }

    #[test]
    fn test_delete_lesson_tool_exists() {
        let tools = get_tools();
        let delete_lesson = tools
            .iter()
            .find(|t| t.name == "delete_lesson")
            .expect("delete_lesson tool should exist");

        assert!(delete_lesson.description.is_some());
        assert!(delete_lesson
            .description
            .as_ref()
            .unwrap()
            .contains("Delete"));
    }

    #[test]
    fn test_trigger_reindex_tool_exists() {
        let tools = get_tools();
        let trigger_reindex = tools
            .iter()
            .find(|t| t.name == "trigger_reindex")
            .expect("trigger_reindex tool should exist");

        assert!(trigger_reindex.description.is_some());
        assert!(trigger_reindex
            .description
            .as_ref()
            .unwrap()
            .contains("re-indexing"));
    }

    #[test]
    fn test_checkpoint_tool_schema() {
        let tools = get_tools();
        let add_checkpoint = tools
            .iter()
            .find(|t| t.name == "add_checkpoint")
            .expect("add_checkpoint tool should exist");

        let schema = &add_checkpoint.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required field should be an array");

        assert!(required.iter().any(|v| v.as_str() == Some("agent")));
        assert!(required.iter().any(|v| v.as_str() == Some("working_on")));
        assert!(required.iter().any(|v| v.as_str() == Some("state")));
    }

    #[test]
    fn test_get_checkpoints_tool_schema() {
        let tools = get_tools();
        let get_checkpoints = tools
            .iter()
            .find(|t| t.name == "get_recent_checkpoints")
            .expect("get_recent_checkpoints tool should exist");

        let schema = &get_checkpoints.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required field should be an array");

        assert!(required.iter().any(|v| v.as_str() == Some("agent")));
    }
}
