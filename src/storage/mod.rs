//! `SQLite` storage with `sqlite-vec` for vector search.
//!
//! This module provides persistent storage for:
//! - Code chunks with embeddings
//! - Lessons learned
//! - Agent checkpoints
//! - Agent status tracking
//! - File state for incremental indexing

mod agent_status;
mod checkpoints;
mod checkpoints_search;
mod chunks;
mod connection;
mod file_state;
mod lessons;
mod lessons_search;
mod models;
mod schema;
mod search;
mod vector;

pub use agent_status::{
    cleanup_stale_statuses, count_agents_in_progress, get_agent_status, get_agents_in_progress,
    get_all_agent_statuses, has_in_progress_work, mark_idle, mark_in_progress, AgentStatus,
    AgentStatusInfo,
};
pub use checkpoints::{
    cleanup_old_checkpoints, count_checkpoints, delete_checkpoint, get_checkpoint,
    get_checkpoints_since, get_latest_checkpoint, get_recent_checkpoints, insert_checkpoint,
};
pub use checkpoints_search::{
    init_checkpoint_vectors, search_checkpoints_by_agent, search_checkpoints_by_agent_and_repo,
    search_checkpoints_by_embedding, search_checkpoints_by_repo, search_checkpoints_by_session,
    search_checkpoints_by_text, store_checkpoint_embedding,
};
pub use chunks::{
    count_chunks, count_chunks_for_file, delete_chunk, delete_chunks_by_file, get_chunk,
    get_chunks_by_file, init_chunk_vectors, insert_chunk, insert_chunks_batch,
    update_chunk_embedding,
};
pub use connection::Database;
pub use file_state::{
    count_tracked_files, delete_file_state, find_stale_entries, get_file_state, list_file_paths,
    needs_reindex, upsert_file_state,
};
pub use lessons::{
    count_lessons, delete_lesson, get_lesson, insert_lesson, list_lessons, list_lessons_by_agent,
    list_lessons_by_severity, update_lesson,
};
pub use lessons_search::{
    filter_lessons_by_tag_and_severity, get_all_tags, init_lesson_vectors,
    search_lessons_by_embedding, search_lessons_by_tag, search_lessons_by_tags_all,
    search_lessons_by_tags_any, search_lessons_by_text, store_lesson_embedding,
};
pub use models::{CheckpointRecord, ChunkRecord, FileState, LessonRecord, SearchResult};
pub use schema::{migrate, verify_schema, SCHEMA_VERSION};
pub use search::{search_chunks, search_chunks_by_text, SearchOptions};
pub use vector::{
    create_vec_table, delete_vector, insert_vector, load_extension, search_similar, EMBEDDING_DIM,
};

/// Initialize storage with migrations.
///
/// # Errors
///
/// Returns an error if database initialization fails or sqlite-vec extension cannot be loaded.
pub fn init_storage(db: &Database) -> crate::Result<()> {
    db.with_conn(|conn| {
        // Load sqlite-vec extension - REQUIRED for vector search
        load_extension(conn)?;

        // Run migrations
        migrate(conn)?;

        // Verify schema
        verify_schema(conn)?;

        tracing::info!("Storage initialized, schema version {SCHEMA_VERSION}");
        Ok(())
    })
}
