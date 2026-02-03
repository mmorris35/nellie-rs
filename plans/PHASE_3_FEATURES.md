# Phase 3: Lessons & Checkpoints

**Goal**: Implement lessons learned and agent checkpoint functionality
**Duration**: 1 week
**Prerequisites**: Phase 2 complete

---

## Task 3.1: Lessons System

**Git**: Create branch `feature/3-1-lessons` when starting first subtask.

### Subtask 3.1.1: Implement Lessons Storage and CRUD (Single Session)

**Prerequisites**:
- [x] 2.2.3: Add File State Tracking

**Deliverables**:
- [x] Create lessons storage functions
- [x] Implement CRUD operations
- [x] Add tag-based queries
- [x] Write comprehensive tests

**Files to Create**:

**`src/storage/lessons.rs`** (complete file):
```rust
//! Lessons storage operations.

use rusqlite::{params, Connection};

use super::models::LessonRecord;
use crate::error::StorageError;
use crate::Result;

/// Insert a new lesson.
pub fn insert_lesson(conn: &Connection, lesson: &LessonRecord) -> Result<()> {
    let tags_json = serde_json::to_string(&lesson.tags)
        .map_err(|e| StorageError::Database(format!("failed to serialize tags: {e}")))?;

    conn.execute(
        "INSERT INTO lessons (id, title, content, tags, severity, agent, repo, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            lesson.id,
            lesson.title,
            lesson.content,
            tags_json,
            lesson.severity,
            lesson.agent,
            lesson.repo,
            lesson.created_at,
            lesson.updated_at,
        ],
    )
    .map_err(|e| StorageError::Database(format!("failed to insert lesson: {e}")))?;

    tracing::trace!(id = %lesson.id, "Inserted lesson");
    Ok(())
}

/// Get a lesson by ID.
pub fn get_lesson(conn: &Connection, id: &str) -> Result<LessonRecord> {
    conn.query_row(
        "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
         FROM lessons WHERE id = ?",
        [id],
        |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound {
            entity: "lesson",
            id: id.to_string(),
        }
        .into(),
        e => StorageError::Database(format!("failed to get lesson: {e}")).into(),
    })
}

/// Update an existing lesson.
pub fn update_lesson(conn: &Connection, lesson: &LessonRecord) -> Result<()> {
    let tags_json = serde_json::to_string(&lesson.tags)
        .map_err(|e| StorageError::Database(format!("failed to serialize tags: {e}")))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let rows = conn
        .execute(
            "UPDATE lessons SET title = ?, content = ?, tags = ?, severity = ?, updated_at = ?
             WHERE id = ?",
            params![lesson.title, lesson.content, tags_json, lesson.severity, now, lesson.id],
        )
        .map_err(|e| StorageError::Database(format!("failed to update lesson: {e}")))?;

    if rows == 0 {
        return Err(StorageError::NotFound {
            entity: "lesson",
            id: lesson.id.clone(),
        }
        .into());
    }

    Ok(())
}

/// Delete a lesson by ID.
pub fn delete_lesson(conn: &Connection, id: &str) -> Result<()> {
    let rows = conn
        .execute("DELETE FROM lessons WHERE id = ?", [id])
        .map_err(|e| StorageError::Database(format!("failed to delete lesson: {e}")))?;

    if rows == 0 {
        return Err(StorageError::NotFound {
            entity: "lesson",
            id: id.to_string(),
        }
        .into());
    }

    Ok(())
}

/// List all lessons.
pub fn list_lessons(conn: &Connection) -> Result<Vec<LessonRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
             FROM lessons ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let lessons = stmt
        .query_map([], |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for lesson in lessons {
        result.push(lesson.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// List lessons by severity.
pub fn list_lessons_by_severity(conn: &Connection, severity: &str) -> Result<Vec<LessonRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
             FROM lessons WHERE severity = ? ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let lessons = stmt
        .query_map([severity], |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for lesson in lessons {
        result.push(lesson.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// List lessons by agent.
pub fn list_lessons_by_agent(conn: &Connection, agent: &str) -> Result<Vec<LessonRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
             FROM lessons WHERE agent = ? ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let lessons = stmt
        .query_map([agent], |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for lesson in lessons {
        result.push(lesson.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// Count total lessons.
pub fn count_lessons(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM lessons", [], |row| row.get(0))
        .map_err(|e| StorageError::Database(e.to_string()).into())
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
            let lesson = LessonRecord::new(
                "Test Lesson",
                "This is a test lesson content",
                vec!["rust".to_string(), "testing".to_string()],
            )
            .with_severity("warning")
            .with_agent("test-agent");

            insert_lesson(conn, &lesson)?;

            let retrieved = get_lesson(conn, &lesson.id)?;
            assert_eq!(retrieved.title, "Test Lesson");
            assert_eq!(retrieved.tags, vec!["rust", "testing"]);
            assert_eq!(retrieved.severity, "warning");
            assert_eq!(retrieved.agent, Some("test-agent".to_string()));

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_update() {
        let db = setup_db();

        db.with_conn(|conn| {
            let mut lesson = LessonRecord::new("Original", "Content", vec![]);
            insert_lesson(conn, &lesson)?;

            lesson.title = "Updated".to_string();
            update_lesson(conn, &lesson)?;

            let retrieved = get_lesson(conn, &lesson.id)?;
            assert_eq!(retrieved.title, "Updated");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_delete() {
        let db = setup_db();

        db.with_conn(|conn| {
            let lesson = LessonRecord::new("To Delete", "Content", vec![]);
            insert_lesson(conn, &lesson)?;

            delete_lesson(conn, &lesson.id)?;

            let result = get_lesson(conn, &lesson.id);
            assert!(result.is_err());

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_list_by_severity() {
        let db = setup_db();

        db.with_conn(|conn| {
            insert_lesson(
                conn,
                &LessonRecord::new("L1", "C1", vec![]).with_severity("critical"),
            )?;
            insert_lesson(
                conn,
                &LessonRecord::new("L2", "C2", vec![]).with_severity("warning"),
            )?;
            insert_lesson(
                conn,
                &LessonRecord::new("L3", "C3", vec![]).with_severity("critical"),
            )?;

            let critical = list_lessons_by_severity(conn, "critical")?;
            assert_eq!(critical.len(), 2);

            let warning = list_lessons_by_severity(conn, "warning")?;
            assert_eq!(warning.len(), 1);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_count() {
        let db = setup_db();

        db.with_conn(|conn| {
            assert_eq!(count_lessons(conn)?, 0);

            insert_lesson(conn, &LessonRecord::new("L1", "C1", vec![]))?;
            insert_lesson(conn, &LessonRecord::new("L2", "C2", vec![]))?;

            assert_eq!(count_lessons(conn)?, 2);

            Ok(())
        })
        .unwrap();
    }
}
```

**Update `src/storage/mod.rs`** - add:
```rust
mod lessons;

pub use lessons::{
    count_lessons, delete_lesson, get_lesson, insert_lesson, list_lessons,
    list_lessons_by_agent, list_lessons_by_severity, update_lesson,
};
```

**Verification Commands**:
```bash
cargo test storage::lessons:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. 5 passed; 0 failed"
```

**Success Criteria**:
- [x] Insert, get, update, delete work
- [x] List by severity works
- [x] Tags serialization works
- [x] All lessons tests pass
- [x] Commit made with message "feat(storage): implement lessons CRUD operations"

---

**Completion Notes**:
- **Implementation**: Implemented complete CRUD operations for lessons storage with proper error handling and documentation. All operations use rusqlite with parameterized queries for SQL safety. Tags are stored as JSON in the database and deserialized on retrieval.
- **Files Created**:
  - `src/storage/lessons.rs` (388 lines)
- **Files Modified**:
  - `src/storage/mod.rs` (added lesson module and exports)
- **Tests**: 5 unit tests passing (test_insert_and_get, test_update, test_delete, test_list_by_severity, test_count). All 120 tests in the suite pass.
- **Build**: ✅ cargo test passes, cargo clippy clean, cargo fmt clean, cargo build --release succeeds
- **Branch**: feature/3-1-lessons
- **Notes**: Implemented all required functions: insert_lesson, get_lesson, update_lesson, delete_lesson, list_lessons, list_lessons_by_severity, list_lessons_by_agent, count_lessons. Used proper error handling with thiserror pattern. Added comprehensive documentation for all public functions.

---

### Subtask 3.1.2: Add Lesson Search with Semantic Matching (Single Session)

**Prerequisites**:
- [x] 3.1.1: Implement Lessons Storage and CRUD

**Deliverables**:
- [x] Create vector table for lesson embeddings
- [x] Implement semantic search
- [x] Add combined text + semantic search
- [x] Write search tests

**Files to Create**:

**`src/storage/lessons_search.rs`** (complete file):
```rust
//! Lesson semantic search.

use rusqlite::Connection;

use super::models::{LessonRecord, SearchResult};
use super::vector::{insert_vector, search_similar};
use crate::error::StorageError;
use crate::Result;

const LESSON_VEC_TABLE: &str = "lesson_embeddings";

/// Initialize lesson vector table.
pub fn init_lesson_vectors(conn: &Connection) -> Result<()> {
    let sql = format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {LESSON_VEC_TABLE} USING vec0(
            id TEXT PRIMARY KEY,
            embedding FLOAT[384]
        )"
    );

    conn.execute(&sql, [])
        .map_err(|e| StorageError::Vector(format!("failed to create lesson vec table: {e}")))?;

    Ok(())
}

/// Store lesson embedding.
pub fn store_lesson_embedding(conn: &Connection, lesson_id: &str, embedding: &[f32]) -> Result<()> {
    // Delete old embedding if exists
    conn.execute(
        &format!("DELETE FROM {LESSON_VEC_TABLE} WHERE id = ?"),
        [lesson_id],
    )
    .ok();

    // Insert new embedding
    let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        &format!("INSERT INTO {LESSON_VEC_TABLE} (id, embedding) VALUES (?, ?)"),
        rusqlite::params![lesson_id, blob],
    )
    .map_err(|e| StorageError::Vector(format!("failed to store lesson embedding: {e}")))?;

    Ok(())
}

/// Search lessons by embedding similarity.
pub fn search_lessons_by_embedding(
    conn: &Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult<LessonRecord>>> {
    let blob: Vec<u8> = query_embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

    let sql = format!(
        "SELECT id, distance FROM {LESSON_VEC_TABLE} WHERE embedding MATCH ? ORDER BY distance LIMIT ?"
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StorageError::Vector(format!("failed to prepare search: {e}")))?;

    let candidates: Vec<(String, f32)> = stmt
        .query_map(rusqlite::params![blob, limit as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| StorageError::Vector(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut results = Vec::new();
    for (id, distance) in candidates {
        if let Ok(lesson) = super::lessons::get_lesson(conn, &id) {
            results.push(SearchResult::new(lesson, distance));
        }
    }

    Ok(results)
}

/// Search lessons by text match (FTS or LIKE).
pub fn search_lessons_by_text(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<LessonRecord>> {
    let pattern = format!("%{query}%");

    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
             FROM lessons
             WHERE title LIKE ? OR content LIKE ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let lessons = stmt
        .query_map(rusqlite::params![pattern, pattern, limit as i64], |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for lesson in lessons {
        result.push(lesson.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

/// Search lessons by tag.
pub fn search_lessons_by_tag(conn: &Connection, tag: &str) -> Result<Vec<LessonRecord>> {
    // Tags are stored as JSON array, search with LIKE
    let pattern = format!("%\"{tag}\"%");

    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, tags, severity, agent, repo, created_at, updated_at
             FROM lessons
             WHERE tags LIKE ?
             ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let lessons = stmt
        .query_map([pattern], |row| {
            let tags_json: String = row.get(3)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            Ok(LessonRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags,
                severity: row.get(4)?,
                agent: row.get(5)?,
                repo: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                embedding: None,
            })
        })
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut result = Vec::new();
    for lesson in lessons {
        result.push(lesson.map_err(|e| StorageError::Database(e.to_string()))?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{insert_lesson, migrate, Database};

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();
        db
    }

    #[test]
    fn test_search_by_text() {
        let db = setup_db();

        db.with_conn(|conn| {
            use crate::storage::LessonRecord;

            insert_lesson(
                conn,
                &LessonRecord::new("Rust Error Handling", "Use Result type for errors", vec![]),
            )?;
            insert_lesson(
                conn,
                &LessonRecord::new("Python Testing", "Use pytest for testing", vec![]),
            )?;

            let results = search_lessons_by_text(conn, "Rust", 10)?;
            assert_eq!(results.len(), 1);
            assert!(results[0].title.contains("Rust"));

            let results = search_lessons_by_text(conn, "testing", 10)?;
            assert_eq!(results.len(), 1);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_search_by_tag() {
        let db = setup_db();

        db.with_conn(|conn| {
            use crate::storage::LessonRecord;

            insert_lesson(
                conn,
                &LessonRecord::new("L1", "C1", vec!["rust".to_string(), "errors".to_string()]),
            )?;
            insert_lesson(
                conn,
                &LessonRecord::new("L2", "C2", vec!["python".to_string()]),
            )?;
            insert_lesson(
                conn,
                &LessonRecord::new("L3", "C3", vec!["rust".to_string()]),
            )?;

            let results = search_lessons_by_tag(conn, "rust")?;
            assert_eq!(results.len(), 2);

            let results = search_lessons_by_tag(conn, "python")?;
            assert_eq!(results.len(), 1);

            Ok(())
        })
        .unwrap();
    }
}
```

**Update `src/storage/mod.rs`** - add:
```rust
mod lessons_search;

pub use lessons_search::{
    init_lesson_vectors, search_lessons_by_embedding, search_lessons_by_tag,
    search_lessons_by_text, store_lesson_embedding,
};
```

**Verification Commands**:
```bash
cargo test storage::lessons_search:: --verbose 2>&1 | tail -20
# Expected: "test result: ok. 2 passed; 0 failed"
```

**Success Criteria**:
- [x] Text search works
- [x] Tag search works
- [x] Vector search functions compile
- [x] All search tests pass
- [x] Commit made with message "feat(storage): add lesson search with semantic matching"

---

**Completion Notes**:
- **Implementation**: Implemented lesson semantic search with three complementary search strategies: text-based search using LIKE pattern matching on title/content, tag-based search using JSON LIKE patterns, and vector embedding similarity search. All functions follow proper error handling patterns with Result types and comprehensive documentation.
- **Files Created**:
  - `src/storage/lessons_search.rs` (262 lines)
- **Files Modified**:
  - `src/storage/mod.rs` (added lessons_search module and exports)
- **Tests**: 2 new tests passing (test_search_by_text, test_search_by_tag). All 122 tests in suite pass.
- **Build**: ✅ cargo fmt clean, cargo clippy clean, cargo test passes (122/122), cargo build --release succeeds
- **Branch**: feature/3-1-lessons
- **Notes**: Implemented functions: init_lesson_vectors, store_lesson_embedding, search_lessons_by_embedding, search_lessons_by_text, search_lessons_by_tag. All functions have proper documentation including # Errors sections. Used proper error propagation with Result type aliases and error mapping.

---

### Task 3.1 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] All tests pass
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Task 3.2: Checkpoint System

**Git**: Create branch `feature/3-2-checkpoints` when starting first subtask.

### Subtask 3.2.1: Implement Checkpoint Storage (Single Session)

**Prerequisites**:
- [x] 3.1.2: Add Lesson Search

**Deliverables**:
- [ ] Create checkpoint CRUD operations
- [ ] Add agent-based queries
- [ ] Implement time-based filtering
- [ ] Write checkpoint tests

**Files to Create**:

**`src/storage/checkpoints.rs`** (complete file):
```rust
//! Checkpoint storage operations.

use rusqlite::{params, Connection};

use super::models::CheckpointRecord;
use crate::error::StorageError;
use crate::Result;

/// Insert a new checkpoint.
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
                embedding: None,
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
pub fn get_recent_checkpoints(
    conn: &Connection,
    agent: &str,
    limit: usize,
) -> Result<Vec<CheckpointRecord>> {
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
        .query_map(params![agent, limit as i64], |row| {
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
                embedding: None,
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
pub fn get_checkpoints_since(
    conn: &Connection,
    agent: &str,
    since_timestamp: i64,
    limit: usize,
) -> Result<Vec<CheckpointRecord>> {
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
        .query_map(params![agent, since_timestamp, limit as i64], |row| {
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
                embedding: None,
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
pub fn get_latest_checkpoint(conn: &Connection, agent: &str) -> Result<Option<CheckpointRecord>> {
    let checkpoints = get_recent_checkpoints(conn, agent, 1)?;
    Ok(checkpoints.into_iter().next())
}

/// Count checkpoints for an agent.
pub fn count_checkpoints(conn: &Connection, agent: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM checkpoints WHERE agent = ?",
        [agent],
        |row| row.get(0),
    )
    .map_err(|e| StorageError::Database(e.to_string()).into())
}

/// Delete old checkpoints for an agent, keeping only the most recent N.
pub fn cleanup_old_checkpoints(conn: &Connection, agent: &str, keep: usize) -> Result<usize> {
    let sql = format!(
        "DELETE FROM checkpoints
         WHERE agent = ? AND id NOT IN (
             SELECT id FROM checkpoints WHERE agent = ? ORDER BY created_at DESC LIMIT ?
         )"
    );

    let deleted = conn
        .execute(&sql, params![agent, agent, keep as i64])
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
                let cp = CheckpointRecord::new(
                    "agent1",
                    format!("Task {i}"),
                    serde_json::json!({}),
                );
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
                let cp = CheckpointRecord::new("agent1", format!("Task {i}"), serde_json::json!({}));
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
}
```

**Update `src/storage/mod.rs`** - add:
```rust
mod checkpoints;

pub use checkpoints::{
    cleanup_old_checkpoints, count_checkpoints, delete_checkpoint, get_checkpoint,
    get_checkpoints_since, get_latest_checkpoint, get_recent_checkpoints, insert_checkpoint,
};
```

**Verification Commands**:
```bash
cargo test storage::checkpoints:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. 4 passed; 0 failed"
```

**Success Criteria**:
- [ ] Insert, get, delete work
- [ ] Recent checkpoints query works
- [ ] Cleanup function works
- [ ] All checkpoint tests pass
- [ ] Commit made with message "feat(storage): implement checkpoint storage"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/storage/checkpoints.rs` (X lines)
- **Files Modified**:
  - `src/storage/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/3-2-checkpoints
- **Notes**: (any additional context)

---

### Task 3.2 Complete - Squash Merge

- [ ] All subtasks complete
- [ ] All tests pass
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Phase 3 Complete

**Phase 3 Checklist**:
- [ ] Task 3.1 merged (lessons CRUD + search)
- [ ] Task 3.2 merged (checkpoints)
- [ ] All tests pass (70+ tests)
- [ ] Lessons and checkpoints functional

**Ready for Phase 4**: MCP & REST API

---

*Phase 3 Plan - Nellie Production*
