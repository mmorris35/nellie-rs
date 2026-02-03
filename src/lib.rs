//! Nellie Production Library
//!
//! Production-grade semantic code memory system for enterprise engineering teams.
//!
//! # Architecture
//!
//! Nellie is organized into the following modules:
//!
//! - [`config`]: Configuration management (CLI args, environment, files)
//! - [`error`]: Error types and Result aliases
//! - [`storage`]: `SQLite` database with `sqlite-vec` for vector search
//! - [`embeddings`]: ONNX-based embedding generation
//! - [`watcher`]: File system watching and indexing
//! - [`server`]: MCP and REST API servers
//!
//! # Example
//!
//! ```rust,ignore
//! use nellie::config::Config;
//! use nellie::server::Server;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load()?;
//!     let server = Server::new(config).await?;
//!     server.run().await
//! }
//! ```

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod embeddings;
pub mod error;
pub mod server;
pub mod storage;
pub mod watcher;

pub use config::Config;
pub use error::{Error, Result};
