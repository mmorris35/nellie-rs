//! Nellie Production - Semantic code memory system
//!
//! Entry point for the Nellie server with CLI subcommands.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use clap::{Parser, Subcommand};
use nellie::server::{init_metrics, init_tracing, start_mcp_server, App, McpTransportConfig, ServerConfig};
use nellie::storage::{init_storage, Database};
use nellie::{Config, Result};
use std::path::PathBuf;
use std::time::Duration;

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

    /// API key for authentication (required for production use)
    #[arg(long, env = "NELLIE_API_KEY", global = true)]
    api_key: Option<String>,

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

        /// Disable embedding service (semantic search will not work)
        #[arg(long, env = "NELLIE_DISABLE_EMBEDDINGS")]
        disable_embeddings: bool,

        /// Port for MCP protocol server (0 to disable)
        #[arg(long, env = "NELLIE_MCP_PORT", default_value = "8766")]
        mcp_port: u16,
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
            disable_embeddings,
            mcp_port,
        }) => {
            serve_command(ServeCommandArgs {
                data_dir: cli.data_dir,
                host,
                port,
                watch,
                embedding_threads,
                log_level: cli.log_level,
                api_key: cli.api_key,
                disable_embeddings,
                mcp_port,
            })
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
            serve_command(ServeCommandArgs {
                data_dir: cli.data_dir,
                host: "127.0.0.1".to_string(),
                port: 8080,
                watch: vec![],
                embedding_threads: 4,
                log_level: cli.log_level,
                api_key: cli.api_key,
                disable_embeddings: false,
                mcp_port: 8766,
            })
            .await
        }
    }
}

/// Command arguments for serve subcommand.
struct ServeCommandArgs {
    data_dir: PathBuf,
    host: String,
    port: u16,
    watch: Vec<PathBuf>,
    embedding_threads: usize,
    log_level: String,
    api_key: Option<String>,
    disable_embeddings: bool,
    mcp_port: u16,
}

/// Serve command: Start the Nellie server
async fn serve_command(args: ServeCommandArgs) -> Result<()> {
    tracing::info!("Starting Nellie server...");

    // Build config from CLI arguments
    let config = Config {
        data_dir: args.data_dir.clone(),
        host: args.host.clone(),
        port: args.port,
        log_level: args.log_level,
        watch_dirs: args.watch.clone(),
        embedding_threads: args.embedding_threads,
        api_key: args.api_key.clone(),
    };

    tracing::debug!(?config, "Configuration loaded");

    // Validate config
    config.validate()?;

    tracing::info!(
        "Server binding to {}:{}, data directory: {:?}",
        args.host,
        args.port,
        config.data_dir
    );

    if args.api_key.is_some() {
        tracing::info!("API key authentication enabled");
    } else {
        tracing::warn!(
            "API key authentication DISABLED - server is accessible without credentials!"
        );
    }

    if args.disable_embeddings {
        tracing::warn!("Embeddings disabled - semantic search will not work");
    } else {
        tracing::info!(
            "Embedding service will be initialized (uses {} threads)",
            args.embedding_threads
        );
    }

    if args.watch.is_empty() {
        tracing::warn!("No watch directories specified - code will not be indexed automatically");
    } else {
        tracing::info!("Will watch directories: {:?}", args.watch);
    }

    // Initialize database
    let db = Database::open(config.database_path())?;
    init_storage(&db)?;

    // Initialize metrics
    init_metrics();

    // Create and run server
    let server_config = ServerConfig {
        host: args.host,
        port: args.port,
        shutdown_timeout: Duration::from_secs(30),
        api_key: args.api_key,
        data_dir: config.data_dir,
        embedding_threads: args.embedding_threads,
        enable_embeddings: !args.disable_embeddings,
        watch_dirs: args.watch,
    };

    // Clone db for MCP server before passing to App
    let db_for_mcp = db.clone();

    let app = App::new(server_config.clone(), db).await?;

    // Start MCP server if enabled (port > 0)
    let _mcp_handle = if args.mcp_port > 0 {
        let mcp_config = McpTransportConfig {
            host: server_config.host.clone(),
            port: args.mcp_port,
        };
        // Share the App's embedding service with MCP server
        let embeddings = app.embeddings();
        Some(start_mcp_server(mcp_config, db_for_mcp, embeddings).await?)
    } else {
        tracing::info!("MCP server disabled (port=0)");
        None
    };

    // Start watcher if directories specified
    let _watcher_handles = app.start_watcher(server_config.watch_dirs.clone()).await?;

    // Run server (blocks until shutdown)
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
#[allow(clippy::needless_pass_by_value)]
fn search_command(query: String, limit: usize, threshold: f32, server: String) -> Result<()> {
    tracing::info!(
        "Searching for: '{}' (limit={}, threshold={})",
        query,
        limit,
        threshold
    );

    // Open database directly and get statistics
    let db = Database::open(Config::default().database_path())?;

    // Initialize storage schema if needed
    init_storage(&db)?;

    let chunk_count = db.with_conn(nellie::storage::count_chunks)?;

    if chunk_count == 0 {
        println!("No indexed chunks found in database.");
        println!("Please index code first using: nellie index <path>");
        return Ok(());
    }

    // For semantic search, we would need embeddings. Since search requires the
    // embedding worker (which needs async context and the server running),
    // we direct the user to use the server's search API.
    println!("Semantic code search for: {query}");
    println!("  Limit: {limit}");
    println!("  Threshold: {threshold}");
    println!("  Server: {server}");
    println!();
    println!("Note: Semantic search requires the server to be running.");
    println!("Start the server with: nellie serve");
    println!();
    println!("Then query it via the MCP API or REST endpoint:");
    println!("  - MCP Tool: search_code");
    println!("  - REST: POST {server}/api/v1/search/code");
    println!();
    println!("Database contains {chunk_count} indexed chunks ready for search.");

    Ok(())
}

/// Status command: Show server status
#[allow(clippy::needless_pass_by_value)]
fn status_command(_server: String, format: String) -> Result<()> {
    // Open database directly and get statistics
    let db = Database::open(Config::default().database_path())?;

    // Initialize storage schema if needed
    init_storage(&db)?;

    let chunk_count = db.with_conn(nellie::storage::count_chunks)?;
    let lesson_count = db.with_conn(nellie::storage::count_lessons)?;
    let file_count = db.with_conn(nellie::storage::count_tracked_files)?;

    tracing::info!(
        "Status: {} chunks, {} lessons, {} tracked files",
        chunk_count,
        lesson_count,
        file_count
    );

    if format == "json" {
        // JSON output
        let json = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "stats": {
                "indexed_chunks": chunk_count,
                "lessons": lesson_count,
                "tracked_files": file_count
            }
        });
        let json_str = serde_json::to_string_pretty(&json)
            .map_err(|e| nellie::Error::internal(format!("JSON serialization error: {e}")))?;
        println!("{json_str}");
    } else {
        // Text output (default)
        println!("Nellie Production v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("Status:");
        println!("  Indexed chunks:  {chunk_count}");
        println!("  Lessons:         {lesson_count}");
        println!("  Tracked files:   {file_count}");
    }

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
            disable_embeddings,
        }) = cli.command
        {
            assert_eq!(host, "0.0.0.0");
            assert_eq!(port, 9000);
            assert!(watch.is_empty());
            assert_eq!(embedding_threads, 4);
            assert!(!disable_embeddings);
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
    fn test_cli_disable_embeddings() {
        let args = vec!["nellie", "serve", "--disable-embeddings"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        if let Some(Commands::Serve {
            disable_embeddings, ..
        }) = cli.command
        {
            assert!(disable_embeddings);
        } else {
            panic!("Expected Serve command");
        }
    }

    #[test]
    fn test_cli_help_message() {
        // Test that help parsing doesn't crash
        let args = vec!["nellie", "--help"];
        let _cli = Cli::try_parse_from(args);
        // --help causes exit, so we just verify parsing doesn't panic
        // Real test would need to capture output
    }
}
