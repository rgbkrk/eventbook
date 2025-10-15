use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod document;
pub mod fractional_index;

/// Core event structure for event sourcing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
    pub version: i64,
}

/// Result type for event operations
pub type EventResult<T> = Result<T, EventError>;

/// Errors that can occur in event operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventError {
    InvalidVersion { expected: i64, got: i64 },
    DuplicateEventId(String),
    InvalidEventType(String),
    InvalidAggregateId(String),
    SerializationError(String),
    ValidationError(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::InvalidVersion { expected, got } => {
                write!(f, "Invalid version: expected {}, got {}", expected, got)
            }
            EventError::DuplicateEventId(id) => write!(f, "Duplicate event ID: {}", id),
            EventError::InvalidEventType(t) => write!(f, "Invalid event type: {}", t),
            EventError::InvalidAggregateId(id) => write!(f, "Invalid aggregate ID: {}", id),
            EventError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            EventError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for EventError {}

/// Trait for event store implementations
pub trait EventStore {
    /// Append an event to the store
    fn append_event(&mut self, event: Event) -> EventResult<()>;

    /// Get all events for a specific aggregate
    fn get_events(&self, aggregate_id: &str) -> EventResult<Vec<Event>>;

    /// Get all events in the store
    fn get_all_events(&self) -> EventResult<Vec<Event>>;

    /// Get the latest version for an aggregate
    fn get_latest_version(&self, aggregate_id: &str) -> i64;

    /// Get total event count
    fn get_event_count(&self) -> usize;
}

/// Trait for materializing events into projections/views
pub trait Materializer {
    type State: Clone;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get the initial state of this materializer
    fn initial_state() -> Self::State;

    /// Apply an event to the current state, returning the new state
    fn apply_event(state: &Self::State, event: &Event) -> Result<Self::State, Self::Error>;

    /// Check if this materializer cares about a specific event type
    fn handles_event_type(event_type: &str) -> bool;
}

/// Trait for managing materialized projections
pub trait Projection {
    type State: Clone;

    /// Rebuild the projection from a sequence of events
    fn rebuild_from_events(&mut self, events: &[Event]) -> EventResult<()>;

    /// Get the current materialized state
    fn get_state(&self) -> &Self::State;

    /// Get the last processed event timestamp (for incremental updates)
    fn last_processed_timestamp(&self) -> i64;

    /// Apply new events since the last processed timestamp
    fn apply_new_events(&mut self, events: &[Event]) -> EventResult<()>;
}

/// Builder for creating events with validation
#[derive(Debug, Clone)]
pub struct EventBuilder {
    event_type: Option<String>,
    aggregate_id: Option<String>,
    payload: serde_json::Value,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self {
            event_type: None,
            aggregate_id: None,
            payload: serde_json::Value::Null,
        }
    }

    pub fn event_type<S: Into<String>>(mut self, event_type: S) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn aggregate_id<S: Into<String>>(mut self, aggregate_id: S) -> Self {
        self.aggregate_id = Some(aggregate_id.into());
        self
    }

    pub fn payload<T: Serialize>(mut self, payload: T) -> EventResult<Self> {
        self.payload = serde_json::to_value(payload)
            .map_err(|e| EventError::SerializationError(e.to_string()))?;
        Ok(self)
    }

    pub fn build(self, version: i64) -> EventResult<Event> {
        let event_type = self
            .event_type
            .ok_or_else(|| EventError::ValidationError("Event type is required".to_string()))?;
        let aggregate_id = self
            .aggregate_id
            .ok_or_else(|| EventError::ValidationError("Aggregate ID is required".to_string()))?;

        // Basic validation
        if event_type.trim().is_empty() {
            return Err(EventError::InvalidEventType(event_type));
        }
        if aggregate_id.trim().is_empty() {
            return Err(EventError::InvalidAggregateId(aggregate_id));
        }
        if version < 1 {
            return Err(EventError::InvalidVersion {
                expected: 1,
                got: version,
            });
        }

        Ok(Event {
            id: generate_event_id(),
            event_type,
            aggregate_id,
            payload: self.payload,
            timestamp: current_timestamp(),
            version,
        })
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory event store implementation for testing and simple use cases
#[derive(Debug, Clone)]
pub struct InMemoryEventStore {
    events: Vec<Event>,
    version_map: HashMap<String, i64>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            version_map: HashMap::new(),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append_event(&mut self, event: Event) -> EventResult<()> {
        // Check for duplicate event ID
        if self.events.iter().any(|e| e.id == event.id) {
            return Err(EventError::DuplicateEventId(event.id));
        }

        // Check version ordering
        let current_version = self.get_latest_version(&event.aggregate_id);
        let expected_version = current_version + 1;

        if event.version != expected_version {
            return Err(EventError::InvalidVersion {
                expected: expected_version,
                got: event.version,
            });
        }

        // Update version map
        self.version_map
            .insert(event.aggregate_id.clone(), event.version);

        // Store event
        self.events.push(event);
        Ok(())
    }

    fn get_events(&self, aggregate_id: &str) -> EventResult<Vec<Event>> {
        let mut events: Vec<Event> = self
            .events
            .iter()
            .filter(|e| e.aggregate_id == aggregate_id)
            .cloned()
            .collect();
        events.sort_by_key(|e| e.version);
        Ok(events)
    }

    fn get_all_events(&self) -> EventResult<Vec<Event>> {
        let mut events = self.events.clone();
        events.sort_by_key(|e| (e.timestamp, e.version));
        Ok(events)
    }

    fn get_latest_version(&self, aggregate_id: &str) -> i64 {
        self.version_map.get(aggregate_id).copied().unwrap_or(0)
    }

    fn get_event_count(&self) -> usize {
        self.events.len()
    }
}

/// Generate a unique event ID
pub fn generate_event_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("event-{}", timestamp)
}

/// Get current timestamp as Unix epoch seconds
pub fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Validate event structure
pub fn validate_event(event: &Event) -> EventResult<()> {
    if event.event_type.trim().is_empty() {
        return Err(EventError::InvalidEventType(event.event_type.clone()));
    }
    if event.aggregate_id.trim().is_empty() {
        return Err(EventError::InvalidAggregateId(event.aggregate_id.clone()));
    }
    if event.version < 1 {
        return Err(EventError::InvalidVersion {
            expected: 1,
            got: event.version,
        });
    }
    Ok(())
}

// Re-export document types
pub use document::{
    create_cell_event, create_document_event, move_cell_event, update_cell_source_event, Cell,
    CellOutput, CellType, Document, DocumentMaterializer, DocumentMetadata, DocumentProjection,
    DocumentProjectionState, ExecutionState, KernelSpec, LanguageInfo, MediaRepresentation,
    OutputType, RuntimeSession, RuntimeStatus,
};

// Re-export fractional index utilities
pub use fractional_index::{
    after as fractional_after, before as fractional_before, between as fractional_between,
    generate_sequence as fractional_generate_sequence, initial as fractional_initial,
    is_valid_order as fractional_is_valid_order, validate_index as fractional_validate_index,
    FractionalIndexError,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new()
            .event_type("CellCreated")
            .aggregate_id("cell-123")
            .payload(serde_json::json!({"source": "print('hello')"}))
            .unwrap()
            .build(1)
            .unwrap();

        assert_eq!(event.event_type, "CellCreated");
        assert_eq!(event.aggregate_id, "cell-123");
        assert_eq!(event.version, 1);
    }

    #[test]
    fn test_in_memory_store() {
        let mut store = InMemoryEventStore::new();

        let event = EventBuilder::new()
            .event_type("CellCreated")
            .aggregate_id("cell-123")
            .payload(serde_json::json!({"source": "print('hello')"}))
            .unwrap()
            .build(1)
            .unwrap();

        store.append_event(event.clone()).unwrap();

        let events = store.get_events("cell-123").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);

        assert_eq!(store.get_latest_version("cell-123"), 1);
    }

    #[test]
    fn test_version_validation() {
        let mut store = InMemoryEventStore::new();

        let event1 = EventBuilder::new()
            .event_type("CellCreated")
            .aggregate_id("cell-123")
            .payload(serde_json::json!({"source": "print('hello')"}))
            .unwrap()
            .build(1)
            .unwrap();

        let event2 = EventBuilder::new()
            .event_type("CellSourceUpdated")
            .aggregate_id("cell-123")
            .payload(serde_json::json!({"source": "print('world')"}))
            .unwrap()
            .build(3) // Wrong version - should be 2
            .unwrap();

        store.append_event(event1).unwrap();
        let result = store.append_event(event2);

        assert!(matches!(
            result,
            Err(EventError::InvalidVersion {
                expected: 2,
                got: 3
            })
        ));
    }
}
