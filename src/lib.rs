use napi::bindgen_prelude::{Error, Result, Status};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub aggregate_id: String,
    pub payload: String, // JSON string
    pub timestamp: i64,
    pub version: i64,
}

#[napi]
pub struct EventStore {
    events: Vec<Event>,
    db_path: Option<String>,
}

#[napi]
impl EventStore {
    #[napi(constructor)]
    pub fn new() -> Self {
        EventStore {
            events: Vec::new(),
            db_path: None,
        }
    }

    #[napi]
    pub fn init(&mut self, db_path: String) -> Result<()> {
        self.db_path = Some(db_path);
        // For now, just store in memory. We'll add real Turso integration later
        Ok(())
    }

    #[napi]
    pub fn append_event(&mut self, event: Event) -> Result<()> {
        // Check for duplicate IDs
        if self.events.iter().any(|e| e.id == event.id) {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Event with ID {} already exists", event.id),
            ));
        }

        // Check version ordering for the aggregate
        if let Some(latest) = self
            .events
            .iter()
            .filter(|e| e.aggregate_id == event.aggregate_id)
            .max_by_key(|e| e.version)
        {
            if event.version != latest.version + 1 {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!(
                        "Invalid version {}. Expected {}",
                        event.version,
                        latest.version + 1
                    ),
                ));
            }
        } else if event.version != 1 {
            return Err(Error::new(
                Status::InvalidArg,
                "First event for aggregate must have version 1".to_string(),
            ));
        }

        self.events.push(event);
        Ok(())
    }

    #[napi]
    pub fn get_event_log(&self, aggregate_id: Option<String>) -> Vec<Event> {
        match aggregate_id {
            Some(id) => self
                .events
                .iter()
                .filter(|e| e.aggregate_id == id)
                .cloned()
                .collect(),
            None => self.events.clone(),
        }
    }

    #[napi]
    pub fn get_latest_version(&self, aggregate_id: String) -> i64 {
        self.events
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .map(|e| e.version)
            .max()
            .unwrap_or(0)
    }

    #[napi]
    pub fn get_event_count(&self) -> u32 {
        self.events.len() as u32
    }
}

// Helper functions
#[napi]
pub fn create_event(
    id: String,
    event_type: String,
    aggregate_id: String,
    payload: String,
    timestamp: i64,
    version: i64,
) -> Event {
    Event {
        id,
        event_type,
        aggregate_id,
        payload,
        timestamp,
        version,
    }
}

#[napi]
pub fn get_current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[napi]
pub fn generate_uuid() -> String {
    // Simple UUID generation for now
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("event-{}", timestamp)
}
