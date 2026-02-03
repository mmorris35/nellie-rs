# Phase 2: File Watcher & Indexing

**Goal**: Implement file watching and incremental code indexing
**Duration**: 1 week
**Prerequisites**: Phase 1 complete

---

## Task 2.1: File Watcher Setup

**Git**: Create branch `feature/2-1-file-watcher` when starting first subtask.

### Subtask 2.1.1: Set Up notify-rs File Watcher (Single Session)

**Prerequisites**:
- [x] 1.3.3: Create Async Embedding API

**Deliverables**:
- [ ] Create file watcher wrapper using notify crate
- [ ] Implement debounced event handling
- [ ] Add directory watching with recursion
- [ ] Write watcher tests

**Files to Create**:

**`src/watcher/events.rs`** (complete file):
```rust
//! File system event types and handling.

use std::path::PathBuf;

/// File system event types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// File was created or modified.
    Modified(PathBuf),
    /// File was deleted.
    Deleted(PathBuf),
    /// File was renamed from old path to new path.
    Renamed { from: PathBuf, to: PathBuf },
}

impl FileEvent {
    /// Get the primary path associated with this event.
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Modified(p) | Self::Deleted(p) => p,
            Self::Renamed { to, .. } => to,
        }
    }

    /// Check if this event affects a file (vs directory).
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.path().is_file() || !self.path().exists()
    }
}

/// Batch of file events for processing.
#[derive(Debug, Default)]
pub struct EventBatch {
    /// Modified files (need re-indexing).
    pub modified: Vec<PathBuf>,
    /// Deleted files (need removal from index).
    pub deleted: Vec<PathBuf>,
}

impl EventBatch {
    /// Create a new empty batch.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the batch.
    pub fn add(&mut self, event: FileEvent) {
        match event {
            FileEvent::Modified(path) => {
                if !self.modified.contains(&path) {
                    self.modified.push(path);
                }
            }
            FileEvent::Deleted(path) => {
                // Remove from modified if present
                self.modified.retain(|p| p != &path);
                if !self.deleted.contains(&path) {
                    self.deleted.push(path);
                }
            }
            FileEvent::Renamed { from, to } => {
                // Treat as delete + create
                self.modified.retain(|p| p != &from);
                if !self.deleted.contains(&from) {
                    self.deleted.push(from);
                }
                if !self.modified.contains(&to) {
                    self.modified.push(to);
                }
            }
        }
    }

    /// Check if batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Get total number of events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.modified.len() + self.deleted.len()
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.modified.clear();
        self.deleted.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_path() {
        let modified = FileEvent::Modified(PathBuf::from("/test/file.rs"));
        assert_eq!(modified.path(), &PathBuf::from("/test/file.rs"));

        let deleted = FileEvent::Deleted(PathBuf::from("/test/removed.rs"));
        assert_eq!(deleted.path(), &PathBuf::from("/test/removed.rs"));

        let renamed = FileEvent::Renamed {
            from: PathBuf::from("/old.rs"),
            to: PathBuf::from("/new.rs"),
        };
        assert_eq!(renamed.path(), &PathBuf::from("/new.rs"));
    }

    #[test]
    fn test_event_batch_add_modified() {
        let mut batch = EventBatch::new();
        batch.add(FileEvent::Modified(PathBuf::from("/a.rs")));
        batch.add(FileEvent::Modified(PathBuf::from("/b.rs")));
        batch.add(FileEvent::Modified(PathBuf::from("/a.rs"))); // Duplicate

        assert_eq!(batch.modified.len(), 2);
        assert!(batch.deleted.is_empty());
    }

    #[test]
    fn test_event_batch_add_deleted() {
        let mut batch = EventBatch::new();
        batch.add(FileEvent::Modified(PathBuf::from("/a.rs")));
        batch.add(FileEvent::Deleted(PathBuf::from("/a.rs")));

        assert!(batch.modified.is_empty());
        assert_eq!(batch.deleted.len(), 1);
    }

    #[test]
    fn test_event_batch_renamed() {
        let mut batch = EventBatch::new();
        batch.add(FileEvent::Renamed {
            from: PathBuf::from("/old.rs"),
            to: PathBuf::from("/new.rs"),
        });

        assert_eq!(batch.modified, vec![PathBuf::from("/new.rs")]);
        assert_eq!(batch.deleted, vec![PathBuf::from("/old.rs")]);
    }

    #[test]
    fn test_event_batch_len_and_empty() {
        let mut batch = EventBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);

        batch.add(FileEvent::Modified(PathBuf::from("/a.rs")));
        batch.add(FileEvent::Deleted(PathBuf::from("/b.rs")));

        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 2);
    }
}
```

**`src/watcher/watcher.rs`** (complete file):
```rust
//! File system watcher using notify-rs.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use parking_lot::Mutex;
use tokio::sync::mpsc;

use super::events::{EventBatch, FileEvent};
use crate::error::WatcherError;
use crate::Result;

/// Debounce duration for file events.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

/// File watcher configuration.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Directories to watch.
    pub watch_dirs: Vec<PathBuf>,
    /// Debounce duration.
    pub debounce: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            watch_dirs: Vec::new(),
            debounce: DEBOUNCE_DURATION,
        }
    }
}

/// File system watcher.
pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    event_rx: mpsc::Receiver<EventBatch>,
    watched_dirs: Arc<Mutex<Vec<PathBuf>>>,
}

impl FileWatcher {
    /// Create a new file watcher.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be created.
    pub fn new(config: WatcherConfig) -> Result<Self> {
        let (batch_tx, event_rx) = mpsc::channel(100);
        let watched_dirs = Arc::new(Mutex::new(Vec::new()));
        let watched_dirs_clone = Arc::clone(&watched_dirs);

        let debouncer = new_debouncer(config.debounce, move |result| {
            match result {
                Ok(events) => {
                    let mut batch = EventBatch::new();
                    for event in events {
                        let file_event = match event.kind {
                            DebouncedEventKind::Any => FileEvent::Modified(event.path),
                            DebouncedEventKind::AnyContinuous => continue, // Skip continuous events
                        };
                        batch.add(file_event);
                    }

                    if !batch.is_empty() {
                        // Filter to watched directories
                        let dirs = watched_dirs_clone.lock();
                        batch.modified.retain(|p| is_under_watched(&dirs, p));
                        batch.deleted.retain(|p| is_under_watched(&dirs, p));

                        if !batch.is_empty() {
                            let _ = batch_tx.blocking_send(batch);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Watch error: {:?}", e);
                }
            }
        })
        .map_err(|e| WatcherError::WatchFailed {
            path: "init".to_string(),
            reason: e.to_string(),
        })?;

        let mut watcher = Self {
            _debouncer: debouncer,
            event_rx,
            watched_dirs,
        };

        // Add initial watch directories
        for dir in &config.watch_dirs {
            watcher.watch(dir)?;
        }

        Ok(watcher)
    }

    /// Add a directory to watch.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be watched.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(WatcherError::WatchFailed {
                path: path.display().to_string(),
                reason: "directory does not exist".to_string(),
            }
            .into());
        }

        self._debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|e| WatcherError::WatchFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

        self.watched_dirs.lock().push(path.clone());
        tracing::info!(path = %path.display(), "Watching directory");

        Ok(())
    }

    /// Stop watching a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if unwatching fails.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        self._debouncer
            .watcher()
            .unwatch(path)
            .map_err(|e| WatcherError::WatchFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;

        self.watched_dirs
            .lock()
            .retain(|p| p != path);

        tracing::info!(path = %path.display(), "Stopped watching directory");
        Ok(())
    }

    /// Receive the next batch of events.
    ///
    /// Returns `None` if the watcher has been dropped.
    pub async fn recv(&mut self) -> Option<EventBatch> {
        self.event_rx.recv().await
    }

    /// Get list of watched directories.
    #[must_use]
    pub fn watched_dirs(&self) -> Vec<PathBuf> {
        self.watched_dirs.lock().clone()
    }
}

/// Check if a path is under any watched directory.
fn is_under_watched(watched: &[PathBuf], path: &Path) -> bool {
    watched.iter().any(|dir| path.starts_with(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert!(config.watch_dirs.is_empty());
        assert_eq!(config.debounce, DEBOUNCE_DURATION);
    }

    #[test]
    fn test_is_under_watched() {
        let watched = vec![
            PathBuf::from("/home/user/project"),
            PathBuf::from("/var/data"),
        ];

        assert!(is_under_watched(&watched, Path::new("/home/user/project/src/main.rs")));
        assert!(is_under_watched(&watched, Path::new("/var/data/file.txt")));
        assert!(!is_under_watched(&watched, Path::new("/tmp/other.txt")));
    }

    #[test]
    fn test_watcher_nonexistent_dir() {
        let config = WatcherConfig::default();
        let mut watcher = FileWatcher::new(config).unwrap();

        let result = watcher.watch("/nonexistent/directory");
        assert!(result.is_err());
    }

    #[test]
    fn test_watcher_watch_and_unwatch() {
        let tmp = TempDir::new().unwrap();
        let config = WatcherConfig::default();
        let mut watcher = FileWatcher::new(config).unwrap();

        watcher.watch(tmp.path()).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);

        watcher.unwatch(tmp.path()).unwrap();
        assert!(watcher.watched_dirs().is_empty());
    }
}
```

**Update `src/watcher/mod.rs`** (replace - complete file):
```rust
//! File system watching and indexing.
//!
//! This module provides:
//! - Directory watching using notify-rs
//! - Gitignore-aware file filtering
//! - Incremental indexing of changed files

mod events;
mod watcher;

pub use events::{EventBatch, FileEvent};
pub use watcher::{FileWatcher, WatcherConfig};

/// Initialize watcher module.
pub fn init() {
    tracing::debug!("Watcher module initialized");
}
```

**Verification Commands**:
```bash
# Verify compilation
cargo check 2>&1 | tail -10
# Expected: "Finished `dev` profile..."

# Run watcher tests
cargo test watcher:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. X passed; 0 failed"
```

**Success Criteria**:
- [ ] Event types defined and working
- [ ] EventBatch deduplication works
- [ ] FileWatcher can watch directories
- [ ] All watcher tests pass
- [ ] Commit made with message "feat(watcher): implement file system watcher"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/watcher/events.rs` (X lines)
  - `src/watcher/watcher.rs` (X lines)
- **Files Modified**:
  - `src/watcher/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-1-file-watcher
- **Notes**: (any additional context)

---

### Subtask 2.1.2: Implement Gitignore-Aware Filtering (Single Session)

**Prerequisites**:
- [x] 2.1.1: Set Up notify-rs File Watcher

**Deliverables**:
- [ ] Create file filter using ignore crate
- [ ] Support .gitignore patterns
- [ ] Add language detection
- [ ] Write filter tests

**Files to Create**:

**`src/watcher/filter.rs`** (complete file):
```rust
//! File filtering with gitignore support.

use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::Result;

/// Supported code file extensions and their languages.
const CODE_EXTENSIONS: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("py", "python"),
    ("js", "javascript"),
    ("ts", "typescript"),
    ("jsx", "javascript"),
    ("tsx", "typescript"),
    ("go", "go"),
    ("java", "java"),
    ("c", "c"),
    ("cpp", "cpp"),
    ("cc", "cpp"),
    ("h", "c"),
    ("hpp", "cpp"),
    ("cs", "csharp"),
    ("rb", "ruby"),
    ("php", "php"),
    ("swift", "swift"),
    ("kt", "kotlin"),
    ("scala", "scala"),
    ("sh", "shell"),
    ("bash", "shell"),
    ("zsh", "shell"),
    ("sql", "sql"),
    ("md", "markdown"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("json", "json"),
    ("toml", "toml"),
    ("xml", "xml"),
    ("html", "html"),
    ("css", "css"),
    ("scss", "scss"),
    ("vue", "vue"),
    ("svelte", "svelte"),
];

/// File filter for indexing.
#[derive(Debug)]
pub struct FileFilter {
    gitignore: Option<Gitignore>,
    base_path: std::path::PathBuf,
}

impl FileFilter {
    /// Create a new file filter.
    ///
    /// If a .gitignore exists in base_path, it will be used for filtering.
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base_path = base_path.as_ref().to_path_buf();
        let gitignore_path = base_path.join(".gitignore");

        let gitignore = if gitignore_path.exists() {
            let mut builder = GitignoreBuilder::new(&base_path);
            if builder.add(&gitignore_path).is_none() {
                builder.build().ok()
            } else {
                None
            }
        } else {
            None
        };

        Self {
            gitignore,
            base_path,
        }
    }

    /// Create a filter with custom ignore patterns.
    ///
    /// # Errors
    ///
    /// Returns an error if patterns are invalid.
    pub fn with_patterns(base_path: impl AsRef<Path>, patterns: &[&str]) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let mut builder = GitignoreBuilder::new(&base_path);

        for pattern in patterns {
            builder
                .add_line(None, pattern)
                .map_err(|e| crate::Error::config(format!("invalid pattern: {e}")))?;
        }

        let gitignore = builder
            .build()
            .map_err(|e| crate::Error::config(format!("failed to build gitignore: {e}")))?;

        Ok(Self {
            gitignore: Some(gitignore),
            base_path,
        })
    }

    /// Check if a file should be indexed.
    #[must_use]
    pub fn should_index(&self, path: &Path) -> bool {
        // Must be a file
        if !path.is_file() {
            return false;
        }

        // Must be a code file
        if !Self::is_code_file(path) {
            return false;
        }

        // Must not be ignored
        if let Some(ref gi) = self.gitignore {
            if gi.matched(path, false).is_ignore() {
                return false;
            }
        }

        // Default ignores
        if Self::is_default_ignored(path) {
            return false;
        }

        true
    }

    /// Check if a path is a code file based on extension.
    #[must_use]
    pub fn is_code_file(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| CODE_EXTENSIONS.iter().any(|(e, _)| *e == ext.to_lowercase()))
            .unwrap_or(false)
    }

    /// Get the language for a file based on extension.
    #[must_use]
    pub fn detect_language(path: &Path) -> Option<&'static str> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(|ext| {
                CODE_EXTENSIONS
                    .iter()
                    .find(|(e, _)| *e == ext.to_lowercase())
                    .map(|(_, lang)| *lang)
            })
    }

    /// Check if a path matches default ignore patterns.
    fn is_default_ignored(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Common directories to ignore
        let ignored_dirs = [
            "/node_modules/",
            "/.git/",
            "/target/",
            "/build/",
            "/dist/",
            "/__pycache__/",
            "/.venv/",
            "/venv/",
            "/.idea/",
            "/.vscode/",
            "/vendor/",
        ];

        for dir in ignored_dirs {
            if path_str.contains(dir) {
                return true;
            }
        }

        // Common files to ignore
        let ignored_files = [
            ".DS_Store",
            "Thumbs.db",
            ".env",
            ".env.local",
        ];

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if ignored_files.contains(&name) {
                return true;
            }

            // Ignore hidden files (starting with .)
            if name.starts_with('.') && name != ".gitignore" {
                return true;
            }

            // Ignore lock files
            if name.ends_with(".lock") || name.ends_with("-lock.json") {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_is_code_file() {
        assert!(FileFilter::is_code_file(Path::new("main.rs")));
        assert!(FileFilter::is_code_file(Path::new("app.py")));
        assert!(FileFilter::is_code_file(Path::new("index.tsx")));
        assert!(!FileFilter::is_code_file(Path::new("image.png")));
        assert!(!FileFilter::is_code_file(Path::new("document.pdf")));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(FileFilter::detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(FileFilter::detect_language(Path::new("app.py")), Some("python"));
        assert_eq!(FileFilter::detect_language(Path::new("index.tsx")), Some("typescript"));
        assert_eq!(FileFilter::detect_language(Path::new("unknown.xyz")), None);
    }

    #[test]
    fn test_default_ignored() {
        assert!(FileFilter::is_default_ignored(Path::new("/project/node_modules/pkg/index.js")));
        assert!(FileFilter::is_default_ignored(Path::new("/project/.git/config")));
        assert!(FileFilter::is_default_ignored(Path::new("/project/target/debug/main")));
        assert!(FileFilter::is_default_ignored(Path::new("/project/.env")));
        assert!(!FileFilter::is_default_ignored(Path::new("/project/src/main.rs")));
    }

    #[test]
    fn test_filter_with_gitignore() {
        let tmp = TempDir::new().unwrap();

        // Create .gitignore
        fs::write(tmp.path().join(".gitignore"), "*.log\ntest_output/\n").unwrap();

        // Create test files
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("debug.log"), "log content").unwrap();

        let filter = FileFilter::new(tmp.path());

        assert!(filter.should_index(&tmp.path().join("main.rs")));
        assert!(!filter.should_index(&tmp.path().join("debug.log")));
    }

    #[test]
    fn test_filter_with_patterns() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("test.rs"), "fn test() {}").unwrap();

        let filter = FileFilter::with_patterns(tmp.path(), &["test*.rs"]).unwrap();

        assert!(filter.should_index(&tmp.path().join("main.rs")));
        assert!(!filter.should_index(&tmp.path().join("test.rs")));
    }
}
```

**Update `src/watcher/mod.rs`** - add after watcher:
```rust
mod filter;

pub use filter::FileFilter;
```

**Verification Commands**:
```bash
cargo test watcher::filter:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. 5 passed; 0 failed"
```

**Success Criteria**:
- [ ] File extension detection works
- [ ] Language detection works
- [ ] Gitignore patterns respected
- [ ] Default ignores work
- [ ] All filter tests pass
- [ ] Commit made with message "feat(watcher): add gitignore-aware file filtering"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/watcher/filter.rs` (X lines)
- **Files Modified**:
  - `src/watcher/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-1-file-watcher
- **Notes**: (any additional context)

---

### Subtask 2.1.3: Create File Change Event Handler (Single Session)

**Prerequisites**:
- [x] 2.1.2: Implement Gitignore-Aware Filtering

**Deliverables**:
- [ ] Create event handler that processes batches
- [ ] Integrate filter with watcher
- [ ] Add event statistics tracking
- [ ] Write handler tests

**Files to Create**:

**`src/watcher/handler.rs`** (complete file):
```rust
//! File change event handler.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use super::events::EventBatch;
use super::filter::FileFilter;
use crate::Result;

/// Statistics for file watching.
#[derive(Debug, Default)]
pub struct WatcherStats {
    pub files_detected: AtomicU64,
    pub files_filtered: AtomicU64,
    pub files_indexed: AtomicU64,
    pub files_deleted: AtomicU64,
    pub errors: AtomicU64,
}

impl WatcherStats {
    /// Create new stats tracker.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get snapshot of current stats.
    #[must_use]
    pub fn snapshot(&self) -> WatcherStatsSnapshot {
        WatcherStatsSnapshot {
            files_detected: self.files_detected.load(Ordering::Relaxed),
            files_filtered: self.files_filtered.load(Ordering::Relaxed),
            files_indexed: self.files_indexed.load(Ordering::Relaxed),
            files_deleted: self.files_deleted.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of watcher stats.
#[derive(Debug, Clone, Copy)]
pub struct WatcherStatsSnapshot {
    pub files_detected: u64,
    pub files_filtered: u64,
    pub files_indexed: u64,
    pub files_deleted: u64,
    pub errors: u64,
}

/// Processed file event for indexing.
#[derive(Debug, Clone)]
pub struct IndexRequest {
    /// File path.
    pub path: PathBuf,
    /// Detected language.
    pub language: Option<String>,
}

/// Event handler configuration.
#[derive(Debug, Clone)]
pub struct HandlerConfig {
    /// Base path for filtering.
    pub base_path: PathBuf,
    /// Custom ignore patterns.
    pub ignore_patterns: Vec<String>,
}

/// Event handler that filters and processes file changes.
pub struct EventHandler {
    filter: FileFilter,
    stats: Arc<WatcherStats>,
    index_tx: mpsc::Sender<IndexRequest>,
    delete_tx: mpsc::Sender<PathBuf>,
}

impl EventHandler {
    /// Create a new event handler.
    ///
    /// # Errors
    ///
    /// Returns an error if the filter cannot be created.
    pub fn new(
        config: HandlerConfig,
        stats: Arc<WatcherStats>,
        index_tx: mpsc::Sender<IndexRequest>,
        delete_tx: mpsc::Sender<PathBuf>,
    ) -> Result<Self> {
        let patterns: Vec<&str> = config.ignore_patterns.iter().map(String::as_str).collect();
        let filter = if patterns.is_empty() {
            FileFilter::new(&config.base_path)
        } else {
            FileFilter::with_patterns(&config.base_path, &patterns)?
        };

        Ok(Self {
            filter,
            stats,
            index_tx,
            delete_tx,
        })
    }

    /// Process a batch of file events.
    pub async fn process_batch(&self, batch: EventBatch) {
        let total = batch.len();
        self.stats
            .files_detected
            .fetch_add(total as u64, Ordering::Relaxed);

        // Process modified files
        for path in batch.modified {
            if self.filter.should_index(&path) {
                let language = FileFilter::detect_language(&path).map(String::from);
                let request = IndexRequest {
                    path: path.clone(),
                    language,
                };

                if self.index_tx.send(request).await.is_ok() {
                    self.stats.files_indexed.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.stats.errors.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                self.stats.files_filtered.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Process deleted files
        for path in batch.deleted {
            if self.delete_tx.send(path).await.is_ok() {
                self.stats.files_deleted.fetch_add(1, Ordering::Relaxed);
            } else {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        let snapshot = self.stats.snapshot();
        tracing::debug!(
            detected = snapshot.files_detected,
            indexed = snapshot.files_indexed,
            filtered = snapshot.files_filtered,
            deleted = snapshot.files_deleted,
            "Processed event batch"
        );
    }

    /// Get current stats.
    #[must_use]
    pub fn stats(&self) -> Arc<WatcherStats> {
        Arc::clone(&self.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_handler_filters_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let stats = WatcherStats::new();
        let (index_tx, mut index_rx) = mpsc::channel(10);
        let (delete_tx, _delete_rx) = mpsc::channel(10);

        let config = HandlerConfig {
            base_path: tmp.path().to_path_buf(),
            ignore_patterns: vec![],
        };

        let handler = EventHandler::new(config, stats.clone(), index_tx, delete_tx).unwrap();

        let mut batch = EventBatch::new();
        batch.modified.push(tmp.path().join("main.rs"));
        batch.modified.push(tmp.path().join("image.png")); // Should be filtered

        handler.process_batch(batch).await;

        // Should only receive the .rs file
        let request = index_rx.recv().await.unwrap();
        assert!(request.path.ends_with("main.rs"));
        assert_eq!(request.language, Some("rust".to_string()));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.files_detected, 2);
        assert_eq!(snapshot.files_indexed, 1);
        assert_eq!(snapshot.files_filtered, 1);
    }

    #[tokio::test]
    async fn test_handler_processes_deletes() {
        let tmp = TempDir::new().unwrap();

        let stats = WatcherStats::new();
        let (index_tx, _index_rx) = mpsc::channel(10);
        let (delete_tx, mut delete_rx) = mpsc::channel(10);

        let config = HandlerConfig {
            base_path: tmp.path().to_path_buf(),
            ignore_patterns: vec![],
        };

        let handler = EventHandler::new(config, stats.clone(), index_tx, delete_tx).unwrap();

        let mut batch = EventBatch::new();
        batch.deleted.push(tmp.path().join("deleted.rs"));

        handler.process_batch(batch).await;

        let path = delete_rx.recv().await.unwrap();
        assert!(path.ends_with("deleted.rs"));

        assert_eq!(stats.snapshot().files_deleted, 1);
    }
}
```

**Update `src/watcher/mod.rs`** - add after filter:
```rust
mod handler;

pub use handler::{EventHandler, HandlerConfig, IndexRequest, WatcherStats, WatcherStatsSnapshot};
```

**Verification Commands**:
```bash
cargo test watcher:: --verbose 2>&1 | grep "test result"
# Expected: "test result: ok"
```

**Success Criteria**:
- [ ] Event handler processes batches
- [ ] Filtering integrated correctly
- [ ] Stats tracking works
- [ ] All handler tests pass
- [ ] Commit made with message "feat(watcher): create event handler with stats tracking"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/watcher/handler.rs` (X lines)
- **Files Modified**:
  - `src/watcher/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-1-file-watcher
- **Notes**: (any additional context)

---

### Task 2.1 Complete - Squash Merge

- [ ] All subtasks complete (2.1.1 - 2.1.3)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test watcher::` passes
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Task 2.2: Indexing Pipeline

**Git**: Create branch `feature/2-2-indexing` when starting first subtask.

### Subtask 2.2.1: Implement Code Chunking Strategy (Single Session)

**Prerequisites**:
- [x] 2.1.3: Create File Change Event Handler

**Deliverables**:
- [ ] Create chunking algorithm
- [ ] Handle different languages
- [ ] Respect function/class boundaries where possible
- [ ] Write chunking tests

**Files to Create**:

**`src/watcher/chunker.rs`** (complete file):
```rust
//! Code chunking for indexing.

use std::path::Path;

/// Chunk of code from a file.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// Starting line (1-based).
    pub start_line: usize,
    /// Ending line (1-based, inclusive).
    pub end_line: usize,
    /// Chunk content.
    pub content: String,
    /// Chunk index within file.
    pub index: usize,
}

/// Chunking configuration.
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Target chunk size in lines.
    pub target_lines: usize,
    /// Minimum chunk size in lines.
    pub min_lines: usize,
    /// Maximum chunk size in lines.
    pub max_lines: usize,
    /// Overlap between chunks in lines.
    pub overlap_lines: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            target_lines: 50,
            min_lines: 10,
            max_lines: 100,
            overlap_lines: 5,
        }
    }
}

/// Code chunker.
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    /// Create a new chunker with config.
    #[must_use]
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }

    /// Create a chunker with default config.
    #[must_use]
    pub fn default_chunker() -> Self {
        Self::new(ChunkerConfig::default())
    }

    /// Chunk file content into pieces.
    #[must_use]
    pub fn chunk_content(&self, content: &str, _language: Option<&str>) -> Vec<CodeChunk> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Vec::new();
        }

        // For small files, return as single chunk
        if lines.len() <= self.config.max_lines {
            return vec![CodeChunk {
                start_line: 1,
                end_line: lines.len(),
                content: content.to_string(),
                index: 0,
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut index = 0;

        while start < lines.len() {
            let end = self.find_chunk_end(&lines, start);
            let chunk_lines = &lines[start..end];

            chunks.push(CodeChunk {
                start_line: start + 1,
                end_line: end,
                content: chunk_lines.join("\n"),
                index,
            });

            index += 1;

            // Move start with overlap
            let next_start = if end >= lines.len() {
                lines.len()
            } else {
                (end - self.config.overlap_lines).max(start + 1)
            };

            if next_start <= start {
                break;
            }
            start = next_start;
        }

        chunks
    }

    /// Find a good end point for a chunk.
    fn find_chunk_end(&self, lines: &[&str], start: usize) -> usize {
        let ideal_end = (start + self.config.target_lines).min(lines.len());
        let max_end = (start + self.config.max_lines).min(lines.len());

        // Try to find a good break point
        for i in (ideal_end..=max_end).rev() {
            if self.is_good_break_point(lines, i) {
                return i;
            }
        }

        // Fall back to ideal end
        ideal_end
    }

    /// Check if a line is a good place to break.
    fn is_good_break_point(&self, lines: &[&str], pos: usize) -> bool {
        if pos >= lines.len() {
            return true;
        }

        let line = lines[pos].trim();

        // Empty lines are good breaks
        if line.is_empty() {
            return true;
        }

        // Lines starting with certain patterns are good breaks
        let good_starts = [
            "fn ", "pub fn ", "async fn ", "pub async fn ",
            "impl ", "pub struct ", "struct ", "enum ", "pub enum ",
            "trait ", "pub trait ", "mod ", "pub mod ",
            "def ", "class ", "async def ",
            "function ", "const ", "let ", "export ",
            "public ", "private ", "protected ",
            "#", "//", "/*", "///",
        ];

        good_starts.iter().any(|s| line.starts_with(s))
    }

    /// Chunk a file from path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn chunk_file(&self, path: &Path) -> std::io::Result<Vec<CodeChunk>> {
        let content = std::fs::read_to_string(path)?;
        let language = super::filter::FileFilter::detect_language(path);
        Ok(self.chunk_content(&content, language))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_small_file() {
        let chunker = Chunker::default_chunker();
        let content = "line 1\nline 2\nline 3";

        let chunks = chunker.chunk_content(content, Some("rust"));

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 3);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_large_file() {
        let chunker = Chunker::new(ChunkerConfig {
            target_lines: 10,
            min_lines: 5,
            max_lines: 15,
            overlap_lines: 2,
        });

        // Create 30 lines
        let content: String = (1..=30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");

        let chunks = chunker.chunk_content(&content, Some("rust"));

        assert!(chunks.len() > 1);
        // Check all content is covered
        assert_eq!(chunks[0].start_line, 1);
        assert!(chunks.last().unwrap().end_line >= 28);
    }

    #[test]
    fn test_chunk_empty_file() {
        let chunker = Chunker::default_chunker();
        let chunks = chunker.chunk_content("", None);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_good_break_points() {
        let chunker = Chunker::default_chunker();
        let lines = vec![
            "fn main() {",
            "    println!(\"hello\");",
            "}",
            "",
            "fn other() {",
        ];

        // Empty line is good break
        assert!(chunker.is_good_break_point(&lines, 3));
        // Function start is good break
        assert!(chunker.is_good_break_point(&lines, 4));
        // Middle of function is not good break
        assert!(!chunker.is_good_break_point(&lines, 1));
    }
}
```

**Update `src/watcher/mod.rs`** - add:
```rust
mod chunker;

pub use chunker::{Chunker, ChunkerConfig, CodeChunk};
```

**Verification Commands**:
```bash
cargo test watcher::chunker:: --verbose 2>&1 | tail -20
# Expected: "test result: ok. 4 passed; 0 failed"
```

**Success Criteria**:
- [ ] Small files chunked as single chunk
- [ ] Large files split appropriately
- [ ] Overlap between chunks works
- [ ] All chunker tests pass
- [ ] Commit made with message "feat(watcher): implement code chunking strategy"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/watcher/chunker.rs` (X lines)
- **Files Modified**:
  - `src/watcher/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-2-indexing
- **Notes**: (any additional context)

---

### Subtask 2.2.2: Build Incremental Indexing Pipeline (Single Session)

**Prerequisites**:
- [x] 2.2.1: Implement Code Chunking Strategy

**Deliverables**:
- [ ] Create indexer service that combines watcher + chunker + embeddings
- [ ] Process index requests asynchronously
- [ ] Handle file updates (re-index) and deletes
- [ ] Write integration tests

**Files to Create**:

**`src/watcher/indexer.rs`** (complete file):
```rust
//! Incremental indexing service.

use std::path::Path;
use std::sync::Arc;

use blake3::Hasher;
use tokio::sync::mpsc;

use super::chunker::Chunker;
use super::handler::IndexRequest;
use crate::embeddings::{placeholder_embedding, EmbeddingService};
use crate::storage::{
    delete_chunks_by_file, insert_chunk, ChunkRecord, Database, FileState,
};
use crate::Result;

/// Indexer service that processes files and stores chunks.
pub struct Indexer {
    db: Database,
    embeddings: Option<EmbeddingService>,
    chunker: Chunker,
}

impl Indexer {
    /// Create a new indexer.
    #[must_use]
    pub fn new(db: Database, embeddings: Option<EmbeddingService>) -> Self {
        Self {
            db,
            embeddings,
            chunker: Chunker::default_chunker(),
        }
    }

    /// Index a single file.
    ///
    /// # Errors
    ///
    /// Returns an error if indexing fails.
    pub async fn index_file(&self, request: &IndexRequest) -> Result<usize> {
        let path = &request.path;

        if !path.exists() {
            tracing::warn!(path = %path.display(), "File no longer exists");
            return Ok(0);
        }

        // Read file content
        let content = tokio::fs::read_to_string(path).await?;
        let file_hash = compute_hash(&content);

        // Check if already indexed with same hash
        if self.is_already_indexed(path, &file_hash)? {
            tracing::debug!(path = %path.display(), "File unchanged, skipping");
            return Ok(0);
        }

        // Remove old chunks
        self.db.with_conn(|conn| {
            delete_chunks_by_file(conn, &path.to_string_lossy())?;
            Ok(())
        })?;

        // Chunk the file
        let chunks = self.chunker.chunk_content(&content, request.language.as_deref());

        if chunks.is_empty() {
            return Ok(0);
        }

        // Generate embeddings
        let embeddings = self.generate_embeddings(&chunks).await?;

        // Store chunks
        let path_str = path.to_string_lossy().to_string();
        let mut count = 0;

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            let record = ChunkRecord::new(
                &path_str,
                chunk.index as i32,
                chunk.start_line as i32,
                chunk.end_line as i32,
                &chunk.content,
                &file_hash,
            )
            .with_language(request.language.clone().unwrap_or_default())
            .with_embedding(embedding.clone());

            self.db.with_conn(|conn| {
                insert_chunk(conn, &record)?;
                Ok(())
            })?;

            count += 1;
        }

        // Update file state
        self.update_file_state(path, &file_hash)?;

        tracing::info!(
            path = %path.display(),
            chunks = count,
            "Indexed file"
        );

        Ok(count)
    }

    /// Delete index for a file.
    ///
    /// # Errors
    ///
    /// Returns an error if deletion fails.
    pub fn delete_file(&self, path: &Path) -> Result<usize> {
        let path_str = path.to_string_lossy();

        let deleted = self.db.with_conn(|conn| {
            let count = delete_chunks_by_file(conn, &path_str)?;
            // Remove file state
            conn.execute("DELETE FROM file_state WHERE path = ?", [&*path_str])
                .ok();
            Ok(count)
        })?;

        if deleted > 0 {
            tracing::info!(path = %path.display(), chunks = deleted, "Deleted file from index");
        }

        Ok(deleted)
    }

    /// Check if file is already indexed with same hash.
    fn is_already_indexed(&self, path: &Path, hash: &str) -> Result<bool> {
        let path_str = path.to_string_lossy();

        self.db.with_conn(|conn| {
            let result: Result<String, _> = conn.query_row(
                "SELECT hash FROM file_state WHERE path = ?",
                [&*path_str],
                |row| row.get(0),
            );

            match result {
                Ok(stored_hash) => Ok(stored_hash == hash),
                Err(_) => Ok(false),
            }
        })
    }

    /// Update file state after indexing.
    fn update_file_state(&self, path: &Path, hash: &str) -> Result<()> {
        let metadata = std::fs::metadata(path)?;
        let mtime = metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);
        let size = metadata.len() as i64;
        let path_str = path.to_string_lossy();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO file_state (path, mtime, size, hash, last_indexed) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![&*path_str, mtime, size, hash, now],
            )?;
            Ok(())
        })
    }

    /// Generate embeddings for chunks.
    async fn generate_embeddings(&self, chunks: &[super::chunker::CodeChunk]) -> Result<Vec<Vec<f32>>> {
        if let Some(ref service) = self.embeddings {
            if service.is_initialized() {
                let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
                return service.embed_batch(texts).await;
            }
        }

        // Fallback to placeholder embeddings
        Ok(chunks.iter().map(|c| placeholder_embedding(&c.content)).collect())
    }

    /// Run the indexer loop processing requests from a channel.
    pub async fn run(
        self: Arc<Self>,
        mut index_rx: mpsc::Receiver<IndexRequest>,
        mut delete_rx: mpsc::Receiver<std::path::PathBuf>,
    ) {
        tracing::info!("Indexer started");

        loop {
            tokio::select! {
                Some(request) = index_rx.recv() => {
                    if let Err(e) = self.index_file(&request).await {
                        tracing::error!(path = %request.path.display(), error = %e, "Failed to index file");
                    }
                }
                Some(path) = delete_rx.recv() => {
                    if let Err(e) = self.delete_file(&path) {
                        tracing::error!(path = %path.display(), error = %e, "Failed to delete file from index");
                    }
                }
                else => {
                    tracing::info!("Indexer channels closed, shutting down");
                    break;
                }
            }
        }
    }
}

/// Compute blake3 hash of content.
fn compute_hash(content: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{init_storage, migrate};
    use tempfile::TempDir;
    use std::fs;

    fn setup_test_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();
        db
    }

    #[tokio::test]
    async fn test_index_file() {
        let db = setup_test_db();
        let indexer = Indexer::new(db.clone(), None);

        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}").unwrap();

        let request = IndexRequest {
            path: file_path.clone(),
            language: Some("rust".to_string()),
        };

        let count = indexer.index_file(&request).await.unwrap();
        assert!(count > 0);

        // Verify chunks in database
        let chunks = db.with_conn(|conn| {
            crate::storage::get_chunks_by_file(conn, &file_path.to_string_lossy())
        }).unwrap();

        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_reindex_unchanged() {
        let db = setup_test_db();
        let indexer = Indexer::new(db, None);

        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let request = IndexRequest {
            path: file_path.clone(),
            language: Some("rust".to_string()),
        };

        // First index
        let count1 = indexer.index_file(&request).await.unwrap();
        assert!(count1 > 0);

        // Second index (unchanged)
        let count2 = indexer.index_file(&request).await.unwrap();
        assert_eq!(count2, 0); // Should skip
    }

    #[tokio::test]
    async fn test_delete_file() {
        let db = setup_test_db();
        let indexer = Indexer::new(db.clone(), None);

        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let request = IndexRequest {
            path: file_path.clone(),
            language: Some("rust".to_string()),
        };

        indexer.index_file(&request).await.unwrap();

        let deleted = indexer.delete_file(&file_path).unwrap();
        assert!(deleted > 0);

        // Verify empty
        let chunks = db.with_conn(|conn| {
            crate::storage::get_chunks_by_file(conn, &file_path.to_string_lossy())
        }).unwrap();

        assert!(chunks.is_empty());
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("hello");
        let hash2 = compute_hash("hello");
        let hash3 = compute_hash("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // blake3 hex is 64 chars
    }
}
```

**Update `src/watcher/mod.rs`** - add:
```rust
mod indexer;

pub use indexer::Indexer;
```

**Verification Commands**:
```bash
cargo test watcher::indexer:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. 4 passed; 0 failed"
```

**Success Criteria**:
- [ ] Files indexed correctly
- [ ] Unchanged files skipped
- [ ] Deleted files removed from index
- [ ] All indexer tests pass
- [ ] Commit made with message "feat(watcher): build incremental indexing pipeline"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/watcher/indexer.rs` (X lines)
- **Files Modified**:
  - `src/watcher/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-2-indexing
- **Notes**: (any additional context)

---

### Subtask 2.2.3: Add File State Tracking (Single Session)

**Prerequisites**:
- [x] 2.2.2: Build Incremental Indexing Pipeline

**Deliverables**:
- [ ] Create file state query functions
- [ ] Add initial scan functionality
- [ ] Implement change detection
- [ ] Write file state tests

**Files to Create**:

**`src/storage/file_state.rs`** (complete file):
```rust
//! File state storage for incremental indexing.

use rusqlite::Connection;
use std::path::Path;

use super::models::FileState;
use crate::error::StorageError;
use crate::Result;

/// Get file state by path.
pub fn get_file_state(conn: &Connection, path: &str) -> Result<Option<FileState>> {
    let result = conn.query_row(
        "SELECT path, mtime, size, hash, last_indexed FROM file_state WHERE path = ?",
        [path],
        |row| {
            Ok(FileState {
                path: row.get(0)?,
                mtime: row.get(1)?,
                size: row.get(2)?,
                hash: row.get(3)?,
                last_indexed: row.get(4)?,
            })
        },
    );

    match result {
        Ok(state) => Ok(Some(state)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(StorageError::Database(e.to_string()).into()),
    }
}

/// Update or insert file state.
pub fn upsert_file_state(conn: &Connection, state: &FileState) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO file_state (path, mtime, size, hash, last_indexed) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![state.path, state.mtime, state.size, state.hash, state.last_indexed],
    )
    .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

/// Delete file state.
pub fn delete_file_state(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("DELETE FROM file_state WHERE path = ?", [path])
        .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

/// List all tracked file paths.
pub fn list_file_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT path FROM file_state ORDER BY path")
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let paths = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| StorageError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(paths)
}

/// Check if a file needs reindexing based on mtime.
pub fn needs_reindex(conn: &Connection, path: &str, current_mtime: i64) -> Result<bool> {
    match get_file_state(conn, path)? {
        Some(state) => Ok(state.mtime < current_mtime),
        None => Ok(true), // New file
    }
}

/// Count tracked files.
pub fn count_tracked_files(conn: &Connection) -> Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM file_state", [], |row| row.get(0))
        .map_err(|e| StorageError::Database(e.to_string()).into())
}

/// Find stale entries (files no longer on disk).
pub fn find_stale_entries(conn: &Connection, base_path: &Path) -> Result<Vec<String>> {
    let all_paths = list_file_paths(conn)?;
    let stale: Vec<String> = all_paths
        .into_iter()
        .filter(|p| {
            let path = Path::new(p);
            path.starts_with(base_path) && !path.exists()
        })
        .collect();
    Ok(stale)
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
    fn test_upsert_and_get() {
        let db = setup_db();

        db.with_conn(|conn| {
            let state = FileState::new("/test/file.rs", 1234567890, 1024, "abc123");
            upsert_file_state(conn, &state)?;

            let retrieved = get_file_state(conn, "/test/file.rs")?.unwrap();
            assert_eq!(retrieved.path, "/test/file.rs");
            assert_eq!(retrieved.mtime, 1234567890);
            assert_eq!(retrieved.hash, "abc123");

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_get_nonexistent() {
        let db = setup_db();

        let result = db.with_conn(|conn| get_file_state(conn, "/nonexistent")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let db = setup_db();

        db.with_conn(|conn| {
            upsert_file_state(conn, &FileState::new("/test.rs", 0, 0, "hash"))?;
            assert!(get_file_state(conn, "/test.rs")?.is_some());

            delete_file_state(conn, "/test.rs")?;
            assert!(get_file_state(conn, "/test.rs")?.is_none());

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_needs_reindex() {
        let db = setup_db();

        db.with_conn(|conn| {
            // New file needs reindex
            assert!(needs_reindex(conn, "/new.rs", 100)?);

            // Add file state
            upsert_file_state(conn, &FileState::new("/test.rs", 100, 0, "hash"))?;

            // Same mtime - no reindex
            assert!(!needs_reindex(conn, "/test.rs", 100)?);

            // Newer mtime - needs reindex
            assert!(needs_reindex(conn, "/test.rs", 200)?);

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_list_and_count() {
        let db = setup_db();

        db.with_conn(|conn| {
            assert_eq!(count_tracked_files(conn)?, 0);

            upsert_file_state(conn, &FileState::new("/a.rs", 0, 0, "h"))?;
            upsert_file_state(conn, &FileState::new("/b.rs", 0, 0, "h"))?;

            assert_eq!(count_tracked_files(conn)?, 2);
            assert_eq!(list_file_paths(conn)?.len(), 2);

            Ok(())
        })
        .unwrap();
    }
}
```

**Update `src/storage/mod.rs`** - add:
```rust
mod file_state;

pub use file_state::{
    count_tracked_files, delete_file_state, find_stale_entries, get_file_state,
    list_file_paths, needs_reindex, upsert_file_state,
};
```

**Verification Commands**:
```bash
cargo test storage::file_state:: --verbose 2>&1 | tail -30
# Expected: "test result: ok. 5 passed; 0 failed"

cargo test 2>&1 | grep "test result"
# Expected: "test result: ok"
```

**Success Criteria**:
- [ ] File state CRUD operations work
- [ ] Change detection works
- [ ] All file state tests pass
- [ ] All tests still pass
- [ ] Commit made with message "feat(storage): add file state tracking for incremental indexing"

---

**Completion Notes**:
- **Implementation**: (describe what was done)
- **Files Created**:
  - `src/storage/file_state.rs` (X lines)
- **Files Modified**:
  - `src/storage/mod.rs` (X lines)
- **Tests**: X tests passing
- **Build**: ✅ cargo test passes
- **Branch**: feature/2-2-indexing
- **Notes**: (any additional context)

---

### Task 2.2 Complete - Squash Merge

- [ ] All subtasks complete (2.2.1 - 2.2.3)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Squash merge to main
- [ ] Push to remote
- [ ] Delete branch

---

## Phase 2 Complete

**Phase 2 Checklist**:
- [ ] Task 2.1 merged (file watcher + filtering)
- [ ] Task 2.2 merged (chunking + indexing + file state)
- [ ] All tests pass (60+ tests)
- [ ] File watching functional
- [ ] Incremental indexing working

**Ready for Phase 3**: Lessons & Checkpoints

---

*Phase 2 Plan - Nellie Production*
