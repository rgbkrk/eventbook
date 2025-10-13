use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use eventbook_core::{
    Event, EventBuilder, EventError, EventResult, EventStore, InMemoryEventStore,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

/// App state shared across handlers
pub type AppState = Arc<RwLock<InMemoryEventStore>>;

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
    State(store): State<AppState>,
    Json(req): Json<SubmitEventRequest>,
) -> Result<Json<SubmitEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut store = store.write().await;

    // Get the next version for this aggregate
    let current_version = store.get_latest_version(&req.aggregate_id);
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
    store.append_event(event).map_err(event_error_to_response)?;

    info!("Event {} submitted successfully", event_id);

    Ok(Json(SubmitEventResponse { event_id, version }))
}

pub async fn get_events(
    State(store): State<AppState>,
    Query(query): Query<GetEventsQuery>,
) -> Result<Json<GetEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = store.read().await;

    let events = match query.aggregate_id.as_ref() {
        Some(aggregate_id) => store.get_events(aggregate_id),
        None => store.get_all_events(),
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
    State(store): State<AppState>,
    Path(aggregate_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let store = store.read().await;

    let events = store.get_events(&aggregate_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    let latest_version = store.get_latest_version(&aggregate_id);

    Ok(Json(serde_json::json!({
        "aggregate_id": aggregate_id,
        "latest_version": latest_version,
        "event_count": events.len(),
        "first_event_timestamp": events.first().map(|e| e.timestamp),
        "last_event_timestamp": events.last().map(|e| e.timestamp),
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
pub fn create_app(store: AppState) -> Router {
    Router::new()
        .route("/", get(serve_client))
        .route("/health", get(health_check))
        .route("/events", post(submit_event))
        .route("/events", get(get_events))
        .route("/aggregates/{aggregate_id}", get(get_aggregate_info))
        .layer(CorsLayer::permissive())
        .with_state(store)
}

/// Start the server
pub async fn start_server(port: u16) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Initializing EventBook server...");

    // Create the event store (in-memory for now)
    let store = InMemoryEventStore::new();
    let app_state = Arc::new(RwLock::new(store));

    info!("Event store initialized (in-memory)");

    // Create the app
    let app = create_app(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("EventBook server listening on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
