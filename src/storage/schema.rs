//! Database schema definitions and migrations.
//!
//! Provides versioned schema migrations for safe database upgrades.

use rusqlite::Connection;

use crate::error::StorageError;
use crate::Result;

/// Current schema version.
pub const SCHEMA_VERSION: i32 = 1;

/// Run all pending migrations.
///
/// # Errors
///
/// Returns an error if migrations fail.
pub fn migrate(conn: &Connection) -> Result<()> {
    // Create migrations table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| StorageError::Migration(format!("failed to create migrations table: {e}")))?;

    let current_version = get_current_version(conn)?;
    tracing::info!(
        current = current_version,
        target = SCHEMA_VERSION,
        "Checking database migrations"
    );

    if current_version < 1 {
        migrate_v1(conn)?;
    }

    // Add future migrations here:
    // if current_version < 2 {
    //     migrate_v2(conn)?;
    // }

    Ok(())
}

/// Get the current schema version.
fn get_current_version(conn: &Connection) -> Result<i32> {
    let result = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    );

    match result {
        Ok(version) => Ok(version),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(StorageError::Migration(format!("failed to get version: {e}")).into()),
    }
}

/// Record a migration as applied.
fn record_migration(conn: &Connection, version: i32) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let now_i64 = i64::try_from(now).unwrap_or_default();

    conn.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)",
        rusqlite::params![version, now_i64],
    )
    .map_err(|e| StorageError::Migration(format!("failed to record migration: {e}")))?;

    Ok(())
}

/// Migration v1: Initial schema with all tables.
fn migrate_v1(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v1: Initial schema");

    conn.execute_batch(
        r"
        -- Code chunks table
        CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            content TEXT NOT NULL,
            language TEXT,
            file_hash TEXT NOT NULL,
            indexed_at INTEGER NOT NULL,
            UNIQUE(file_path, chunk_index)
        );

        CREATE INDEX IF NOT EXISTS idx_chunks_file_path ON chunks(file_path);
        CREATE INDEX IF NOT EXISTS idx_chunks_file_hash ON chunks(file_hash);
        CREATE INDEX IF NOT EXISTS idx_chunks_language ON chunks(language);

        -- Lessons table
        CREATE TABLE IF NOT EXISTS lessons (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT NOT NULL,  -- JSON array
            severity TEXT NOT NULL DEFAULT 'info',
            agent TEXT,
            repo TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_lessons_severity ON lessons(severity);
        CREATE INDEX IF NOT EXISTS idx_lessons_agent ON lessons(agent);
        CREATE INDEX IF NOT EXISTS idx_lessons_created_at ON lessons(created_at);

        -- Checkpoints table
        CREATE TABLE IF NOT EXISTS checkpoints (
            id TEXT PRIMARY KEY,
            agent TEXT NOT NULL,
            repo TEXT,
            session_id TEXT,
            working_on TEXT NOT NULL,
            state TEXT NOT NULL,  -- JSON object
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_checkpoints_agent ON checkpoints(agent);
        CREATE INDEX IF NOT EXISTS idx_checkpoints_repo ON checkpoints(repo);
        CREATE INDEX IF NOT EXISTS idx_checkpoints_created_at ON checkpoints(created_at);

        -- File state for incremental indexing
        CREATE TABLE IF NOT EXISTS file_state (
            path TEXT PRIMARY KEY,
            mtime INTEGER NOT NULL,
            size INTEGER NOT NULL,
            hash TEXT NOT NULL,
            last_indexed INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_file_state_mtime ON file_state(mtime);

        -- Watch directories configuration
        CREATE TABLE IF NOT EXISTS watch_dirs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        );
        ",
    )
    .map_err(|e| StorageError::Migration(format!("v1 migration failed: {e}")))?;

    record_migration(conn, 1)?;
    tracing::info!("Migration v1 complete");

    Ok(())
}

/// Verify all expected tables exist.
///
/// # Errors
///
/// Returns an error if any expected table is missing from the schema.
pub fn verify_schema(conn: &Connection) -> Result<()> {
    let tables = [
        "chunks",
        "lessons",
        "checkpoints",
        "file_state",
        "watch_dirs",
    ];

    for table in tables {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            return Err(StorageError::Migration(format!("table '{table}' not found")).into());
        }
    }

    tracing::debug!("Schema verification passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_migrate_empty_database() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;
            verify_schema(conn)?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_migrate_idempotent() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            // Run migrations twice
            migrate(conn)?;
            migrate(conn)?;
            verify_schema(conn)?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_schema_version_tracking() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            let version = get_current_version(conn)?;
            assert_eq!(version, SCHEMA_VERSION);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_chunks_table_structure() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            // Insert a chunk
            conn.execute(
                "INSERT INTO chunks (file_path, chunk_index, start_line, end_line, content, \
                 language, file_hash, indexed_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "/test/file.rs",
                    0,
                    1,
                    10,
                    "fn main() {}",
                    "rust",
                    "abc123",
                    1234567890i64
                ],
            )
            .unwrap();

            // Verify we can read it back
            let content: String = conn
                .query_row(
                    "SELECT content FROM chunks WHERE file_path = ?",
                    ["/test/file.rs"],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(content, "fn main() {}");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_lessons_table_structure() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            conn.execute(
                "INSERT INTO lessons (id, title, content, tags, severity, created_at, \
                 updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "lesson-1",
                    "Test Lesson",
                    "This is a test lesson content",
                    r#"["rust", "testing"]"#,
                    "info",
                    1234567890i64,
                    1234567890i64
                ],
            )
            .unwrap();

            let title: String = conn
                .query_row(
                    "SELECT title FROM lessons WHERE id = ?",
                    ["lesson-1"],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(title, "Test Lesson");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_checkpoints_table_structure() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            conn.execute(
                "INSERT INTO checkpoints (id, agent, repo, working_on, state, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "cp-1",
                    "test-agent",
                    "nellie-rs",
                    "Implementing feature X",
                    r#"{"key": "value"}"#,
                    1234567890i64
                ],
            )
            .unwrap();

            let working_on: String = conn
                .query_row(
                    "SELECT working_on FROM checkpoints WHERE id = ?",
                    ["cp-1"],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(working_on, "Implementing feature X");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_file_state_table_structure() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            conn.execute(
                "INSERT INTO file_state (path, mtime, size, hash, last_indexed)
                 VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![
                    "/test/file.rs",
                    1234567890i64,
                    1024i64,
                    "abc123",
                    1234567890i64
                ],
            )
            .unwrap();

            let hash: String = conn
                .query_row(
                    "SELECT hash FROM file_state WHERE path = ?",
                    ["/test/file.rs"],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(hash, "abc123");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_unique_chunk_constraint() {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| {
            migrate(conn)?;

            // Insert first chunk
            conn.execute(
                "INSERT INTO chunks (file_path, chunk_index, start_line, end_line, content, \
                 file_hash, indexed_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "/test/file.rs",
                    0,
                    1,
                    10,
                    "content1",
                    "hash1",
                    1234567890i64
                ],
            )
            .unwrap();

            // Try to insert duplicate - should fail
            let result = conn.execute(
                "INSERT INTO chunks (file_path, chunk_index, start_line, end_line, content, \
                 file_hash, indexed_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    "/test/file.rs",
                    0,
                    1,
                    10,
                    "content2",
                    "hash2",
                    1234567890i64
                ],
            );

            assert!(result.is_err());

            Ok(())
        })
        .unwrap();
    }
}
