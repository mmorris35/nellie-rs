//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::Parser;
use nellie::{Config, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Nellie Production - Semantic code memory system
#[derive(Parser, Debug)]
#[command(name = "nellie")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Data directory for `SQLite` database
    #[arg(short, long, env = "NELLIE_DATA_DIR", default_value = "./data")]
    data_dir: std::path::PathBuf,

    /// Host address to bind to
    #[arg(long, env = "NELLIE_HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, env = "NELLIE_PORT", default_value = "8080")]
    port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "NELLIE_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Directories to watch for code changes (can be specified multiple times)
    #[arg(short, long, env = "NELLIE_WATCH_DIRS", value_delimiter = ',')]
    watch: Vec<std::path::PathBuf>,

    /// Number of embedding worker threads
    #[arg(long, env = "NELLIE_EMBEDDING_THREADS", default_value = "4")]
    embedding_threads: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Nellie Production v{} starting...",
        env!("CARGO_PKG_VERSION")
    );

    // Build config from CLI
    let config = Config {
        data_dir: cli.data_dir,
        host: cli.host,
        port: cli.port,
        log_level: cli.log_level,
        watch_dirs: cli.watch,
        embedding_threads: cli.embedding_threads,
    };

    tracing::debug!(?config, "Configuration loaded");

    // Validate config
    config.validate()?;

    tracing::info!(
        "Server will bind to {}:{}, data in {:?}",
        config.host,
        config.port,
        config.data_dir
    );

    // TODO: Start server in Phase 4
    tracing::info!("Server startup not yet implemented - exiting");

    Ok(())
}
