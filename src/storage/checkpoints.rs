//! Checkpoint storage operations.

use rusqlite::{params, Connection};

use super::models::CheckpointRecord;
use crate::error::StorageError;
use crate::Result;

/// Insert a new checkpoint.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn insert_checkpoint(conn: &Connection, checkpoint: &CheckpointRecord) -> Result<()> {
    let state_json = serde_json::to_string(&checkpoint.state)
        .map_err(|e| StorageError::Database(format!("failed to serialize state: {e}")))?;

    conn.execute(
        "INSERT INTO checkpoints (id, agent, repo, session_id, working_on, state, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            checkpoint.id,
            checkpoint.agent,
            checkpoint.repo,
            checkpoint.session_id,
            checkpoint.working_on,
            state_json,
            checkpoint.created_at,
        ],
    )
    .map_err(|e| StorageError::Database(format!("failed to insert checkpoint: {e}")))?;

    tracing::trace!(id = %checkpoint.id, agent = %checkpoint.agent, "Inserted checkpoint");
    Ok(())
}

/// Get a checkpoint by ID.
///
/// # Errors
///
/// Returns an error if the checkpoint is not found or database operation fails.
pub fn get_checkpoint(conn: &Connection, id: &str) -> Result<CheckpointRecord> {
    conn.query_row(
        "SELECT id, agent, repo, session_id, working_on, state, created_at
         FROM checkpoints WHERE id = ?",
        [id],
        |row| {
            let state_json: String = row.get(5)?;
            let state: serde_json::Value = serde_json::from_str(&state_json).unwrap_or_default();

            Ok(CheckpointRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                repo: row.get(2)?,
                session_id: row.get(3)?,
                working_on: row.get(4)?,
                state,
                created_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound {
            entity: "checkpoint",
            id: id.to_string(),
        }
        .into(),
        e => StorageError::Database(format!("failed to get checkpoint: {e}")).into(),
    })
}

/// Delete a checkpoint by ID.
///
/// # Errors
///
/// Returns an error if the checkpoint is not found or database operation fails.
pub fn delete_checkpoint(conn: &Connection, id: &str) -> Result<()> {
    let rows = conn
        .execute("DELETE FROM checkpoints WHERE id = ?", [id])
        .map_err(|e| StorageError::Database(e.to_string()))?;

    if rows == 0 {
        return Err(StorageError::NotFound {
            entity: "checkpoint",
            id: id.to_string(),
        }
        .into());
    }

    Ok(())
}

/// Get recent checkpoints for an agent.
///
/// Returns the most recent `limit` checkpoints for a given agent,
/// ordered by creation time (newest first).
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn get_recent_checkpoints(
    conn: &Connection,
    agent: &str,
    limit: usize,
) -> Result<Vec<CheckpointRecord>> {
    let limit_i64 = i64::try_from(limit).unwrap_or(0);
    let mut stmt = conn
        .prepare(
            "SELECT id, agent, repo, session_id, working_on, state, created_at
             FROM checkpoints
             WHERE agent = ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let checkpoints = stmt
        .query_map(params![agent, limit_i64], |row| {
            let state_json: String = row.get(5)?;
            let state: serde_json::Value = serde_json::from_str(&state_json).unwrap_or_default();

            Ok(CheckpointRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                repo: row.get(2)?,
                session_id: row.get(3)?,
                working_on: row.get(4)?,
                state,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for cp in checkpoints {
        result.push(cp.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// Get checkpoints for an agent within a time range.
///
/// Returns checkpoints created at or after `since_timestamp`, ordered by
/// creation time (newest first), limited to `limit` results.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn get_checkpoints_since(
    conn: &Connection,
    agent: &str,
    since_timestamp: i64,
    limit: usize,
) -> Result<Vec<CheckpointRecord>> {
    let limit_i64 = i64::try_from(limit).unwrap_or(0);
    let mut stmt = conn
        .prepare(
            "SELECT id, agent, repo, session_id, working_on, state, created_at
             FROM checkpoints
             WHERE agent = ? AND created_at >= ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let checkpoints = stmt
        .query_map(params![agent, since_timestamp, limit_i64], |row| {
            let state_json: String = row.get(5)?;
            let state: serde_json::Value = serde_json::from_str(&state_json).unwrap_or_default();

            Ok(CheckpointRecord {
                id: row.get(0)?,
                agent: row.get(1)?,
                repo: row.get(2)?,
                session_id: row.get(3)?,
                working_on: row.get(4)?,
                state,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for cp in checkpoints {
        result.push(cp.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// Get the most recent checkpoint for an agent.
///
/// Returns `None` if the agent has no checkpoints.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn get_latest_checkpoint(conn: &Connection, agent: &str) -> Result<Option<CheckpointRecord>> {
    let checkpoints = get_recent_checkpoints(conn, agent, 1)?;
    Ok(checkpoints.into_iter().next())
}

/// Count checkpoints for an agent.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn count_checkpoints(conn: &Connection, agent: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE agent = ?",
        [agent],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(e.to_string()).into())
}

/// Delete old checkpoints for an agent, keeping only the most recent N.
///
/// Returns the number of checkpoints deleted.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn cleanup_old_checkpoints(conn: &Connection, agent: &str, keep: usize) -> Result<usize> {
    let sql = "DELETE FROM checkpoints \
         WHERE agent = ? AND id NOT IN ( \
             SELECT id FROM checkpoints WHERE agent = ? ORDER BY created_at DESC LIMIT ? \
         )";

    let keep_i64 = i64::try_from(keep).unwrap_or(0);
    let deleted = conn
        .execute(sql, params![agent, agent, keep_i64])
        .map_err(|e| StorageError::Database(e.to_string()))?;

    if deleted > 0 {
        tracing::debug!(agent, deleted, "Cleaned up old checkpoints");
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{migrate, Database};

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();
        db
    }

    #[test]
    fn test_insert_and_get() {
        let db = setup_db();

        db.with_conn(|conn| {
            let checkpoint = CheckpointRecord::new(
                "test-agent",
                "Working on feature X",
                serde_json::json!({"key": "value"}),
            )
            .with_repo("test-repo");

            insert_checkpoint(conn, &checkpoint)?;

            let retrieved = get_checkpoint(conn, &checkpoint.id)?;
            assert_eq!(retrieved.agent, "test-agent");
            assert_eq!(retrieved.working_on, "Working on feature X");
            assert_eq!(retrieved.repo, Some("test-repo".to_string()));

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_get_recent() {
        let db = setup_db();

        db.with_conn(|conn| {
            for i in 0..5 {
                let cp =
                    CheckpointRecord::new("agent1", format!("Task {i}"), serde_json::json!({}));
                insert_checkpoint(conn, &cp)?;
            }

            let recent = get_recent_checkpoints(conn, "agent1", 3)?;
            assert_eq!(recent.len(), 3);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_cleanup() {
        let db = setup_db();

        db.with_conn(|conn| {
            for i in 0..10 {
                let cp =
                    CheckpointRecord::new("agent1", format!("Task {i}"), serde_json::json!({}));
                insert_checkpoint(conn, &cp)?;
            }

            assert_eq!(count_checkpoints(conn, "agent1")?, 10);

            cleanup_old_checkpoints(conn, "agent1", 3)?;

            assert_eq!(count_checkpoints(conn, "agent1")?, 3);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_get_latest() {
        let db = setup_db();

        db.with_conn(|conn| {
            assert!(get_latest_checkpoint(conn, "agent1")?.is_none());

            let cp = CheckpointRecord::new("agent1", "Latest task", serde_json::json!({}));
            insert_checkpoint(conn, &cp)?;

            let latest = get_latest_checkpoint(conn, "agent1")?.unwrap();
            assert_eq!(latest.working_on, "Latest task");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_delete() {
        let db = setup_db();

        db.with_conn(|conn| {
            let cp = CheckpointRecord::new("agent1", "Task", serde_json::json!({}));
            insert_checkpoint(conn, &cp)?;

            let retrieved = get_checkpoint(conn, &cp.id)?;
            assert_eq!(retrieved.id, cp.id);

            delete_checkpoint(conn, &cp.id)?;

            let result = get_checkpoint(conn, &cp.id);
            assert!(result.is_err());

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_get_checkpoints_since() {
        let db = setup_db();

        db.with_conn(|conn| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            // Create checkpoint now
            let cp = CheckpointRecord::new("agent1", "Task", serde_json::json!({}));
            insert_checkpoint(conn, &cp)?;

            // Query with since_timestamp = now (should find the checkpoint)
            let results = get_checkpoints_since(conn, "agent1", now, 10)?;
            assert_eq!(results.len(), 1);

            // Query with future timestamp (should find nothing)
            let results = get_checkpoints_since(conn, "agent1", now + 1000, 10)?;
            assert_eq!(results.len(), 0);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_multiple_agents() {
        let db = setup_db();

        db.with_conn(|conn| {
            let cp1 = CheckpointRecord::new("agent1", "Task 1", serde_json::json!({}));
            let cp2 = CheckpointRecord::new("agent2", "Task 2", serde_json::json!({}));

            insert_checkpoint(conn, &cp1)?;
            insert_checkpoint(conn, &cp2)?;

            assert_eq!(count_checkpoints(conn, "agent1")?, 1);
            assert_eq!(count_checkpoints(conn, "agent2")?, 1);
            assert_eq!(count_checkpoints(conn, "agent3")?, 0);

            let agent1_checkpoints = get_recent_checkpoints(conn, "agent1", 10)?;
            assert_eq!(agent1_checkpoints.len(), 1);
            assert_eq!(agent1_checkpoints[0].agent, "agent1");

            Ok(())
        })
        .unwrap();
    }
}
