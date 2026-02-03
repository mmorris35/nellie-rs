//! Main application server.
//!
//! Provides the complete server application with signal handling
//! and graceful shutdown coordination.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::mcp::{create_mcp_router, McpState};
use super::rest::create_rest_router;
use crate::storage::Database;
use crate::Result;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Shutdown timeout duration
    pub shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

/// Application server.
pub struct App {
    config: ServerConfig,
    state: Arc<McpState>,
}

impl App {
    /// Create a new application.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `db` - Database instance
    ///
    /// # Returns
    ///
    /// New application instance
    #[must_use]
    pub fn new(config: ServerConfig, db: Database) -> Self {
        let state = Arc::new(McpState::new(db));
        Self { config, state }
    }

    /// Build the router with all endpoints.
    fn router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .merge(create_mcp_router(Arc::clone(&self.state)))
            .merge(create_rest_router(Arc::clone(&self.state)))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &axum::http::Request<_>| {
                        let method = request.method();
                        let uri = request.uri();
                        let headers = request.headers();
                        let request_id = headers
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown");

                        tracing::info_span!(
                            "http_request",
                            method = %method,
                            uri = %uri,
                            request_id = %request_id,
                        )
                    })
                    .on_response(
                        |response: &axum::response::Response,
                         _latency: std::time::Duration,
                         _span: &tracing::Span| {
                            tracing::info!(
                                status = %response.status(),
                                "Request completed"
                            );
                        },
                    ),
            )
            .layer(cors)
    }

    /// Run the server until shutdown signal.
    ///
    /// The server listens for SIGTERM (Unix) and Ctrl+C signals,
    /// then gracefully shuts down all connections.
    ///
    /// # Errors
    ///
    /// Returns an error if the server cannot start or encounters
    /// a fatal error during execution.
    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| crate::Error::config(format!("invalid address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            crate::error::ServerError::BindFailed {
                address: addr.to_string(),
                reason: e.to_string(),
            }
        })?;

        tracing::info!(%addr, "Server listening");

        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| crate::error::ServerError::Request(e.to_string()))?;

        tracing::info!("Server shut down gracefully");
        Ok(())
    }
}

/// Wait for shutdown signal (SIGTERM or Ctrl+C).
///
/// This function will block until one of the following signals is received:
/// - `SIGTERM` (Unix/Linux only)
/// - `SIGINT` (Ctrl+C on all platforms)
///
/// Once a signal is received, the function returns and allows the server
/// to begin graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating shutdown");
        }
        () = terminate => {
            tracing::info!("Received SIGTERM, initiating shutdown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_server_config_custom() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 9000,
            shutdown_timeout: Duration::from_secs(60),
        };
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_app_creation() {
        let config = ServerConfig::default();
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let _app = App::new(config, db);
        // App created successfully
        assert!(true);
    }

    #[test]
    fn test_app_router() {
        let config = ServerConfig::default();
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let _router = app.router();
        // Router created successfully
        assert!(true);
    }
}
