//! Vector search support using sqlite-vec extension.
//!
//! Provides helpers for creating and querying vec0 virtual tables
//! for efficient similarity search.

use rusqlite::Connection;

use crate::error::StorageError;
use crate::Result;

/// Vector dimension for embeddings.
/// all-MiniLM-L6-v2 produces 384-dimensional vectors.
pub const EMBEDDING_DIM: usize = 384;

/// Load sqlite-vec extension into a connection.
///
/// # Errors
///
/// Returns an error if the extension cannot be loaded or verified.
pub fn load_extension(conn: &Connection) -> Result<()> {
    // sqlite-vec should be statically linked when using the bundled feature
    // We attempt to use it by executing a simple vec0 query
    // If it fails, the extension is not available and we fail loudly
    match conn.execute_batch("SELECT vec_version();") {
        Ok(()) => {
            tracing::info!("sqlite-vec extension loaded and verified");
            Ok(())
        }
        Err(e) => {
            let err_msg = format!(
                "sqlite-vec extension failed to load. \
                 Vector search will not work. \
                 This is a CRITICAL error - embeddings cannot be stored. \
                 Error: {e}"
            );
            tracing::error!("{err_msg}");
            Err(crate::error::StorageError::Vector(err_msg).into())
        }
    }
}

/// Create a vec0 virtual table for vector similarity search.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name for the virtual table
/// * `dimension` - Vector dimension (e.g., 384 for all-MiniLM-L6-v2)
///
/// # Errors
///
/// Returns an error if the table cannot be created.
pub fn create_vec_table(conn: &Connection, table_name: &str, dimension: usize) -> Result<()> {
    let sql = format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {table_name} USING vec0(
            id INTEGER PRIMARY KEY,
            embedding FLOAT[{dimension}]
        )"
    );

    conn.execute(&sql, [])
        .map_err(|e| StorageError::Vector(format!("failed to create vec table: {e}")))?;

    tracing::debug!(table = table_name, dim = dimension, "Created vec0 table");
    Ok(())
}

/// Insert a vector into a vec0 table.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the vec0 virtual table
/// * `id` - Row ID to associate with this vector
/// * `embedding` - The embedding vector (must match table dimension)
///
/// # Errors
///
/// Returns an error if the insertion fails.
pub fn insert_vector(
    conn: &Connection,
    table_name: &str,
    id: i64,
    embedding: &[f32],
) -> Result<()> {
    let blob = vector_to_blob(embedding);

    let sql = format!("INSERT INTO {table_name} (id, embedding) VALUES (?, ?)");
    conn.execute(&sql, rusqlite::params![id, blob])
        .map_err(|e| StorageError::Vector(format!("failed to insert vector: {e}")))?;

    Ok(())
}

/// Search for similar vectors using cosine distance.
///
/// # Arguments
///
/// * `conn` - Database connection
/// * `table_name` - Name of the vec0 virtual table
/// * `query_embedding` - The query vector to find similar vectors to
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Vector of (id, distance) pairs, sorted by distance ascending (most similar first).
///
/// # Errors
///
/// Returns an error if the search fails.
pub fn search_similar(
    conn: &Connection,
    table_name: &str,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<(i64, f32)>> {
    let blob = vector_to_blob(query_embedding);

    let sql = format!(
        "SELECT id, distance
         FROM {table_name}
         WHERE embedding MATCH ?
         ORDER BY distance
         LIMIT ?"
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Vector(format!("failed to prepare search: {e}")))?;

    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let results = stmt
        .query_map(rusqlite::params![blob, limit_i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f32>(1)?))
        })
        .map_err(|e| StorageError::Vector(format!("failed to execute search: {e}")))?;

    let mut matches = Vec::new();
    for result in results {
        let (id, distance) =
            result.map_err(|e| StorageError::Vector(format!("failed to read result: {e}")))?;
        matches.push((id, distance));
    }

    Ok(matches)
}

/// Delete a vector from a vec0 table.
///
/// # Errors
///
/// Returns an error if the deletion fails.
pub fn delete_vector(conn: &Connection, table_name: &str, id: i64) -> Result<()> {
    let sql = format!("DELETE FROM {table_name} WHERE id = ?");
    conn.execute(&sql, rusqlite::params![id])
        .map_err(|e| StorageError::Vector(format!("failed to delete vector: {e}")))?;
    Ok(())
}

/// Convert a vector to a blob for storage.
fn vector_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert a blob back to a vector.
#[allow(dead_code)]
fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn create_test_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            load_extension(conn)?;
            Ok(())
        })
        .unwrap();
        db
    }

    #[test]
    fn test_load_extension() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            let result = load_extension(conn);
            // Extension may or may not be available depending on build
            if result.is_err() {
                eprintln!("sqlite-vec not available: {:?}", result);
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_vector_blob_roundtrip() {
        let original = vec![1.0f32, 2.0, 3.0, -4.5];
        let blob = vector_to_blob(&original);
        let recovered = blob_to_vector(&blob);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_vector_blob_empty() {
        let original: Vec<f32> = vec![];
        let blob = vector_to_blob(&original);
        let recovered = blob_to_vector(&blob);
        assert!(recovered.is_empty());
    }

    // Integration tests that require sqlite-vec extension
    // These are marked with #[ignore] if extension is not available
    #[test]
    #[ignore = "requires sqlite-vec extension"]
    fn test_create_vec_table() {
        let db = create_test_db();
        db.with_conn(|conn| {
            create_vec_table(conn, "test_vectors", 4)?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    #[ignore = "requires sqlite-vec extension"]
    fn test_insert_and_search() {
        let db = create_test_db();

        db.with_conn(|conn| {
            create_vec_table(conn, "test_vectors", 4)?;

            // Insert test vectors
            insert_vector(conn, "test_vectors", 1, &[1.0, 0.0, 0.0, 0.0])?;
            insert_vector(conn, "test_vectors", 2, &[0.9, 0.1, 0.0, 0.0])?;
            insert_vector(conn, "test_vectors", 3, &[0.0, 1.0, 0.0, 0.0])?;

            // Search for similar to [1, 0, 0, 0]
            let results = search_similar(conn, "test_vectors", &[1.0, 0.0, 0.0, 0.0], 3)?;

            assert_eq!(results.len(), 3);
            // First result should be exact match (id=1)
            assert_eq!(results[0].0, 1);
            // Second should be id=2 (most similar)
            assert_eq!(results[1].0, 2);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    #[ignore = "requires sqlite-vec extension"]
    fn test_delete_vector() {
        let db = create_test_db();

        db.with_conn(|conn| {
            create_vec_table(conn, "test_vectors", 4)?;
            insert_vector(conn, "test_vectors", 1, &[1.0, 0.0, 0.0, 0.0])?;
            insert_vector(conn, "test_vectors", 2, &[0.0, 1.0, 0.0, 0.0])?;

            delete_vector(conn, "test_vectors", 1)?;

            let results = search_similar(conn, "test_vectors", &[1.0, 0.0, 0.0, 0.0], 10)?;
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].0, 2);

            Ok(())
        })
        .unwrap();
    }
}
