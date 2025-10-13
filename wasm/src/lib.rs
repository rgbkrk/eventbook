use eventbook_core::{
    Event, EventBuilder, EventStore, InMemoryEventStore, Projection, User, UserProjection,
};
use js_sys::Promise;
use serde::{Deserialize, Serialize};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, Request, RequestInit, Response};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Set up panic hook for better error messages in browser
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// Console logging macro for debugging
macro_rules! log {
    ( $( $t:tt )* ) => {
        console::log_1(&format!( $( $t )* ).into());
    }
}

/// JavaScript-compatible Event type
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsEvent {
    id: String,
    event_type: String,
    aggregate_id: String,
    payload: String, // JSON string for JS compatibility
    timestamp: f64,  // JS numbers are f64
    version: f64,
}

#[wasm_bindgen]
impl JsEvent {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: String,
        event_type: String,
        aggregate_id: String,
        payload: String,
        timestamp: f64,
        version: f64,
    ) -> JsEvent {
        JsEvent {
            id,
            event_type,
            aggregate_id,
            payload,
            timestamp,
            version,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn event_type(&self) -> String {
        self.event_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn aggregate_id(&self) -> String {
        self.aggregate_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn payload(&self) -> String {
        self.payload.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> f64 {
        self.version
    }
}

impl From<Event> for JsEvent {
    fn from(event: Event) -> Self {
        JsEvent {
            id: event.id,
            event_type: event.event_type,
            aggregate_id: event.aggregate_id,
            payload: serde_json::to_string(&event.payload).unwrap_or_default(),
            timestamp: event.timestamp as f64,
            version: event.version as f64,
        }
    }
}

impl TryFrom<JsEvent> for Event {
    type Error = JsError;

    fn try_from(js_event: JsEvent) -> Result<Self, Self::Error> {
        let payload = serde_json::from_str(&js_event.payload)
            .map_err(|e| JsError::new(&format!("Invalid JSON payload: {}", e)))?;

        Ok(Event {
            id: js_event.id,
            event_type: js_event.event_type,
            aggregate_id: js_event.aggregate_id,
            payload,
            timestamp: js_event.timestamp as i64,
            version: js_event.version as i64,
        })
    }
}

/// JavaScript-compatible User type
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsUser {
    id: String,
    name: String,
    email: String,
    created_at: f64,
    updated_at: f64,
    active: bool,
}

#[wasm_bindgen]
impl JsUser {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn email(&self) -> String {
        self.email.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn created_at(&self) -> f64 {
        self.created_at
    }

    #[wasm_bindgen(getter)]
    pub fn updated_at(&self) -> f64 {
        self.updated_at
    }

    #[wasm_bindgen(getter)]
    pub fn active(&self) -> bool {
        self.active
    }
}

impl From<User> for JsUser {
    fn from(user: User) -> Self {
        JsUser {
            id: user.id,
            name: user.name,
            email: user.email,
            created_at: user.created_at as f64,
            updated_at: user.updated_at as f64,
            active: user.active,
        }
    }
}

/// Sync result for JavaScript
#[wasm_bindgen]
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    events_pulled: u32,
    success: bool,
    error_message: Option<String>,
}

#[wasm_bindgen]
impl SyncResult {
    #[wasm_bindgen(getter)]
    pub fn events_pulled(&self) -> u32 {
        self.events_pulled
    }

    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter)]
    pub fn error_message(&self) -> Option<String> {
        self.error_message.clone()
    }
}

/// Main EventBook client for browser
#[wasm_bindgen]
pub struct EventBookClient {
    local_store: InMemoryEventStore,
    user_projection: UserProjection,
    server_url: String,
}

#[wasm_bindgen]
impl EventBookClient {
    #[wasm_bindgen(constructor)]
    pub fn new(server_url: String) -> EventBookClient {
        log!("Creating EventBook client with server: {}", server_url);

        EventBookClient {
            local_store: InMemoryEventStore::new(),
            user_projection: UserProjection::new(),
            server_url,
        }
    }

    /// Submit an event locally
    #[wasm_bindgen]
    pub fn submit_event(
        &mut self,
        event_type: String,
        aggregate_id: String,
        payload: String,
    ) -> Result<JsEvent, JsError> {
        // Parse payload JSON
        let payload_value: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|e| JsError::new(&format!("Invalid JSON payload: {}", e)))?;

        // Get next version
        let current_version = self.local_store.get_latest_version(&aggregate_id);
        let next_version = current_version + 1;

        // Build event
        let event = EventBuilder::new()
            .event_type(event_type)
            .aggregate_id(aggregate_id)
            .payload(payload_value)
            .map_err(|e| JsError::new(&format!("Payload error: {}", e)))?
            .build(next_version)
            .map_err(|e| JsError::new(&format!("Event build error: {}", e)))?;

        // Store locally
        self.local_store
            .append_event(event.clone())
            .map_err(|e| JsError::new(&format!("Store error: {}", e)))?;

        // Update projection
        self.user_projection
            .apply_new_events(&[event.clone()])
            .map_err(|e| JsError::new(&format!("Projection error: {}", e)))?;

        log!("Event {} submitted locally", event.id);
        Ok(event.into())
    }

    /// Get all local events
    #[wasm_bindgen]
    pub fn get_events(&self) -> Result<js_sys::Array, JsError> {
        let events = self
            .local_store
            .get_all_events()
            .map_err(|e| JsError::new(&format!("Get events error: {}", e)))?;

        let js_array = js_sys::Array::new();
        for event in events {
            let js_event = JsEvent::from(event);
            js_array.push(&JsValue::from(js_event));
        }

        Ok(js_array)
    }

    /// Get events for specific aggregate
    #[wasm_bindgen]
    pub fn get_events_for_aggregate(&self, aggregate_id: String) -> Result<js_sys::Array, JsError> {
        let events = self
            .local_store
            .get_events(&aggregate_id)
            .map_err(|e| JsError::new(&format!("Get events error: {}", e)))?;

        let js_array = js_sys::Array::new();
        for event in events {
            let js_event = JsEvent::from(event);
            js_array.push(&JsValue::from(js_event));
        }

        Ok(js_array)
    }

    /// Get materialized users
    #[wasm_bindgen]
    pub fn get_materialized_users(&self) -> js_sys::Array {
        let users = self.user_projection.get_active_users();
        let js_array = js_sys::Array::new();

        for user in users {
            let js_user = JsUser::from(user.clone());
            js_array.push(&JsValue::from(js_user));
        }

        js_array
    }

    /// Get specific materialized user
    #[wasm_bindgen]
    pub fn get_materialized_user(&self, user_id: String) -> Option<JsUser> {
        self.user_projection
            .get_user(&user_id)
            .map(|u| JsUser::from(u.clone()))
    }

    /// Get user count
    #[wasm_bindgen]
    pub fn get_user_count(&self) -> u32 {
        self.user_projection.user_count() as u32
    }

    /// Get event count
    #[wasm_bindgen]
    pub fn get_event_count(&self) -> u32 {
        self.local_store.get_event_count() as u32
    }

    /// Clear local store
    #[wasm_bindgen]
    pub fn clear_local_store(&mut self) {
        self.local_store = InMemoryEventStore::new();
        self.user_projection = UserProjection::new();
        log!("Local store cleared");
    }

    /// Rebuild projections from local events
    #[wasm_bindgen]
    pub fn rebuild_projections(&mut self) -> Result<u32, JsError> {
        let events = self
            .local_store
            .get_all_events()
            .map_err(|e| JsError::new(&format!("Failed to get events: {}", e)))?;

        self.user_projection
            .rebuild_from_events(&events)
            .map_err(|e| JsError::new(&format!("Failed to rebuild projections: {}", e)))?;

        log!("Rebuilt projections from {} events", events.len());
        Ok(events.len() as u32)
    }

    /// Sync event log from server
    #[wasm_bindgen]
    pub fn sync_event_log(&mut self) -> Promise {
        let server_url = self.server_url.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            match fetch_events_from_server(&server_url).await {
                Ok(events) => {
                    let sync_result = SyncResult {
                        events_pulled: events.len() as u32,
                        success: true,
                        error_message: None,
                    };
                    Ok(JsValue::from(sync_result))
                }
                Err(e) => {
                    let sync_result = SyncResult {
                        events_pulled: 0,
                        success: false,
                        error_message: Some(e),
                    };
                    Ok(JsValue::from(sync_result))
                }
            }
        })
    }
}

/// Fetch events from server via HTTP
async fn fetch_events_from_server(server_url: &str) -> Result<Vec<Event>, String> {
    let window = web_sys::window().ok_or("No global window object")?;

    let url = format!("{}/events", server_url);
    log!("Fetching events from: {}", url);

    let opts = RequestInit::new();
    opts.set_method("GET");

    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(|_| "Failed to create request")?;

    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|_| "Failed to set headers")?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Fetch request failed")?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response conversion failed")?;

    if !resp.ok() {
        log!("HTTP error: {} for URL: {}", resp.status(), url);
        return Err(format!("HTTP error: {} for URL: {}", resp.status(), url));
    }

    let text = JsFuture::from(resp.text().map_err(|_| "Failed to get response text")?)
        .await
        .map_err(|_| "Failed to read response text")?;

    let response_text = text.as_string().unwrap_or_default();
    log!(
        "Server response: {}",
        if response_text.len() > 200 {
            format!("{}...", &response_text[..200])
        } else {
            response_text.clone()
        }
    );

    #[derive(Deserialize)]
    struct ServerResponse {
        events: Vec<ServerEvent>,
    }

    #[derive(Deserialize)]
    struct ServerEvent {
        id: String,
        event_type: String,
        aggregate_id: String,
        payload: serde_json::Value,
        timestamp: i64,
        version: i64,
    }

    let server_response: ServerResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse server response: {}", e))?;

    let events: Vec<Event> = server_response
        .events
        .into_iter()
        .map(|se| Event {
            id: se.id,
            event_type: se.event_type,
            aggregate_id: se.aggregate_id,
            payload: se.payload,
            timestamp: se.timestamp,
            version: se.version,
        })
        .collect();

    log!("Fetched {} events from server", events.len());
    Ok(events)
}

// Helper functions for JavaScript

#[wasm_bindgen]
pub fn current_timestamp() -> f64 {
    eventbook_core::current_timestamp() as f64
}

#[wasm_bindgen]
pub fn generate_event_id() -> String {
    eventbook_core::generate_event_id()
}

#[wasm_bindgen]
pub fn validate_json_payload(payload: String) -> Result<(), JsError> {
    serde_json::from_str::<serde_json::Value>(&payload)
        .map_err(|e| JsError::new(&format!("Invalid JSON: {}", e)))?;
    Ok(())
}

/// Create sample user payload for testing
#[wasm_bindgen]
pub fn create_sample_user_payload(name: String, email: String) -> String {
    let payload = serde_json::json!({
        "name": name,
        "email": email,
        "created_at": current_timestamp()
    });

    serde_json::to_string(&payload).unwrap_or_default()
}

/// Test the materializer with sample events
#[wasm_bindgen]
pub fn test_materializer() -> js_sys::Array {
    let timestamp = current_timestamp() as i64;
    let user_id = format!("test-user-{}", timestamp);

    let events = vec![
        Event {
            id: generate_event_id(),
            event_type: "UserCreated".to_string(),
            aggregate_id: user_id.clone(),
            payload: serde_json::json!({
                "name": "Test User",
                "email": "test@example.com"
            }),
            timestamp,
            version: 1,
        },
        Event {
            id: generate_event_id(),
            event_type: "UserUpdated".to_string(),
            aggregate_id: user_id,
            payload: serde_json::json!({
                "name": "Updated Test User",
                "email": "updated@example.com"
            }),
            timestamp: timestamp + 1000,
            version: 2,
        },
    ];

    // Create projection and materialize
    let mut projection = UserProjection::new();
    let _ = projection.rebuild_from_events(&events);

    // Return materialized users
    let users = projection.get_active_users();
    let js_array = js_sys::Array::new();

    for user in users {
        let js_user = JsUser::from(user.clone());
        js_array.push(&JsValue::from(js_user));
    }

    js_array
}

// Log a greeting from WASM
#[wasm_bindgen]
pub fn greet(name: &str) {
    log!("Hello from EventBook WASM, {}! 🦀", name);
}
