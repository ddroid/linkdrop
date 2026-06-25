use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use linkdrop_core::{default_data_dir, storage::MAX_HTML_BYTES, Storage};
use serde::{Deserialize, Serialize};
use tokio::time;
use tracing::{info, warn};

const SWEEP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

struct AppState {
    storage: Storage,
    token: String,
    public_url: String,
}

#[derive(Deserialize)]
struct PushBody {
    html: String,
    slug: Option<String>,
    force: Option<bool>,
    ttl: Option<String>,
}

#[derive(Serialize)]
struct PageResponse {
    slug: String,
    url: String,
    created_at: String,
    expires_at: Option<String>,
    size_bytes: i64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkdrop_server=info,tower_http=info".into()),
        )
        .init();

    let token = std::env::var("LINKDROP_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| anyhow::anyhow!("LINKDROP_TOKEN must be set"))?;

    let public_url = std::env::var("LINKDROP_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
        .trim_end_matches('/')
        .to_string();

    let data_dir = default_data_dir();
    let storage = Storage::open(data_dir)?;
    info!("data directory: {}", storage.data_dir().display());

    let state = Arc::new(AppState {
        storage,
        token,
        public_url,
    });

    spawn_sweeper(state.clone());

    let api = Router::new()
        .route("/pages", post(push_page).get(list_pages))
        .route("/pages/{slug}", delete(delete_page))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .route("/", get(root_page))
        .route("/{slug}", get(serve_page))
        .merge(Router::new().nest("/api", api))
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn spawn_sweeper(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = time::interval(SWEEP_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            match state.storage.sweep_expired() {
                Ok(n) if n > 0 => info!("sweeper removed {n} expired page(s)"),
                Ok(_) => {}
                Err(e) => warn!("sweeper error: {e:#}"),
            }
        }
    });
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let provided = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| {
            request
                .headers()
                .get("x-linkdrop-token")
                .and_then(|v| v.to_str().ok())
        });

    if provided == Some(state.token.as_str()) {
        return next.run(request).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "unauthorized".into(),
        }),
    )
        .into_response()
}

async fn root_page() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("flappy.html"),
    )
}

async fn serve_page(State(state): State<Arc<AppState>>, Path(slug): Path<String>) -> Response {
    match state.storage.get(&slug) {
        Ok((_, content)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(e) => error_response(e),
    }
}

async fn push_page(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PushBody>,
) -> Response {
    let force = body.force.unwrap_or(false);
    let ttl = match body.ttl.as_deref() {
        Some(t) => match linkdrop_core::ttl::parse_ttl(t) {
            Ok(d) => Some(d),
            Err(e) => return error_response(e),
        },
        None => None,
    };

    let content = body.html.as_bytes();
    if content.len() > MAX_HTML_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse {
                error: format!("content exceeds maximum size of {} bytes", MAX_HTML_BYTES),
            }),
        )
            .into_response();
    }

    match state
        .storage
        .put(body.slug.as_deref(), content, force, ttl)
    {
        Ok(record) => Json(page_to_response(&state, &record)).into_response(),
        Err(e) => error_response(e),
    }
}

async fn list_pages(State(state): State<Arc<AppState>>) -> Response {
    match state.storage.list() {
        Ok(pages) => {
            let items: Vec<PageResponse> = pages
                .iter()
                .map(|p| page_to_response(&state, p))
                .collect();
            Json(items).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn delete_page(State(state): State<Arc<AppState>>, Path(slug): Path<String>) -> Response {
    match state.storage.delete(&slug) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_response(linkdrop_core::LinkdropError::NotFound(slug)),
        Err(e) => error_response(e),
    }
}

fn page_to_response(state: &AppState, record: &linkdrop_core::db::PageRecord) -> PageResponse {
    PageResponse {
        slug: record.slug.clone(),
        url: format!("{}/{}", state.public_url, record.slug),
        created_at: record.created_at.to_rfc3339(),
        expires_at: record.expires_at.map(|t| t.to_rfc3339()),
        size_bytes: record.size_bytes,
    }
}

fn error_response(err: linkdrop_core::LinkdropError) -> Response {
    let status = StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
        .into_response()
}
