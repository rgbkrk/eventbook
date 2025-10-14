use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use eventbook_core::{
    DocumentProjection, Event, EventBuilder, EventError, EventStore, InMemoryEventStore, Projection,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

mod websocket;
use websocket::{websocket_handler, ConnectionManager};

/// App state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Map of store_id -> event store
    pub stores: Arc<RwLock<HashMap<String, InMemoryEventStore>>>,
    /// Map of store_id -> document projection
    pub projections: Arc<RwLock<HashMap<String, DocumentProjection>>>,
    /// WebSocket connection manager
    pub connection_manager: Arc<ConnectionManager>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            stores: Arc::new(RwLock::new(HashMap::new())),
            projections: Arc::new(RwLock::new(HashMap::new())),
            connection_manager: Arc::new(ConnectionManager::new()),
        }
    }

    /// Ensure a store exists for the given store_id
    async fn ensure_store_exists(&self, store_id: &str) {
        let mut stores = self.stores.write().await;
        let mut projections = self.projections.write().await;

        stores
            .entry(store_id.to_string())
            .or_insert_with(InMemoryEventStore::new);

        projections
            .entry(store_id.to_string())
            .or_insert_with(DocumentProjection::new);
    }
}

/// Request/Response types for the API

#[derive(Debug, Deserialize)]
pub struct SubmitEventRequest {
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SubmitEventResponse {
    pub event_id: String,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetEventsQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub since_timestamp: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct GetEventsResponse {
    pub events: Vec<Event>,
    pub total_count: usize,
    pub store_id: String,
}

#[derive(Debug, Serialize)]
pub struct StoreInfoResponse {
    pub store_id: String,
    pub event_count: usize,
    pub latest_version: i64,
    pub first_event_timestamp: Option<i64>,
    pub last_event_timestamp: Option<i64>,
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

/// Submit an event to a store
pub async fn submit_event(
    State(app_state): State<AppState>,
    Path(store_id): Path<String>,
    Json(req): Json<SubmitEventRequest>,
) -> Result<Json<SubmitEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    app_state.ensure_store_exists(&store_id).await;

    let mut stores = app_state.stores.write().await;
    let mut projections = app_state.projections.write().await;

    let event_store = stores.get_mut(&store_id).unwrap();
    let projection = projections.get_mut(&store_id).unwrap();

    // Get the next version for this store
    let current_version = event_store.get_latest_version(&store_id);
    let next_version = current_version + 1;

    // Build the event
    let event = EventBuilder::new()
        .event_type(req.event_type)
        .aggregate_id(store_id.clone()) // Use store_id as aggregate_id
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

    // Update projection
    if let Err(e) = projection.apply_new_events(&[event.clone()]) {
        warn!("Failed to update projection for store {}: {}", store_id, e);
    }

    // Broadcast event to WebSocket connections
    app_state
        .connection_manager
        .broadcast_event(store_id.clone(), event)
        .await;

    info!(
        "Event {} submitted to store {} successfully",
        event_id, store_id
    );

    Ok(Json(SubmitEventResponse { event_id, version }))
}

/// Get events from a store
pub async fn get_events(
    State(app_state): State<AppState>,
    Path(store_id): Path<String>,
    Query(query): Query<GetEventsQuery>,
) -> Result<Json<GetEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    app_state.ensure_store_exists(&store_id).await;

    let stores = app_state.stores.read().await;
    let event_store = stores.get(&store_id).unwrap();

    let mut events = event_store.get_events(&store_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    // Filter by timestamp if requested
    if let Some(since) = query.since_timestamp {
        events.retain(|e| e.timestamp > since);
    }

    let total_count = events.len();

    // Apply pagination if requested
    if let (Some(limit), Some(offset)) = (query.limit, query.offset) {
        events = events
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
    }

    Ok(Json(GetEventsResponse {
        events,
        total_count,
        store_id,
    }))
}

/// Get store information
pub async fn get_store_info(
    State(app_state): State<AppState>,
    Path(store_id): Path<String>,
) -> Result<Json<StoreInfoResponse>, (StatusCode, Json<ErrorResponse>)> {
    app_state.ensure_store_exists(&store_id).await;

    let stores = app_state.stores.read().await;
    let event_store = stores.get(&store_id).unwrap();

    let events = event_store.get_events(&store_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
                code: "EVENT_RETRIEVAL_FAILED".to_string(),
            }),
        )
    })?;

    let latest_version = event_store.get_latest_version(&store_id);

    Ok(Json(StoreInfoResponse {
        store_id,
        event_count: events.len(),
        latest_version,
        first_event_timestamp: events.first().map(|e| e.timestamp),
        last_event_timestamp: events.last().map(|e| e.timestamp),
    }))
}

/// List all stores
pub async fn list_stores(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {
    let stores = app_state.stores.read().await;
    let store_ids: Vec<String> = stores.keys().cloned().collect();
    Ok(Json(store_ids))
}

/// Health check
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": eventbook_core::current_timestamp()
    }))
}

/// Serve the client HTML
pub async fn serve_client() -> Html<&'static str> {
    Html(include_str!("../../client.html"))
}

/// Create the application router
pub fn create_app(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_client))
        .route("/health", get(health_check))
        .route("/stores", get(list_stores))
        .route("/stores/{store_id}/events", post(submit_event))
        .route("/stores/{store_id}/events", get(get_events))
        .route("/stores/{store_id}", get(get_store_info))
        .route("/stores/{store_id}/ws", get(websocket_handler))
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

    // Create the app state
    let app_state = AppState::new();

    info!("Event stores initialized (in-memory)");

    // Create the app
    let app = create_app(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("EventBook server listening on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
