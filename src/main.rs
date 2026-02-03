//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server with CLI subcommands.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::{Parser, Subcommand};
use nellie::server::{init_metrics, init_tracing, App, ServerConfig};
use nellie::storage::{init_storage, Database};
use nellie::{Config, Result};
use std::path::PathBuf;

/// Nellie Production - Semantic code memory system for enterprise teams
///
/// A production-grade semantic code search engine with AI-powered indexing,
/// lessons management, and agent checkpoints.
#[derive(Parser, Debug)]
#[command(name = "nellie")]
#[command(version)]
#[command(long_about = None)]
#[command(about = "Semantic code memory system for enterprise engineering teams")]
struct Cli {
    /// Data directory for `SQLite` database
    #[arg(
        short,
        long,
        env = "NELLIE_DATA_DIR",
        default_value = "./data",
        global = true
    )]
    data_dir: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "NELLIE_LOG_LEVEL", default_value = "info", global = true)]
    log_level: String,

    /// Enable JSON logging output
    #[arg(long, env = "NELLIE_LOG_JSON", global = true)]
    log_json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the Nellie server
    ///
    /// Starts the MCP and REST API server for semantic code search,
    /// lessons management, and agent checkpoints. Optionally watches
    /// specified directories for automatic indexing.
    Serve {
        /// Host address to bind to
        #[arg(long, env = "NELLIE_HOST", default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, env = "NELLIE_PORT", default_value = "8080")]
        port: u16,

        /// Directories to watch for code changes (comma-separated)
        #[arg(short, long, env = "NELLIE_WATCH_DIRS", value_delimiter = ',')]
        watch: Vec<PathBuf>,

        /// Number of embedding worker threads
        #[arg(long, env = "NELLIE_EMBEDDING_THREADS", default_value = "4")]
        embedding_threads: usize,
    },

    /// Manually index a directory
    ///
    /// Triggers immediate indexing of one or more directories.
    /// Useful for forcing re-indexing without waiting for file watcher.
    Index {
        /// Path(s) to index (comma-separated)
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Number of embedding worker threads
        #[arg(long, env = "NELLIE_EMBEDDING_THREADS", default_value = "4")]
        embedding_threads: usize,
    },

    /// Search for code semantically
    ///
    /// Performs a semantic search across indexed code.
    /// Requires the server to be running in another terminal.
    Search {
        /// Search query (natural language or code keywords)
        #[arg(value_name = "QUERY")]
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        threshold: f32,

        /// Server URL
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,
    },

    /// Show server status and statistics
    ///
    /// Displays current server status, configuration, and indexed statistics.
    /// Requires the server to be running.
    Status {
        /// Server URL
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing with configuration
    init_tracing(&cli.log_level, cli.log_json);

    tracing::info!(
        "Nellie Production v{} - Semantic code memory system",
        env!("CARGO_PKG_VERSION")
    );

    // Route to appropriate command handler
    match cli.command {
        Some(Commands::Serve {
            host,
            port,
            watch,
            embedding_threads,
        }) => {
            serve_command(
                cli.data_dir,
                host,
                port,
                watch,
                embedding_threads,
                cli.log_level,
            )
            .await
        }
        Some(Commands::Index {
            paths,
            embedding_threads,
        }) => index_command(cli.data_dir, paths, embedding_threads),
        Some(Commands::Search {
            query,
            limit,
            threshold,
            server,
        }) => search_command(query, limit, threshold, server),
        Some(Commands::Status { server, format }) => status_command(server, format),
        None => {
            // Default to serve command for backward compatibility
            tracing::info!("No command specified, starting server (use 'serve' explicitly)");
            serve_command(
                cli.data_dir,
                "127.0.0.1".to_string(),
                8080,
                vec![],
                4,
                cli.log_level,
            )
            .await
        }
    }
}

/// Serve command: Start the Nellie server
async fn serve_command(
    data_dir: PathBuf,
    host: String,
    port: u16,
    watch: Vec<PathBuf>,
    embedding_threads: usize,
    log_level: String,
) -> Result<()> {
    tracing::info!("Starting Nellie server...");

    // Build config from CLI arguments
    let config = Config {
        data_dir,
        host: host.clone(),
        port,
        log_level,
        watch_dirs: watch.clone(),
        embedding_threads,
    };

    tracing::debug!(?config, "Configuration loaded");

    // Validate config
    config.validate()?;

    tracing::info!(
        "Server binding to {}:{}, data directory: {:?}",
        host,
        port,
        config.data_dir
    );

    if !watch.is_empty() {
        tracing::info!("Watching directories: {:?}", watch);
    }

    // Initialize database
    let db = Database::open(config.database_path())?;
    init_storage(&db)?;

    // Initialize metrics
    init_metrics();

    // Create and run server
    let server_config = ServerConfig {
        host,
        port,
        ..Default::default()
    };

    let app = App::new(server_config, db);
    app.run().await
}

/// Index command: Manually index directories
#[allow(clippy::unnecessary_wraps)]
fn index_command(_data_dir: PathBuf, paths: Vec<PathBuf>, embedding_threads: usize) -> Result<()> {
    if paths.is_empty() {
        return Err(nellie::Error::internal(
            "at least one path must be specified",
        ));
    }

    tracing::info!(
        "Starting manual indexing of {} directories with {} threads",
        paths.len(),
        embedding_threads
    );

    // Initialize database
    let db = Database::open(Config::default().database_path())?;
    init_storage(&db)?;

    // Initialize metrics
    init_metrics();

    for path in paths {
        if !path.exists() {
            tracing::warn!("Path does not exist: {:?}", path);
            continue;
        }

        tracing::info!("Indexing: {:?}", path);
        // TODO: Implement directory indexing
        // This will be called from watcher module with actual indexing logic
    }

    tracing::info!("Indexing complete");
    Ok(())
}

/// Search command: Perform semantic search
#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn search_command(query: String, limit: usize, threshold: f32, server: String) -> Result<()> {
    tracing::info!(
        "Searching for: '{}' (limit={}, threshold={})",
        query,
        limit,
        threshold
    );

    // TODO: Implement REST client for search
    // This will connect to running server and fetch results
    tracing::warn!("Search command not yet fully implemented");
    println!("Query: {query}");
    println!("Limit: {limit}");
    println!("Threshold: {threshold}");
    println!("Server: {server}");

    Ok(())
}

/// Status command: Show server status
#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn status_command(server: String, format: String) -> Result<()> {
    tracing::info!("Fetching status from {server}");

    // TODO: Implement REST client for status
    // This will connect to running server and fetch stats
    tracing::warn!("Status command not yet fully implemented");
    println!("Server: {server}");
    println!("Format: {format}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_serve() {
        let args = vec!["nellie", "serve", "--host", "0.0.0.0", "--port", "9000"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Serve {
            host,
            port,
            watch,
            embedding_threads,
        }) = cli.command
        {
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 9000);
            assert!(watch.is_empty());
            assert_eq!(embedding_threads, 4);
        } else {
            panic!("Expected Serve command");
        }
    }

    #[test]
    fn test_cli_parsing_index() {
        let args = vec!["nellie", "index", "/path/to/code"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Index {
            paths,
            embedding_threads,
        }) = cli.command
        {
            assert_eq!(paths.len(), 1);
            assert_eq!(embedding_threads, 4);
        } else {
            panic!("Expected Index command");
        }
    }

    #[test]
    fn test_cli_parsing_search() {
        let args = vec!["nellie", "search", "find auth handler"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Search {
            query,
            limit,
            threshold,
            server,
        }) = cli.command
        {
            assert_eq!(query, "find auth handler");
            assert_eq!(limit, 10);
            assert_eq!(threshold, 0.5);
            assert_eq!(server, "http://127.0.0.1:8080");
        } else {
            panic!("Expected Search command");
        }
    }

    #[test]
    fn test_cli_parsing_status() {
        let args = vec!["nellie", "status"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Status { server, format }) = cli.command {
            assert_eq!(server, "http://127.0.0.1:8080");
            assert_eq!(format, "text");
        } else {
            panic!("Expected Status command");
        }
    }

    #[test]
    fn test_cli_global_options() {
        let args = vec![
            "nellie",
            "--data-dir",
            "/custom/data",
            "--log-level",
            "debug",
            "serve",
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert_eq!(cli.data_dir, PathBuf::from("/custom/data"));
        assert_eq!(cli.log_level, "debug");
    }

    #[test]
    fn test_cli_json_logging() {
        let args = vec!["nellie", "--log-json", "serve"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(cli.log_json);
    }

    #[test]
    fn test_cli_search_with_options() {
        let args = vec![
            "nellie",
            "search",
            "database query",
            "--limit",
            "20",
            "--threshold",
            "0.7",
            "--server",
            "http://custom.server:9000",
        ];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Search {
            query,
            limit,
            threshold,
            server,
        }) = cli.command
        {
            assert_eq!(query, "database query");
            assert_eq!(limit, 20);
            assert_eq!(threshold, 0.7);
            assert_eq!(server, "http://custom.server:9000");
        } else {
            panic!("Expected Search command");
        }
    }

    #[test]
    fn test_cli_help_message() {
        // Test that help parsing doesn't crash
        let args = vec!["nellie", "--help"];
        let cli = Cli::try_parse_from(args);
        // --help causes exit, so we just verify parsing doesn't panic
        // Real test would need to capture output
    }
}
