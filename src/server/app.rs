//! Main application server.
//!
//! Provides the complete server application with signal handling
//! and graceful shutdown coordination.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::auth::ApiKeyConfig;
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
    /// API key for authentication (None = disabled)
    pub api_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            shutdown_timeout: Duration::from_secs(30),
            api_key: None,
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
        let state = Arc::new(McpState::with_api_key(db, config.api_key.clone()));
        Self { config, state }
    }

    /// Get the API key configuration for this app.
    fn api_key_config(&self) -> Arc<ApiKeyConfig> {
        Arc::new(ApiKeyConfig::new(self.config.api_key.clone()))
    }

    /// Build the router with all endpoints.
    fn router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let api_key_config = self.api_key_config();

        Router::new()
            .merge(create_mcp_router(Arc::clone(&self.state)))
            .merge(create_rest_router(Arc::clone(&self.state)))
            .layer(middleware::from_fn(auth_middleware_wrapper(api_key_config)))
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

/// Create an authentication middleware function.
fn auth_middleware_wrapper(
    config: Arc<ApiKeyConfig>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send
       + 'static {
    move |request: Request, next: Next| {
        let config = Arc::clone(&config);
        Box::pin(async move {
            // Allow /health endpoint without authentication (needed for load balancers)
            if request.uri().path() == "/health" {
                return next.run(request).await;
            }

            // If authentication is disabled, allow the request
            if !config.is_enabled() {
                return next.run(request).await;
            }

            // Extract API key from headers
            let api_key = extract_api_key_from_headers(request.headers());

            // Validate the key
            if let Some(key) = api_key {
                if config.validate(&key) {
                    return next.run(request).await;
                }
            }

            // Authentication failed
            tracing::warn!(
                path = %request.uri(),
                method = %request.method(),
                "Authentication failed - invalid or missing API key"
            );

            (
                StatusCode::UNAUTHORIZED,
                "Unauthorized - invalid or missing API key",
            )
                .into_response()
        })
    }
}

/// Extract API key from request headers.
fn extract_api_key_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    // Check Authorization header (Bearer scheme)
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key.to_string());
            }
        }
    }

    // Check X-API-Key header
    if let Some(key_header) = headers.get("x-api-key") {
        if let Ok(key_str) = key_header.to_str() {
            return Some(key_str.to_string());
        }
    }

    None
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
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(30));
        assert_eq!(config.api_key, None);
    }

    #[test]
    fn test_server_config_custom() {
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 9000,
            shutdown_timeout: Duration::from_secs(60),
            api_key: Some("test-key".to_string()),
        };
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.shutdown_timeout, Duration::from_secs(60));
        assert_eq!(config.api_key, Some("test-key".to_string()));
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

    #[tokio::test]
    async fn test_health_without_auth() {
        let config = ServerConfig::default();
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_without_api_key_auth_disabled() {
        let config = ServerConfig::default();
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_without_api_key_auth_enabled() {
        let config = ServerConfig {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_with_wrong_api_key() {
        let config = ServerConfig {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("x-api-key", "wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_with_correct_api_key_header() {
        let config = ServerConfig {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("x-api-key", "secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_with_correct_bearer_token() {
        let config = ServerConfig {
            api_key: Some("secret-key".to_string()),
            ..Default::default()
        };
        let db = Database::open_in_memory().unwrap();
        db.with_conn(|conn| migrate(conn)).unwrap();

        let app = App::new(config, db);
        let router = app.router();

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("authorization", "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
