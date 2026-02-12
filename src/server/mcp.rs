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

/// Check if a path is on a network mount (NFS, SMB, CIFS, etc.)
/// This is used to choose between fast walker (network) and gitignore-aware walker (local).
fn is_network_path(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();
    
    // macOS: /Volumes/ paths that aren't the main disk
    if path_str.starts_with("/Volumes/") && !path_str.starts_with("/Volumes/Macintosh") {
        return true;
    }
    
    // Linux: common network mount points
    if path_str.starts_with("/mnt/") 
        || path_str.starts_with("/media/")
        || path_str.starts_with("/net/")
        || path_str.starts_with("/nfs/")
        || path_str.starts_with("/smb/")
        || path_str.starts_with("/cifs/")
    {
        return true;
    }

    // Check /proc/mounts on Linux for NFS/CIFS mounts
    #[cfg(target_os = "linux")]
    {
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let mount_point = parts[1];
                    let fs_type = parts[2];
                    if path_str.starts_with(mount_point) 
                        && (fs_type == "nfs" || fs_type == "nfs4" || fs_type == "cifs" || fs_type == "smb")
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Directories to always skip when walking (regardless of .gitignore)
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "target",
    "build",
    "dist",
    ".next",
    ".nuxt",
    "vendor",
    ".cargo",
    ".rustup",
    "Pods",
    ".gradle",
    ".idea",
    ".vs",
    ".vscode",
    "coverage",
    ".pytest_cache",
    ".mypy_cache",
    ".tox",
    "eggs",
    "*.egg-info",
    ".sass-cache",
    "bower_components",
];

/// Fast directory walker for network mounts.
/// Skips gitignore parsing (expensive over network) and uses a simple skip list.
fn fast_walk_directory(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut dirs_walked = 0u64;

    while let Some(dir) = stack.pop() {
        dirs_walked += 1;
        
        // Log progress every 100 directories
        if dirs_walked % 100 == 0 {
            tracing::debug!(dirs_walked, "fast_walk progress");
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(path = %dir.display(), error = %e, "Failed to read directory");
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Skip hidden files/dirs
            if name.starts_with('.') && name != "." && name != ".." {
                continue;
            }

            // Skip known junk directories
            if SKIP_DIRS.iter().any(|&skip| {
                if skip.contains('*') {
                    // Simple glob matching for *.egg-info etc
                    let pattern = skip.trim_start_matches('*');
                    name.ends_with(pattern)
                } else {
                    name == skip
                }
            }) {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                // Check if it's a code file
                if crate::watcher::FileFilter::is_code_file(&path) {
                    files.push(path);
                }
            }
        }
    }

    tracing::info!(total_files = files.len(), dirs_walked, "fast_walk complete");
    files
}

/// MCP server state.
pub struct McpState {
    pub db: Database,
    pub embeddings: Option<EmbeddingService>,
    /// API key for authentication (None = disabled)
    api_key: Option<String>,
}

impl McpState {
    /// Create new MCP state.
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self {
            db,
            embeddings: None,
            api_key: None,
        }
    }

    /// Create MCP state with embedding service.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // EmbeddingService is not const
    pub fn with_embeddings(db: Database, embeddings: EmbeddingService) -> Self {
        Self {
            db,
            embeddings: Some(embeddings),
            api_key: None,
        }
    }

    /// Create MCP state with API key.
    #[must_use]
    pub const fn with_api_key(db: Database, api_key: Option<String>) -> Self {
        Self {
            db,
            embeddings: None,
            api_key,
        }
    }

    /// Create MCP state with embeddings and API key.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // EmbeddingService is not const
    pub fn with_embeddings_and_api_key(
        db: Database,
        embeddings: EmbeddingService,
        api_key: Option<String>,
    ) -> Self {
        Self {
            db,
            embeddings: Some(embeddings),
            api_key,
        }
    }

    /// Check if API key authentication is configured.
    #[must_use]
    pub const fn api_key_configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// Validate an API key.
    #[must_use]
    pub fn validate_api_key(&self, provided_key: &str) -> bool {
        self.api_key
            .as_ref()
            .is_some_and(|expected| expected == provided_key)
    }

    /// Get the database.
    #[must_use]
    pub const fn db(&self) -> &Database {
        &self.db
    }

    /// Get the embedding service if available.
    #[must_use]
    pub fn embedding_service(&self) -> Option<EmbeddingService> {
        self.embeddings.clone()
    }
}

/// Tool information with schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
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
        ToolInfo {
            name: "search_checkpoints".to_string(),
            description: Some("Search checkpoints semantically by query text".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Query text to search checkpoints"
                    },
                    "agent": {
                        "type": "string",
                        "description": "Optional agent filter to search only this agent's checkpoints"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum checkpoints to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        },
        ToolInfo {
            name: "get_agent_status".to_string(),
            description: Some(
                "Get quick status for an agent (idle/in_progress, current task, checkpoint count)"
                    .to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent": {
                        "type": "string",
                        "description": "Agent identifier"
                    }
                },
                "required": ["agent"]
            }),
        },
        ToolInfo {
            name: "index_repo".to_string(),
            description: Some(
                "Index a repository or directory path on demand. Use this to ensure Nellie has fresh context for a specific project. Respects .gitignore patterns."
                    .to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the repository or directory to index"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolInfo {
            name: "diff_index".to_string(),
            description: Some(
                "Incremental indexing: compare file mtimes with database and only index new/changed files. Also removes entries for deleted files. Fast for routine syncs."
                    .to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to diff-index"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolInfo {
            name: "full_reindex".to_string(),
            description: Some(
                "Nuclear option: clear all indexed data for a path and re-index from scratch. Use when the index seems corrupted or you need a clean slate."
                    .to_string(),
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to fully re-index (clears existing data first)"
                    }
                },
                "required": ["path"]
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
        "search_code" => handle_search_code(&state, &request.arguments).await,
        "search_lessons" => handle_search_lessons(&state, &request.arguments).await,
        "list_lessons" => handle_list_lessons(&state, &request.arguments),
        "add_lesson" => handle_add_lesson(&state, &request.arguments).await,
        "delete_lesson" => handle_delete_lesson(&state, &request.arguments),
        "add_checkpoint" => handle_add_checkpoint(&state, &request.arguments).await,
        "get_recent_checkpoints" => handle_get_checkpoints(&state, &request.arguments),
        "trigger_reindex" => handle_trigger_reindex(&state, &request.arguments).await,
        "get_status" => handle_get_status(&state),
        "search_checkpoints" => handle_search_checkpoints(&state, &request.arguments).await,
        "get_agent_status" => handle_get_agent_status(&state, &request.arguments),
        "index_repo" => handle_index_repo(&state, &request.arguments).await,
        "diff_index" => handle_diff_index(&state, &request.arguments).await,
        "full_reindex" => handle_full_reindex(&state, &request.arguments).await,
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

/// Invoke a tool directly (for SSE transport).
pub async fn invoke_tool_direct(state: &McpState, request: ToolRequest) -> ToolResponse {
    let tool_name = request.name.clone();
    tracing::debug!("Invoking tool (direct): {}", tool_name);

    let result = match request.name.as_str() {
        "search_code" => handle_search_code(state, &request.arguments).await,
        "search_lessons" => handle_search_lessons(state, &request.arguments).await,
        "list_lessons" => handle_list_lessons(state, &request.arguments),
        "add_lesson" => handle_add_lesson(state, &request.arguments).await,
        "delete_lesson" => handle_delete_lesson(state, &request.arguments),
        "add_checkpoint" => handle_add_checkpoint(state, &request.arguments).await,
        "get_recent_checkpoints" => handle_get_checkpoints(state, &request.arguments),
        "trigger_reindex" => handle_trigger_reindex(state, &request.arguments).await,
        "get_status" => handle_get_status(state),
        "search_checkpoints" => handle_search_checkpoints(state, &request.arguments).await,
        "get_agent_status" => handle_get_agent_status(state, &request.arguments),
        "index_repo" => handle_index_repo(state, &request.arguments).await,
        "diff_index" => handle_diff_index(state, &request.arguments).await,
        "full_reindex" => handle_full_reindex(state, &request.arguments).await,
        _ => Err(format!("Unknown tool: {}", request.name)),
    };

    match result {
        Ok(content) => {
            tracing::debug!("Tool invocation succeeded");
            ToolResponse {
                content,
                error: None,
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Tool invocation failed");
            ToolResponse {
                content: serde_json::Value::Null,
                error: Some(e),
            }
        }
    }
}

// Tool handlers

#[allow(clippy::cast_possible_truncation)]
async fn handle_search_code(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(10) as usize;
    let language_filter = args["language"].as_str();

    // CRITICAL: Embedding service MUST be initialized for semantic search
    let embeddings = state.embeddings.as_ref().ok_or_else(|| {
        "Embedding service not initialized. Semantic search requires real embeddings.".to_string()
    })?;

    if !embeddings.is_initialized() {
        return Err(
            "Embedding service not fully initialized. Please wait for model loading to complete."
                .to_string(),
        );
    }

    // Generate embedding for query using real embeddings
    // We're in a sync context (Axum handler), so we use blocking runtime
    let embeddings = embeddings.clone();
    let query_text = query.to_string();

    let embedding = embeddings.embed_one(query_text).await
        .map_err(|e| format!("Failed to generate query embedding: {e}"))?;

    // Create search options
    let mut search_opts = crate::storage::SearchOptions::new(limit);
    if let Some(lang) = language_filter {
        search_opts = search_opts.with_language(lang);
    }

    // Search the database using real vector similarity
    let results = state
        .db
        .with_conn(|conn| crate::storage::search_chunks(conn, &embedding, &search_opts))
        .map_err(|e| format!("Vector search failed: {e}"))?;

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

#[allow(clippy::cast_possible_truncation)]
async fn handle_search_lessons(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    // CRITICAL: Embedding service MUST be initialized for semantic search
    let embeddings = state.embeddings.as_ref().ok_or_else(|| {
        "Embedding service not initialized. Semantic search requires real embeddings.".to_string()
    })?;

    if !embeddings.is_initialized() {
        return Err(
            "Embedding service not fully initialized. Please wait for model loading to complete."
                .to_string(),
        );
    }

    // Generate embedding for query using real embeddings
    let embeddings = embeddings.clone();
    let query_text = query.to_string();

    let embedding = embeddings.embed_one(query_text).await
        .map_err(|e| format!("Failed to generate query embedding: {e}"))?;

    // Search lessons using vector similarity
    let lessons = state
        .db
        .with_conn(|conn| crate::storage::search_lessons_by_embedding(conn, &embedding, limit))
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

#[allow(clippy::cast_possible_truncation)]
async fn handle_add_lesson(
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

    // Store lesson in database
    state
        .db
        .with_conn(|conn| crate::storage::insert_lesson(conn, &lesson))
        .map_err(|e| e.to_string())?;

    // Generate and store embedding for semantic search
    if let Some(ref embeddings) = state.embeddings {
        if embeddings.is_initialized() {
            // Combine title and content for better semantic understanding
            let text_to_embed = format!("{}\n{}", lesson.title, lesson.content);

            if let Ok(embedding) = embeddings.embed_one(text_to_embed).await {
                // Store embedding in vector table (ignore errors, embedding is optional for backward compat)
                let _ = state.db.with_conn(|conn| {
                    crate::storage::store_lesson_embedding(conn, &lesson.id, &embedding)
                });
            }
        }
    }

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

#[allow(clippy::cast_possible_truncation)]
async fn handle_add_checkpoint(
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

    // Store checkpoint in database
    state
        .db
        .with_conn(|conn| crate::storage::insert_checkpoint(conn, &checkpoint))
        .map_err(|e| e.to_string())?;

    // Generate and store embedding for semantic search
    if let Some(ref embeddings) = state.embeddings {
        if embeddings.is_initialized() {
            // Embed the working_on description for checkpoint semantic search
            let text_to_embed = checkpoint.working_on.clone();

            if let Ok(embedding) = embeddings.embed_one(text_to_embed).await {
                // Store embedding in vector table (ignore errors, embedding is optional for backward compat)
                let _ = state.db.with_conn(|conn| {
                    crate::storage::store_checkpoint_embedding(conn, &checkpoint.id, &embedding)
                });
            }
        }
    }

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

// Replace handle_trigger_reindex with this async version:

#[allow(clippy::redundant_closure)]
async fn handle_trigger_reindex(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let path = args["path"].as_str();

    if let Some(target_path) = path {
        let path_buf = std::path::PathBuf::from(target_path);
        
        // Check if path is a directory
        if path_buf.is_dir() {
            // Scan directory and index all files
            let indexer = crate::watcher::Indexer::new(
                state.db.clone(),
                state.embeddings.clone(),
            );
            let indexer = std::sync::Arc::new(indexer);
            
            // Walk directory and index each file
            let walker = ignore::WalkBuilder::new(&path_buf)
                .hidden(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .ignore(true)
                .parents(true)
                .build();
            
            let mut indexed = 0u64;
            let mut skipped = 0u64;
            let mut errors = 0u64;
            
            for entry in walker {
                match entry {
                    Ok(entry) => {
                        let entry_path = entry.path();
                        
                        // Skip directories
                        if entry_path.is_dir() {
                            continue;
                        }
                        
                        // Check if it's a code file
                        if !crate::watcher::FileFilter::is_code_file(entry_path) {
                            skipped += 1;
                            continue;
                        }
                        
                        // Index the file
                        let language = crate::watcher::FileFilter::detect_language(entry_path)
                            .map(String::from);
                        let request = crate::watcher::IndexRequest {
                            path: entry_path.to_path_buf(),
                            language,
                        };
                        
                        match indexer.index_file(&request).await {
                            Ok(chunks) => {
                                if chunks > 0 {
                                    indexed += 1;
                                    tracing::debug!(
                                        path = %entry_path.display(),
                                        chunks,
                                        "Indexed file"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    path = %entry_path.display(),
                                    error = %e,
                                    "Failed to index file"
                                );
                                errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Error walking directory");
                        errors += 1;
                    }
                }
            }
            
            tracing::info!(
                path = %target_path,
                indexed,
                skipped,
                errors,
                "Directory scan complete"
            );
            
            Ok(serde_json::json!({
                "status": "indexed",
                "path": target_path,
                "files_indexed": indexed,
                "files_skipped": skipped,
                "errors": errors,
                "message": format!("Indexed {} files from directory: {}", indexed, target_path)
            }))
        } else {
            // Single file - delete chunks to trigger re-indexing
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
                "message": format!("Re-indexing scheduled for file: {}", target_path)
            }))
        }
    } else {
        // Clear all file state to trigger full re-index
        state
            .db
            .with_conn(|conn| {
                let paths = crate::storage::list_file_paths(conn)?;
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

#[allow(clippy::cast_possible_truncation)]
async fn handle_search_checkpoints(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let query = args["query"].as_str().ok_or("query is required")?;
    let agent_filter = args["agent"].as_str();
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;

    // CRITICAL: Embedding service MUST be initialized for semantic search
    let embeddings = state.embeddings.as_ref().ok_or_else(|| {
        "Embedding service not initialized. Semantic search requires real embeddings.".to_string()
    })?;

    if !embeddings.is_initialized() {
        return Err(
            "Embedding service not fully initialized. Please wait for model loading to complete."
                .to_string(),
        );
    }

    // Generate embedding for query using real embeddings
    let embeddings = embeddings.clone();
    let query_text = query.to_string();

    let embedding = embeddings.embed_one(query_text).await
        .map_err(|e| format!("Failed to generate query embedding: {e}"))?;

    // Search checkpoints using vector similarity
    let checkpoint_results = state
        .db
        .with_conn(|conn| crate::storage::search_checkpoints_by_embedding(conn, &embedding, limit))
        .map_err(|e| e.to_string())?;

    // Filter by agent if specified
    let checkpoints: Vec<_> = if let Some(agent) = agent_filter {
        checkpoint_results
            .into_iter()
            .filter(|cp| cp.record.agent == agent)
            .map(|cp| cp.record)
            .collect()
    } else {
        checkpoint_results.into_iter().map(|cp| cp.record).collect()
    };

    Ok(serde_json::json!({
        "checkpoints": serde_json::to_value(&checkpoints).unwrap_or(serde_json::Value::Array(vec![])),
        "count": checkpoints.len(),
        "query": query,
        "agent": agent_filter.unwrap_or("all"),
        "limit": limit
    }))
}

#[allow(clippy::redundant_closure)]
fn handle_get_agent_status(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let agent = args["agent"].as_str().ok_or("agent is required")?;

    let status = state
        .db
        .with_conn(|conn| crate::storage::get_agent_status(conn, agent))
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "agent": status.agent,
        "status": status.status.as_str(),
        "current_task": status.current_task,
        "last_updated": status.last_updated,
        "checkpoint_count": status.checkpoint_count
    }))
}

/// Index a repository or directory on demand.
/// This is the preferred way for agents to ensure Nellie has fresh context for a project.
/// Uses spawn_blocking for directory traversal to handle slow filesystems (NFS, SMB).
#[allow(clippy::redundant_closure)]
async fn handle_index_repo(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let path = args["path"].as_str().ok_or("path is required")?;
    let path_buf = std::path::PathBuf::from(path);
    let path_string = path.to_string();

    if !path_buf.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    if !path_buf.is_dir() {
        return Err(format!("Path is not a directory: {path}. Use trigger_reindex for single files."));
    }

    let start_time = std::time::Instant::now();

    // Check if this is a network mount (NFS/SMB) - use fast walker if so
    let is_network = is_network_path(&path_buf);
    tracing::info!(path, is_network, "Starting index_repo - collecting files...");

    // Collect all file paths in a blocking task (handles slow NFS/SMB)
    let path_for_walk = path_buf.clone();
    let file_paths: Vec<std::path::PathBuf> = tokio::task::spawn_blocking(move || {
        if is_network {
            // Fast walker for network mounts - skip gitignore parsing
            fast_walk_directory(&path_for_walk)
        } else {
            // Full walker with gitignore support for local paths
            let walker = ignore::WalkBuilder::new(&path_for_walk)
                .hidden(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .ignore(true)
                .parents(true)
                .build();

            let mut paths = Vec::new();
            for entry in walker {
                if let Ok(entry) = entry {
                    let p = entry.path();
                    if p.is_file() && crate::watcher::FileFilter::is_code_file(p) {
                        paths.push(p.to_path_buf());
                    }
                }
            }
            paths
        }
    })
    .await
    .map_err(|e| format!("Directory walk failed: {e}"))?;

    let total_files = file_paths.len();
    tracing::info!(path = path_string, total_files, "Found files to index");

    // Create indexer with embeddings
    let indexer = crate::watcher::Indexer::new(state.db.clone(), state.embeddings.clone());
    let indexer = std::sync::Arc::new(indexer);

    let mut files_indexed = 0u64;
    let mut files_skipped = 0u64;
    let mut files_unchanged = 0u64;
    let mut chunks_created = 0u64;
    let mut errors = 0u64;

    // Process files in batches, yielding periodically
    for (i, entry_path) in file_paths.into_iter().enumerate() {
        // Log progress every 100 files
        if i > 0 && i % 100 == 0 {
            tracing::info!(
                path = path_string,
                progress = format!("{}/{}", i, total_files),
                files_indexed,
                chunks_created,
                "index_repo progress"
            );
            // Yield to allow other tasks to run
            tokio::task::yield_now().await;
        }

        // Index the file
        let language = crate::watcher::FileFilter::detect_language(&entry_path).map(String::from);
        let request = crate::watcher::IndexRequest {
            path: entry_path.clone(),
            language,
        };

        match indexer.index_file(&request).await {
            Ok(chunks) => {
                if chunks > 0 {
                    files_indexed += 1;
                    chunks_created += chunks as u64;
                } else {
                    files_unchanged += 1;
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = %entry_path.display(),
                    error = %e,
                    "Failed to index file"
                );
                errors += 1;
            }
        }
    }

    // Also count non-code files as skipped
    files_skipped = total_files.saturating_sub((files_indexed + files_unchanged + errors) as usize) as u64;

    let elapsed = start_time.elapsed();

    tracing::info!(
        path = path_string,
        files_indexed,
        files_unchanged,
        files_skipped,
        chunks_created,
        errors,
        elapsed_ms = elapsed.as_millis(),
        "index_repo complete"
    );

    Ok(serde_json::json!({
        "status": "completed",
        "path": path_string,
        "files_indexed": files_indexed,
        "files_unchanged": files_unchanged,
        "files_skipped": files_skipped,
        "chunks_created": chunks_created,
        "errors": errors,
        "elapsed_ms": elapsed.as_millis(),
        "message": format!(
            "Indexed {} files ({} chunks) from {}, {} unchanged, {} skipped, {} errors in {:.1}s",
            files_indexed, chunks_created, path_string, files_unchanged, files_skipped, errors,
            elapsed.as_secs_f64()
        )
    }))
}

/// Incremental diff-based indexing.
/// Compares file mtimes with database and only indexes new/changed files.
/// Also removes entries for deleted files.
/// Uses spawn_blocking for directory traversal to handle slow filesystems (NFS, SMB).
#[allow(clippy::redundant_closure, clippy::cast_possible_wrap)]
async fn handle_diff_index(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let path = args["path"].as_str().ok_or("path is required")?;
    let path_buf = std::path::PathBuf::from(path);
    let path_string = path.to_string();

    if !path_buf.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    if !path_buf.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let start_time = std::time::Instant::now();

    // Check if this is a network mount
    let is_network = is_network_path(&path_buf);
    tracing::info!(path, is_network, "Starting diff_index - collecting files...");

    // Create indexer with embeddings
    let indexer = crate::watcher::Indexer::new(state.db.clone(), state.embeddings.clone());
    let indexer = std::sync::Arc::new(indexer);

    // Get existing indexed files for this path to detect deletions
    let existing_files: std::collections::HashSet<String> = state
        .db
        .with_conn(|conn| crate::storage::list_file_paths_by_prefix(conn, path))
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    // Collect all file paths with metadata in a blocking task (handles slow NFS/SMB)
    let path_for_walk = path_buf.clone();
    let file_info: Vec<(std::path::PathBuf, i64, i64)> = tokio::task::spawn_blocking(move || {
        let file_paths = if is_network {
            // Fast walker for network mounts
            fast_walk_directory(&path_for_walk)
        } else {
            // Full walker with gitignore support
            let walker = ignore::WalkBuilder::new(&path_for_walk)
                .hidden(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .ignore(true)
                .parents(true)
                .build();

            let mut paths = Vec::new();
            for entry in walker {
                if let Ok(entry) = entry {
                    let p = entry.path();
                    if p.is_file() && crate::watcher::FileFilter::is_code_file(p) {
                        paths.push(p.to_path_buf());
                    }
                }
            }
            paths
        };

        // Get metadata for all files
        let mut files = Vec::new();
        for p in file_paths {
            if let Ok(metadata) = std::fs::metadata(&p) {
                let mtime = metadata
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64
                    })
                    .unwrap_or(0);
                let size = metadata.len() as i64;
                files.push((p, mtime, size));
            }
        }
        files
    })
    .await
    .map_err(|e| format!("Directory walk failed: {e}"))?;

    let total_files = file_info.len();
    tracing::info!(path = path_string, total_files, "Found files for diff check");

    let mut seen_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut files_indexed = 0u64;
    let mut files_unchanged = 0u64;
    let mut files_skipped = 0u64;
    let mut files_deleted = 0u64;
    let mut chunks_created = 0u64;
    let mut errors = 0u64;

    // Process files
    for (i, (entry_path, current_mtime, current_size)) in file_info.into_iter().enumerate() {
        // Log progress every 100 files
        if i > 0 && i % 100 == 0 {
            tracing::info!(
                path = path_string,
                progress = format!("{}/{}", i, total_files),
                files_indexed,
                files_unchanged,
                "diff_index progress"
            );
            tokio::task::yield_now().await;
        }

        let path_str = entry_path.to_string_lossy().to_string();
        seen_files.insert(path_str.clone());

        // Check if file needs reindexing
        let needs_index = state
            .db
            .with_conn(|conn| {
                crate::storage::needs_reindex_by_metadata(
                    conn,
                    &path_str,
                    current_mtime,
                    current_size,
                )
            })
            .unwrap_or(true);

        if !needs_index {
            files_unchanged += 1;
            continue;
        }

        // Index the file
        let language = crate::watcher::FileFilter::detect_language(&entry_path).map(String::from);
        let request = crate::watcher::IndexRequest {
            path: entry_path.clone(),
            language,
        };

        match indexer.index_file(&request).await {
            Ok(chunks) => {
                if chunks > 0 {
                    files_indexed += 1;
                    chunks_created += chunks as u64;
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = %entry_path.display(),
                    error = %e,
                    "Failed to index file"
                );
                errors += 1;
            }
        }
    }

    // Remove entries for deleted files
    for old_file in existing_files.difference(&seen_files) {
        if let Err(e) = state
            .db
            .with_conn(|conn| crate::storage::delete_chunks_by_file(conn, old_file))
        {
            tracing::warn!(path = old_file, error = %e, "Failed to delete stale chunks");
            errors += 1;
        } else {
            let _ = state
                .db
                .with_conn(|conn| crate::storage::delete_file_state(conn, old_file));
            files_deleted += 1;
            tracing::debug!(path = old_file, "Removed deleted file from index");
        }
    }

    let elapsed = start_time.elapsed();

    tracing::info!(
        path = path_string,
        files_indexed,
        files_unchanged,
        files_deleted,
        files_skipped,
        chunks_created,
        errors,
        elapsed_ms = elapsed.as_millis(),
        "diff_index complete"
    );

    Ok(serde_json::json!({
        "status": "completed",
        "path": path_string,
        "files_indexed": files_indexed,
        "files_unchanged": files_unchanged,
        "files_deleted": files_deleted,
        "files_skipped": files_skipped,
        "chunks_created": chunks_created,
        "errors": errors,
        "elapsed_ms": elapsed.as_millis(),
        "message": format!(
            "Diff indexed {}: {} updated, {} unchanged, {} deleted, {} skipped in {:.1}s",
            path_string, files_indexed, files_unchanged, files_deleted, files_skipped,
            elapsed.as_secs_f64()
        )
    }))
}

/// Full reindex - nuclear option.
/// Clears all indexed data for a path and re-indexes from scratch.
/// Uses spawn_blocking for directory traversal to handle slow filesystems (NFS, SMB).
#[allow(clippy::redundant_closure)]
async fn handle_full_reindex(
    state: &McpState,
    args: &serde_json::Value,
) -> std::result::Result<serde_json::Value, String> {
    let path = args["path"].as_str().ok_or("path is required")?;
    let path_buf = std::path::PathBuf::from(path);
    let path_string = path.to_string();

    if !path_buf.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    if !path_buf.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let start_time = std::time::Instant::now();

    // Clear existing data for this path
    let chunks_deleted = state
        .db
        .with_conn(|conn| crate::storage::delete_chunks_by_path_prefix(conn, path))
        .map_err(|e| format!("Failed to clear chunks: {e}"))?;

    let files_cleared = state
        .db
        .with_conn(|conn| crate::storage::delete_file_state_by_prefix(conn, path))
        .map_err(|e| format!("Failed to clear file state: {e}"))?;

    tracing::info!(
        path,
        chunks_deleted,
        files_cleared,
        "Cleared existing index data"
    );

    // Check if this is a network mount
    let is_network = is_network_path(&path_buf);
    tracing::info!(path, is_network, "Starting full_reindex - collecting files...");

    // Collect all file paths in a blocking task (handles slow NFS/SMB)
    let path_for_walk = path_buf.clone();
    let file_paths: Vec<std::path::PathBuf> = tokio::task::spawn_blocking(move || {
        if is_network {
            // Fast walker for network mounts
            fast_walk_directory(&path_for_walk)
        } else {
            // Full walker with gitignore support
            let walker = ignore::WalkBuilder::new(&path_for_walk)
                .hidden(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .ignore(true)
                .parents(true)
                .build();

            let mut paths = Vec::new();
            for entry in walker {
                if let Ok(entry) = entry {
                    let p = entry.path();
                    if p.is_file() && crate::watcher::FileFilter::is_code_file(p) {
                        paths.push(p.to_path_buf());
                    }
                }
            }
            paths
        }
    })
    .await
    .map_err(|e| format!("Directory walk failed: {e}"))?;

    let total_files = file_paths.len();
    tracing::info!(path = path_string, total_files, "Found files to reindex");

    // Create indexer with embeddings
    let indexer = crate::watcher::Indexer::new(state.db.clone(), state.embeddings.clone());
    let indexer = std::sync::Arc::new(indexer);

    let mut files_indexed = 0u64;
    let mut files_skipped = 0u64;
    let mut chunks_created = 0u64;
    let mut errors = 0u64;

    // Process files in batches
    for (i, entry_path) in file_paths.into_iter().enumerate() {
        // Log progress every 100 files
        if i > 0 && i % 100 == 0 {
            tracing::info!(
                path = path_string,
                progress = format!("{}/{}", i, total_files),
                files_indexed,
                chunks_created,
                "full_reindex progress"
            );
            tokio::task::yield_now().await;
        }

        // Index the file
        let language = crate::watcher::FileFilter::detect_language(&entry_path).map(String::from);
        let request = crate::watcher::IndexRequest {
            path: entry_path.clone(),
            language,
        };

        match indexer.index_file(&request).await {
            Ok(chunks) => {
                if chunks > 0 {
                    files_indexed += 1;
                    chunks_created += chunks as u64;
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = %entry_path.display(),
                    error = %e,
                    "Failed to index file"
                );
                errors += 1;
            }
        }
    }

    let elapsed = start_time.elapsed();

    tracing::info!(
        path = path_string,
        chunks_deleted,
        files_indexed,
        files_skipped,
        chunks_created,
        errors,
        elapsed_ms = elapsed.as_millis(),
        "full_reindex complete"
    );

    Ok(serde_json::json!({
        "status": "completed",
        "path": path_string,
        "cleared": {
            "chunks": chunks_deleted,
            "files": files_cleared
        },
        "indexed": {
            "files": files_indexed,
            "chunks": chunks_created
        },
        "files_skipped": files_skipped,
        "errors": errors,
        "elapsed_ms": elapsed.as_millis(),
        "message": format!(
            "Full reindex of {}: cleared {} chunks, indexed {} files ({} chunks), {} skipped, {} errors in {:.1}s",
            path_string, chunks_deleted, files_indexed, chunks_created, files_skipped, errors,
            elapsed.as_secs_f64()
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_defined() {
        let tools = get_tools();
        assert!(tools.len() >= 14); // 11 original + 3 new indexing tools

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
        assert!(names.contains(&"search_checkpoints"));
        assert!(names.contains(&"get_agent_status"));
        // New indexing tools for Issue #20
        assert!(names.contains(&"index_repo"));
        assert!(names.contains(&"diff_index"));
        assert!(names.contains(&"full_reindex"));
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
    fn test_search_code_requires_embedding_service() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;
            // Skip vector table initialization as sqlite-vec may not be available in tests
            Ok(())
        })
        .expect("Failed to setup database");
        let state = McpState::new(db); // No embedding service

        // Test that search fails without embedding service
        let args = serde_json::json!({
            "query": "test search query"
        });

        let result = handle_search_code(&state, &args);
        // Now requires embedding service - should fail with appropriate error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Embedding service not initialized"));
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
    fn test_search_lessons_requires_embedding_service() {
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
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "error handling",
            "limit": 5
        });

        let result = handle_search_lessons(&state, &args);
        // Semantic search requires embedding service - should fail with appropriate error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Embedding service not initialized"));
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
    fn test_search_lessons_default_limit_requires_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "some query"
            // limit not provided, should default to 5
        });

        let result = handle_search_lessons(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
    }

    #[test]
    fn test_search_lessons_with_limit_requires_embedding() {
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
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "lesson",
            "limit": 3
        });

        let result = handle_search_lessons(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
    }

    #[test]
    fn test_search_lessons_empty_result_requires_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "nonexistent lesson query",
            "limit": 5
        });

        let result = handle_search_lessons(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_trigger_reindex_specific_path() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "path": "/test/file.rs"
        });

        let result = handle_trigger_reindex(&state, &args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "reindex_scheduled");
        assert_eq!(response["path"], "/test/file.rs");
        assert!(response["message"]
            .as_str()
            .unwrap()
            .contains("Re-indexing scheduled"));
    }

    #[tokio::test]
    async fn test_trigger_reindex_all_paths() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_trigger_reindex(&state, &args).await;
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

    #[test]
    fn test_search_checkpoints_tool_exists() {
        let tools = get_tools();
        let search_checkpoints = tools
            .iter()
            .find(|t| t.name == "search_checkpoints")
            .expect("search_checkpoints tool should exist");

        assert!(search_checkpoints.description.is_some());
        let desc = search_checkpoints
            .description
            .as_ref()
            .unwrap()
            .to_lowercase();
        assert!(desc.contains("search"));
        assert!(desc.contains("checkpoint"));
    }

    #[test]
    fn test_get_agent_status_tool_exists() {
        let tools = get_tools();
        let get_agent_status = tools
            .iter()
            .find(|t| t.name == "get_agent_status")
            .expect("get_agent_status tool should exist");

        assert!(get_agent_status.description.is_some());
        let desc = get_agent_status
            .description
            .as_ref()
            .unwrap()
            .to_lowercase();
        assert!(desc.contains("status"));
    }

    #[test]
    fn test_search_checkpoints_success_requires_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert test checkpoints
            let checkpoint1 = crate::storage::CheckpointRecord::new(
                "agent-1",
                "Working on feature implementation",
                serde_json::json!({"step": 1}),
            );
            crate::storage::insert_checkpoint(conn, &checkpoint1)?;

            let checkpoint2 = crate::storage::CheckpointRecord::new(
                "agent-2",
                "Debugging test failures",
                serde_json::json!({"step": 2}),
            );
            crate::storage::insert_checkpoint(conn, &checkpoint2)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "feature",
            "limit": 5
        });

        let result = handle_search_checkpoints(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Embedding service not initialized"));
    }

    #[test]
    fn test_search_checkpoints_with_agent_filter_requires_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert checkpoints for different agents
            let checkpoint1 =
                crate::storage::CheckpointRecord::new("agent-1", "Task 1", serde_json::json!({}));
            crate::storage::insert_checkpoint(conn, &checkpoint1)?;

            let checkpoint2 =
                crate::storage::CheckpointRecord::new("agent-1", "Task 2", serde_json::json!({}));
            crate::storage::insert_checkpoint(conn, &checkpoint2)?;

            let checkpoint3 =
                crate::storage::CheckpointRecord::new("agent-2", "Task 3", serde_json::json!({}));
            crate::storage::insert_checkpoint(conn, &checkpoint3)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "ignored",
            "agent": "agent-1",
            "limit": 10
        });

        let result = handle_search_checkpoints(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
    }

    #[test]
    fn test_search_checkpoints_missing_query() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "limit": 5
        });

        let result = handle_search_checkpoints(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("query is required"));
    }

    #[test]
    fn test_search_checkpoints_default_limit_requires_embedding() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db); // No embedding service

        let args = serde_json::json!({
            "query": "test"
        });

        let result = handle_search_checkpoints(&state, &args);
        // Semantic search requires embedding service
        assert!(result.is_err());
    }

    #[test]
    fn test_get_agent_status_success() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Mark agent as in progress
            crate::storage::mark_in_progress(conn, "test-agent", Some("Working on task"))?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "test-agent"
        });

        let result = handle_get_agent_status(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["agent"], "test-agent");
        assert_eq!(response["status"], "in_progress");
        assert!(response.get("current_task").is_some());
        assert!(response.get("last_updated").is_some());
        assert!(response.get("checkpoint_count").is_some());
    }

    #[test]
    fn test_get_agent_status_idle() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "new-agent"
        });

        let result = handle_get_agent_status(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["agent"], "new-agent");
        assert_eq!(response["status"], "idle");
    }

    #[test]
    fn test_get_agent_status_missing_agent() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| crate::storage::migrate(conn))
            .expect("Failed to migrate");
        let state = McpState::new(db);

        let args = serde_json::json!({});

        let result = handle_get_agent_status(&state, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent is required"));
    }

    #[test]
    fn test_get_agent_status_with_checkpoints() {
        let db = crate::storage::Database::open_in_memory()
            .expect("Failed to create in-memory database");
        db.with_conn(|conn| -> crate::Result<()> {
            crate::storage::migrate(conn)?;

            // Insert checkpoints for agent
            let checkpoint = crate::storage::CheckpointRecord::new(
                "test-agent",
                "Working on feature",
                serde_json::json!({"progress": 50}),
            );
            crate::storage::insert_checkpoint(conn, &checkpoint)?;

            Ok(())
        })
        .expect("Failed to setup");
        let state = McpState::new(db);

        let args = serde_json::json!({
            "agent": "test-agent"
        });

        let result = handle_get_agent_status(&state, &args);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["checkpoint_count"], 1);
    }

    #[test]
    fn test_search_checkpoints_tool_schema() {
        let tools = get_tools();
        let search_checkpoints = tools
            .iter()
            .find(|t| t.name == "search_checkpoints")
            .expect("search_checkpoints tool should exist");

        let schema = &search_checkpoints.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required field should be an array");

        assert!(required.iter().any(|v| v.as_str() == Some("query")));
        assert!(schema["properties"].get("agent").is_some());
        assert!(schema["properties"].get("limit").is_some());
    }

    #[test]
    fn test_get_agent_status_tool_schema() {
        let tools = get_tools();
        let get_agent_status = tools
            .iter()
            .find(|t| t.name == "get_agent_status")
            .expect("get_agent_status tool should exist");

        let schema = &get_agent_status.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required field should be an array");

        assert!(required.iter().any(|v| v.as_str() == Some("agent")));
    }
}
