use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use eventbook_core::{
    Event, EventBuilder, EventError, EventResult, EventStore, InMemoryEventStore, Projection, User,
    UserProjection,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

/// App state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub event_store: Arc<RwLock<InMemoryEventStore>>,
    pub user_projection: Arc<RwLock<UserProjection>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            event_store: Arc::new(RwLock::new(InMemoryEventStore::new())),
            user_projection: Arc::new(RwLock::new(UserProjection::new())),
        }
    }
}

/// Request/Response types for the API

#[derive(Debug, Deserialize)]
pub struct SubmitEventRequest {
    pub event_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SubmitEventResponse {
    pub event_id: String,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetEventsQuery {
    pub aggregate_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GetEventsResponse {
    pub events: Vec<Event>,
    pub total_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Convert EventError to HTTP status and error response
fn event_error_to_response(err: EventError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code) = match &err {
        EventError::InvalidVersion { .. } => (StatusCode::CONFLICT, "VERSION_CONFLICT"),
        EventError::DuplicateEventId(_) => (StatusCode::CONFLICT, "DUPLICATE_EVENT"),
        _ => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
    };

    (
        status,
        Json(ErrorResponse {
            error: err.to_string(),
            code: code.to_string(),
        }),
    )
}

/// HTTP handlers

pub async fn submit_event(
    State(app_state): State<AppState>,
    Json(req): Json<SubmitEventRequest>,
) -> Result<Json<SubmitEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut event_store = app_state.event_store.write().await;

    // Get the next version for this aggregate
    let current_version = event_store.get_latest_version(&req.aggregate_id);
    let next_version = current_version + 1;

    // Build the event
    let event = EventBuilder::new()
        .event_type(req.event_type)
        .aggregate_id(req.aggregate_id)
        .payload(req.payload)
        .map_err(event_error_to_response)?
        .build(next_version)
        .map_err(event_error_to_response)?;

    let event_id = event.id.clone();
    let version = event.version;

    // Store the event
    event_store
        .append_event(event.clone())
        .map_err(event_error_to_response)?;

    // Update projections
    let mut user_projection = app_state.user_projection.write().await;
    if let Err(e) = user_projection.apply_new_events(&[event]) {
        warn!("Failed to update user projection: {}", e);
    }

    info!("Event {} submitted successfully", event_id);

    Ok(Json(SubmitEventResponse { event_id, version }))
}

pub async fn get_events(
    State(app_state): State<AppState>,
    Query(query): Query<GetEventsQuery>,
) -> Result<Json<GetEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let event_store = app_state.event_store.read().await;

    let events = match query.aggregate_id.as_ref() {
        Some(aggregate_id) => event_store.get_events(aggregate_id),
        None => event_store.get_all_events(),
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    let total_count = events.len();

    // Apply client-side pagination if needed
    let events = if let (Some(limit), Some(offset)) = (query.limit, query.offset) {
        events
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect()
    } else {
        events
    };

    Ok(Json(GetEventsResponse {
        events,
        total_count,
    }))
}

pub async fn get_aggregate_info(
    State(app_state): State<AppState>,
    Path(aggregate_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let event_store = app_state.event_store.read().await;

    let events = event_store.get_events(&aggregate_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    let latest_version = event_store.get_latest_version(&aggregate_id);

    Ok(Json(serde_json::json!({
        "aggregate_id": aggregate_id,
        "latest_version": latest_version,
        "event_count": events.len(),
        "first_event_timestamp": events.first().map(|e| e.timestamp),
        "last_event_timestamp": events.last().map(|e| e.timestamp),
    })))
}

pub async fn get_users(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<User>>, (StatusCode, Json<ErrorResponse>)> {
    let user_projection = app_state.user_projection.read().await;
    let users = user_projection
        .get_active_users()
        .into_iter()
        .cloned()
        .collect();
    Ok(Json(users))
}

pub async fn get_user(
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<User>, (StatusCode, Json<ErrorResponse>)> {
    let user_projection = app_state.user_projection.read().await;

    match user_projection.get_user(&user_id) {
        Some(user) => Ok(Json(user.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("User {} not found", user_id),
                code: "USER_NOT_FOUND".to_string(),
            }),
        )),
    }
}

pub async fn rebuild_projections(
    State(app_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let event_store = app_state.event_store.read().await;
    let events = event_store.get_all_events().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    let mut user_projection = app_state.user_projection.write().await;
    user_projection.rebuild_from_events(&events).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "PROJECTION_REBUILD_FAILED".to_string(),
            }),
        )
    })?;

    info!("Projections rebuilt from {} events", events.len());

    Ok(Json(serde_json::json!({
        "success": true,
        "events_processed": events.len(),
        "user_count": user_projection.user_count()
    })))
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": eventbook_core::current_timestamp()
    }))
}

pub async fn serve_client() -> Html<&'static str> {
    Html(include_str!("../../client.html"))
}

/// Create the application router
pub fn create_app(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_client))
        .route("/health", get(health_check))
        .route("/events", post(submit_event))
        .route("/events", get(get_events))
        .route("/aggregates/{aggregate_id}", get(get_aggregate_info))
        .route("/users", get(get_users))
        .route("/users/{user_id}", get(get_user))
        .route("/projections/rebuild", post(rebuild_projections))
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}

/// Start the server
pub async fn start_server(port: u16) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Initializing EventBook server...");

    // Create the app state with event store and projections
    let app_state = AppState::new();

    info!("Event store initialized (in-memory)");

    // Create the app
    let app = create_app(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("EventBook server listening on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
