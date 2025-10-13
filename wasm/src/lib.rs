use eventbook_core::{Event, EventBuilder, EventStore, InMemoryEventStore};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// NAPI-compatible Event type for JavaScript interop
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsEvent {
    pub id: String,
    pub event_type: String,
    pub aggregate_id: String,
    pub payload: String, // JSON string for simplicity
    pub timestamp: i64,
    pub version: i64,
}

impl From<Event> for JsEvent {
    fn from(event: Event) -> Self {
        Self {
            id: event.id,
            event_type: event.event_type,
            aggregate_id: event.aggregate_id,
            payload: serde_json::to_string(&event.payload).unwrap_or_default(),
            timestamp: event.timestamp,
            version: event.version,
        }
    }
}

/// Convert JsEvent back to Event (fallible)
impl JsEvent {
    pub fn to_event(&self) -> Result<Event> {
        let payload = serde_json::from_str(&self.payload)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid JSON payload: {}", e)))?;

        Ok(Event {
            id: self.id.clone(),
            event_type: self.event_type.clone(),
            aggregate_id: self.aggregate_id.clone(),
            payload,
            timestamp: self.timestamp,
            version: self.version,
        })
    }
}

/// Configuration for the client
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub sync_enabled: Option<bool>,
}

/// Sync operation result
#[napi(object)]
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub events_pushed: u32,
    pub events_pulled: u32,
    pub success: bool,
    pub error: Option<String>,
}

/// Client-side event store with server synchronization capability
#[napi]
pub struct EventBookClient {
    local_store: InMemoryEventStore,
    server_url: String,
    sync_enabled: bool,
}

#[napi]
impl EventBookClient {
    #[napi(constructor)]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            local_store: InMemoryEventStore::new(),
            server_url: config.server_url,
            sync_enabled: config.sync_enabled.unwrap_or(true),
        }
    }

    /// Submit an event locally
    #[napi]
    pub fn submit_event(
        &mut self,
        event_type: String,
        aggregate_id: String,
        payload: String,
    ) -> Result<JsEvent> {
        // Parse payload
        let payload_value: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid JSON payload: {}", e)))?;

        // Get next version
        let current_version = self.local_store.get_latest_version(&aggregate_id);
        let next_version = current_version + 1;

        // Build event
        let event = EventBuilder::new()
            .event_type(event_type)
            .aggregate_id(aggregate_id)
            .payload(payload_value)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Payload error: {}", e)))?
            .build(next_version)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Event build error: {}", e)))?;

        // Store locally
        self.local_store
            .append_event(event.clone())
            .map_err(|e| Error::new(Status::InvalidArg, format!("Store error: {}", e)))?;

        Ok(event.into())
    }

    /// Get events from local store
    #[napi]
    pub fn get_events(&self, aggregate_id: Option<String>) -> Result<Vec<JsEvent>> {
        let events = match aggregate_id {
            Some(id) => self.local_store.get_events(&id),
            None => self.local_store.get_all_events(),
        }
        .map_err(|e| Error::new(Status::GenericFailure, format!("Get events error: {}", e)))?;

        Ok(events.into_iter().map(JsEvent::from).collect())
    }

    /// Get the latest version for an aggregate
    #[napi]
    pub fn get_latest_version(&self, aggregate_id: String) -> i64 {
        self.local_store.get_latest_version(&aggregate_id)
    }

    /// Get total event count in local store
    #[napi]
    pub fn get_event_count(&self) -> u32 {
        self.local_store.get_event_count() as u32
    }

    /// Enable or disable server synchronization
    #[napi]
    pub fn set_sync_enabled(&mut self, enabled: bool) {
        self.sync_enabled = enabled;
    }

    /// Check if sync is enabled
    #[napi]
    pub fn is_sync_enabled(&self) -> bool {
        self.sync_enabled
    }

    /// Get server URL
    #[napi]
    pub fn get_server_url(&self) -> String {
        self.server_url.clone()
    }

    /// Set server URL
    #[napi]
    pub fn set_server_url(&mut self, url: String) {
        self.server_url = url;
    }

    /// Clear local event store (useful for testing)
    #[napi]
    pub fn clear_local_store(&mut self) {
        self.local_store = InMemoryEventStore::new();
    }

    /// Get aggregates summary
    #[napi]
    pub fn get_aggregates_summary(&self) -> Result<Vec<AggregateInfo>> {
        let events = self
            .local_store
            .get_all_events()
            .map_err(|e| Error::new(Status::GenericFailure, format!("Get events error: {}", e)))?;

        let mut aggregates: std::collections::HashMap<String, AggregateInfo> =
            std::collections::HashMap::new();

        for event in events {
            let entry = aggregates
                .entry(event.aggregate_id.clone())
                .or_insert_with(|| AggregateInfo {
                    aggregate_id: event.aggregate_id.clone(),
                    latest_version: 0,
                    event_count: 0,
                    first_event_timestamp: event.timestamp,
                    last_event_timestamp: event.timestamp,
                });

            entry.event_count += 1;
            entry.latest_version = entry.latest_version.max(event.version);
            entry.first_event_timestamp = entry.first_event_timestamp.min(event.timestamp);
            entry.last_event_timestamp = entry.last_event_timestamp.max(event.timestamp);
        }

        Ok(aggregates.into_values().collect())
    }
}

/// Information about an aggregate
#[napi(object)]
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateInfo {
    pub aggregate_id: String,
    pub latest_version: i64,
    pub event_count: u32,
    pub first_event_timestamp: i64,
    pub last_event_timestamp: i64,
}

/// Helper functions for JavaScript

#[napi]
pub fn create_client_config(server_url: String) -> ClientConfig {
    ClientConfig {
        server_url,
        sync_enabled: Some(true),
    }
}

#[napi]
pub fn current_timestamp() -> i64 {
    eventbook_core::current_timestamp()
}

#[napi]
pub fn generate_event_id() -> String {
    eventbook_core::generate_event_id()
}

/// Validate event payload JSON
#[napi]
pub fn validate_json_payload(payload: String) -> Result<bool> {
    match serde_json::from_str::<serde_json::Value>(&payload) {
        Ok(_) => Ok(true),
        Err(e) => Err(Error::new(
            Status::InvalidArg,
            format!("Invalid JSON: {}", e),
        )),
    }
}

/// Create a sample event payload for testing
#[napi]
pub fn create_sample_user_payload(name: String, email: String) -> Result<String> {
    let payload = serde_json::json!({
        "name": name,
        "email": email,
        "created_at": current_timestamp()
    });

    serde_json::to_string(&payload).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Serialization error: {}", e),
        )
    })
}

/// Create a sample event update payload for testing
#[napi]
pub fn create_sample_update_payload(field: String, value: String) -> Result<String> {
    let payload = serde_json::json!({
        field: value,
        "updated_at": current_timestamp()
    });

    serde_json::to_string(&payload).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Serialization error: {}", e),
        )
    })
}
