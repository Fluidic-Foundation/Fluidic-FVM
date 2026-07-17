use crate::api::evm_rpc::evm_rpc_router;
use crate::api::routes::api_router;
use crate::api::state::ApiState;
use crate::api::websocket::start_broadcast_task;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::IntoResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

/// Global token-bucket state shared by the rate-limit middleware.
#[derive(Clone)]
struct RateLimitState {
    bucket: Arc<Mutex<TokenBucket>>,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
    rate_per_sec: f64,
    capacity: f64,
}

impl TokenBucket {
    fn new(rate_per_sec: f64, capacity: f64) -> Self {
        Self {
            tokens: capacity,
            last_update: Instant::now(),
            rate_per_sec,
            capacity,
        }
    }

    /// Try to consume one token. Returns true if allowed.
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_update = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    request: axum::extract::Request,
    next: Next,
) -> impl IntoResponse {
    let allowed = state.bucket.lock().map(|mut b| b.allow()).unwrap_or(true);
    if allowed {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "rate limit exceeded" })),
        )
            .into_response()
    }
}

pub async fn start_api_server(state: Arc<ApiState>, port: u16) -> Result<(), String> {
    start_broadcast_task(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let rate_limit_state = RateLimitState {
        bucket: Arc::new(Mutex::new(TokenBucket::new(300.0, 600.0))),
    };

    let app: Router = api_router()
        .merge(evm_rpc_router())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(256 * 1024))
        .layer(from_fn_with_state(rate_limit_state, rate_limit_middleware))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e| format!("invalid API address: {}", e))?;

    // Retry binding with SO_REUSEADDR. Railway (and similar platforms) may
    // keep the previous container's socket alive briefly during rolling
    // deploys, so we set SO_REUSEADDR and retry for up to several minutes.
    // On Linux we also set SO_REUSEPORT: this lets Railway start the new
    // container while the old one is still bound, avoiding a hard bind failure
    // during rolling deploys.  The kernel load-balances healthchecks across
    // both listeners only until the old container is stopped.
    let listener = {
        let mut last_err = None;
        let mut retries = 0;
        const MAX_RETRIES: u32 = 180;
        loop {
            match tokio::task::spawn_blocking(move || {
                let socket = socket2::Socket::new(
                    socket2::Domain::for_address(addr),
                    socket2::Type::STREAM,
                    None,
                )?;
                socket.set_nonblocking(true)?;
                socket.set_reuse_address(true)?;
                #[cfg(target_os = "linux")]
                socket.set_reuse_port(true)?;
                socket.bind(&addr.into())?;
                socket.listen(128)?;
                Ok::<_, std::io::Error>(std::net::TcpListener::from(socket))
            })
            .await
            {
                Ok(Ok(listener)) => break Ok(listener),
                Ok(Err(e)) => {
                    last_err = Some(e);
                    retries += 1;
                    if retries > MAX_RETRIES {
                        break Err(last_err.unwrap());
                    }
                    tracing::warn!(
                        "API port {} in use, retrying in 2s (attempt {}/{})",
                        port,
                        retries,
                        MAX_RETRIES
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Err(e) => break Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        }
    }
    .map_err(|e| format!("failed to bind API server: {}", e))?;

    let listener = tokio::net::TcpListener::from_std(listener)
        .map_err(|e| format!("failed to convert API listener: {}", e))?;

    tracing::info!("API server listening on http://{}", addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| format!("API server error: {}", e))
}
