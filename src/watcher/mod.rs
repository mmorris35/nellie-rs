//! File system watching and indexing.
//!
//! This module provides:
//! - Directory watching using notify-rs
//! - Gitignore-aware file filtering
//! - Incremental indexing of changed files

mod events;
mod filter;
mod handler;
#[allow(clippy::module_inception)]
mod watcher;

pub use events::{EventBatch, FileEvent};
pub use filter::FileFilter;
pub use handler::{EventHandler, HandlerConfig, IndexRequest, WatcherStats, WatcherStatsSnapshot};
pub use watcher::{FileWatcher, WatcherConfig};

/// Initialize watcher module.
pub fn init() {
    tracing::debug!("Watcher module initialized");
}
