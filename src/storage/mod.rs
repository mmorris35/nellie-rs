//! `SQLite` storage with `sqlite-vec` for vector search.
//!
//! This module provides persistent storage for:
//! - Code chunks with embeddings
//! - Lessons learned
//! - Agent checkpoints
//! - File state for incremental indexing

mod connection;
mod schema;
mod vector;

pub use connection::Database;
pub use schema::{migrate, verify_schema, SCHEMA_VERSION};
pub use vector::{
    create_vec_table, delete_vector, insert_vector, load_extension, search_similar, EMBEDDING_DIM,
};

/// Initialize storage with migrations.
///
/// # Errors
///
/// Returns an error if database initialization fails.
pub fn init_storage(db: &Database) -> crate::Result<()> {
    db.with_conn(|conn| {
        // Load sqlite-vec extension (optional - may not be available)
        if let Err(e) = load_extension(conn) {
            tracing::warn!("sqlite-vec extension not available: {e}");
        }

        // Run migrations
        migrate(conn)?;

        // Verify schema
        verify_schema(conn)?;

        tracing::info!("Storage initialized, schema version {SCHEMA_VERSION}");
        Ok(())
    })
}
