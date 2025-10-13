use crate::{Event, EventError, EventResult, Materializer, Projection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single cell in a document, aligned with anode schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub id: String,
    pub cell_type: CellType,
    pub source: String,
    pub fractional_index: Option<String>, // Fractional index for deterministic ordering

    // Execution state
    pub execution_count: Option<u64>,
    pub execution_state: ExecutionState,
    pub assigned_runtime_session: Option<String>,
    pub last_execution_duration_ms: Option<u64>,

    // Cell type specific fields
    pub sql_connection_id: Option<String>,
    pub sql_result_variable: Option<String>,

    // AI-specific fields
    pub ai_provider: Option<String>, // 'openai', 'anthropic', 'local'
    pub ai_model: Option<String>,
    pub ai_settings: Option<serde_json::Value>,

    // Display visibility controls
    pub source_visible: bool,
    pub output_visible: bool,
    pub ai_context_visible: bool,

    pub created_by: String,
    pub document_id: String, // Track which document this cell belongs to
    pub created_at: i64,
    pub updated_at: i64,
}

/// Cell types supported in the document engine, matching anode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
    Sql,
    Ai,
    Raw,
}

/// Execution states for cells
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionState {
    Idle,
    Queued,
    Running,
    Completed,
    Error,
}

impl Default for ExecutionState {
    fn default() -> Self {
        ExecutionState::Idle
    }
}

/// Output types matching anode schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    MultimediaDisplay,
    MultimediaResult,
    Terminal,
    Markdown,
    Error,
}

/// Media representation for unified output system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MediaRepresentation {
    #[serde(rename = "inline")]
    Inline {
        data: serde_json::Value,
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
    #[serde(rename = "artifact")]
    Artifact {
        artifact_id: String,
        metadata: Option<HashMap<String, serde_json::Value>>,
    },
}

/// Cell output with rich media support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellOutput {
    pub id: String,
    pub cell_id: String,
    pub output_type: OutputType,
    pub position: f64,

    // Type-specific fields
    pub stream_name: Option<String>, // 'stdout', 'stderr' for terminal outputs
    pub execution_count: Option<u64>, // Only for multimedia_result
    pub display_id: Option<String>,  // Only for multimedia_display

    // Flattened content for primary access
    pub data: Option<String>,                // Primary/concatenated content
    pub artifact_id: Option<String>,         // Primary artifact reference
    pub mime_type: Option<String>,           // Primary mime type
    pub metadata: Option<serde_json::Value>, // Primary metadata

    // Multi-media support
    pub representations: Option<HashMap<String, MediaRepresentation>>,

    pub created_at: i64,
}

/// Document metadata matching anode's notebook metadata concept
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub kernel_spec: Option<KernelSpec>,
    pub language_info: Option<LanguageInfo>,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    pub custom: HashMap<String, String>, // Key-value metadata storage
}

/// Kernel specification for code execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    pub language: String,
}

/// Language-specific information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub mimetype: Option<String>,
    pub file_extension: Option<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            kernel_spec: None,
            language_info: None,
            authors: Vec::new(),
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// Document containing cells with fractional indexing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub metadata: DocumentMetadata,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Runtime session for execution management
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSession {
    pub session_id: String,
    pub runtime_id: String,
    pub runtime_type: String,
    pub status: RuntimeStatus,
    pub is_active: bool,

    // Capability flags
    pub can_execute_code: bool,
    pub can_execute_sql: bool,
    pub can_execute_ai: bool,
    pub available_ai_models: Option<Vec<String>>,

    pub last_renewed_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeStatus {
    Starting,
    Ready,
    Busy,
    Restarting,
    Terminated,
}

/// State for the Document projection
#[derive(Debug, Clone, Default)]
pub struct DocumentProjectionState {
    pub documents: HashMap<String, Document>,
    pub cells: HashMap<String, Cell>,
    pub outputs: HashMap<String, CellOutput>,
    pub runtime_sessions: HashMap<String, RuntimeSession>,
    pub last_processed_timestamp: i64,
}

impl DocumentProjectionState {
    /// Get all cells for a specific document ordered by fractional index
    pub fn get_document_cells(&self, document_id: &str) -> Vec<&Cell> {
        let mut cells: Vec<&Cell> = self
            .cells
            .values()
            .filter(|cell| cell.document_id == document_id)
            .collect();

        // Sort by fractional index
        cells.sort_by(|a, b| match (&a.fractional_index, &b.fractional_index) {
            (Some(a_idx), Some(b_idx)) => a_idx.cmp(b_idx),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.created_at.cmp(&b.created_at),
        });

        cells
    }

    /// Get outputs for a specific cell
    pub fn get_cell_outputs(&self, cell_id: &str) -> Vec<&CellOutput> {
        let mut outputs: Vec<&CellOutput> = self
            .outputs
            .values()
            .filter(|output| output.cell_id == cell_id)
            .collect();

        outputs.sort_by(|a, b| {
            a.position
                .partial_cmp(&b.position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        outputs
    }
}

/// Materializer for Document events
pub struct DocumentMaterializer;

impl Materializer for DocumentMaterializer {
    type State = DocumentProjectionState;
    type Error = EventError;

    fn initial_state() -> Self::State {
        DocumentProjectionState::default()
    }

    fn apply_event(state: &Self::State, event: &Event) -> Result<Self::State, Self::Error> {
        let mut new_state = state.clone();
        new_state.last_processed_timestamp = event.timestamp;

        match event.event_type.as_str() {
            "DocumentCreated" => {
                let document = Document {
                    id: event.aggregate_id.clone(),
                    title: event
                        .payload
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled")
                        .to_string(),
                    metadata: serde_json::from_value(
                        event.payload.get("metadata").cloned().unwrap_or_default(),
                    )
                    .unwrap_or_default(),
                    created_at: event.timestamp,
                    updated_at: event.timestamp,
                };
                new_state
                    .documents
                    .insert(event.aggregate_id.clone(), document);
            }

            "DocumentTitleUpdated" => {
                if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                    if let Some(title) = event.payload.get("title").and_then(|v| v.as_str()) {
                        document.title = title.to_string();
                        document.updated_at = event.timestamp;
                    }
                }
            }

            "DocumentMetadataUpdated" => {
                if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                    if let Some(metadata) = event.payload.get("metadata") {
                        document.metadata = serde_json::from_value(metadata.clone())
                            .unwrap_or_else(|_| document.metadata.clone());
                        document.updated_at = event.timestamp;
                    }
                }
            }

            "CellCreated" => {
                let cell_data = &event.payload;
                let cell_id = cell_data
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                let cell_type_str = cell_data
                    .get("cell_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_type".to_string()))?;

                let cell_type = match cell_type_str {
                    "code" => CellType::Code,
                    "markdown" => CellType::Markdown,
                    "sql" => CellType::Sql,
                    "ai" => CellType::Ai,
                    "raw" => CellType::Raw,
                    _ => {
                        return Err(EventError::ValidationError(format!(
                            "Invalid cell_type: {}",
                            cell_type_str
                        )))
                    }
                };

                let cell = Cell {
                    id: cell_id.to_string(),
                    cell_type,
                    source: cell_data
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    fractional_index: cell_data
                        .get("fractional_index")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    execution_count: cell_data.get("execution_count").and_then(|v| v.as_u64()),
                    execution_state: ExecutionState::default(),
                    assigned_runtime_session: None,
                    last_execution_duration_ms: None,
                    sql_connection_id: cell_data
                        .get("sql_connection_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    sql_result_variable: cell_data
                        .get("sql_result_variable")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ai_provider: cell_data
                        .get("ai_provider")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ai_model: cell_data
                        .get("ai_model")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    ai_settings: cell_data.get("ai_settings").cloned(),
                    source_visible: cell_data
                        .get("source_visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    output_visible: cell_data
                        .get("output_visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    ai_context_visible: cell_data
                        .get("ai_context_visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    created_by: cell_data
                        .get("created_by")
                        .and_then(|v| v.as_str())
                        .unwrap_or("system")
                        .to_string(),
                    document_id: event.aggregate_id.clone(), // Store document association
                    created_at: event.timestamp,
                    updated_at: event.timestamp,
                };

                new_state.cells.insert(cell_id.to_string(), cell);

                // Update document timestamp
                if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                    document.updated_at = event.timestamp;
                }
            }

            "CellSourceUpdated" => {
                let cell_id = event
                    .payload
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                if let Some(cell) = new_state.cells.get_mut(cell_id) {
                    if let Some(source) = event.payload.get("source").and_then(|v| v.as_str()) {
                        cell.source = source.to_string();
                    }
                    cell.updated_at = event.timestamp;

                    // Update document timestamp
                    if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                        document.updated_at = event.timestamp;
                    }
                }
            }

            "CellExecutionStateChanged" => {
                let cell_id = event
                    .payload
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                if let Some(cell) = new_state.cells.get_mut(cell_id) {
                    if let Some(state_str) = event
                        .payload
                        .get("execution_state")
                        .and_then(|v| v.as_str())
                    {
                        cell.execution_state = match state_str {
                            "idle" => ExecutionState::Idle,
                            "queued" => ExecutionState::Queued,
                            "running" => ExecutionState::Running,
                            "completed" => ExecutionState::Completed,
                            "error" => ExecutionState::Error,
                            _ => cell.execution_state.clone(),
                        };
                    }

                    if let Some(runtime_session) = event
                        .payload
                        .get("assigned_runtime_session")
                        .and_then(|v| v.as_str())
                    {
                        cell.assigned_runtime_session = Some(runtime_session.to_string());
                    }

                    if let Some(duration) = event
                        .payload
                        .get("execution_duration_ms")
                        .and_then(|v| v.as_u64())
                    {
                        cell.last_execution_duration_ms = Some(duration);
                    }

                    cell.updated_at = event.timestamp;
                }
            }

            "CellOutputCreated" => {
                let output_data = &event.payload;
                let output_id = output_data
                    .get("output_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing output_id".to_string()))?;

                let cell_id = output_data
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                let output_type_str = output_data
                    .get("output_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        EventError::ValidationError("Missing output_type".to_string())
                    })?;

                let output_type = match output_type_str {
                    "multimedia_display" => OutputType::MultimediaDisplay,
                    "multimedia_result" => OutputType::MultimediaResult,
                    "terminal" => OutputType::Terminal,
                    "markdown" => OutputType::Markdown,
                    "error" => OutputType::Error,
                    _ => {
                        return Err(EventError::ValidationError(format!(
                            "Invalid output_type: {}",
                            output_type_str
                        )))
                    }
                };

                let output = CellOutput {
                    id: output_id.to_string(),
                    cell_id: cell_id.to_string(),
                    output_type,
                    position: output_data
                        .get("position")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    stream_name: output_data
                        .get("stream_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    execution_count: output_data.get("execution_count").and_then(|v| v.as_u64()),
                    display_id: output_data
                        .get("display_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    data: output_data
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    artifact_id: output_data
                        .get("artifact_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    mime_type: output_data
                        .get("mime_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    metadata: output_data.get("metadata").cloned(),
                    representations: output_data
                        .get("representations")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    created_at: event.timestamp,
                };

                new_state.outputs.insert(output_id.to_string(), output);
            }

            "CellMoved" => {
                let cell_id = event
                    .payload
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                let new_fractional_index = event
                    .payload
                    .get("fractional_index")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        EventError::ValidationError("Missing fractional_index".to_string())
                    })?;

                if let Some(cell) = new_state.cells.get_mut(cell_id) {
                    cell.fractional_index = Some(new_fractional_index.to_string());
                    cell.updated_at = event.timestamp;

                    // Update document timestamp
                    if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                        document.updated_at = event.timestamp;
                    }
                }
            }

            "CellDeleted" => {
                let cell_id = event
                    .payload
                    .get("cell_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| EventError::ValidationError("Missing cell_id".to_string()))?;

                // Remove cell and its outputs
                new_state.cells.remove(cell_id);
                new_state
                    .outputs
                    .retain(|_, output| output.cell_id != cell_id);

                // Update document timestamp
                if let Some(document) = new_state.documents.get_mut(&event.aggregate_id) {
                    document.updated_at = event.timestamp;
                }
            }

            "DocumentDeleted" => {
                // Remove document and all associated cells/outputs
                new_state.documents.remove(&event.aggregate_id);

                // For proper cleanup, we'd need to track which cells belong to which document
                // This could be done by storing document_id in cells or using aggregate relationships
            }

            _ => {
                // Unknown event type, ignore
            }
        }

        Ok(new_state)
    }

    fn handles_event_type(event_type: &str) -> bool {
        matches!(
            event_type,
            "DocumentCreated"
                | "DocumentTitleUpdated"
                | "DocumentMetadataUpdated"
                | "CellCreated"
                | "CellSourceUpdated"
                | "CellExecutionStateChanged"
                | "CellOutputCreated"
                | "CellMoved"
                | "CellDeleted"
                | "DocumentDeleted"
        )
    }
}

/// Document projection implementation
pub struct DocumentProjection {
    state: DocumentProjectionState,
}

impl DocumentProjection {
    pub fn new() -> Self {
        Self {
            state: DocumentMaterializer::initial_state(),
        }
    }

    /// Get all documents
    pub fn get_documents(&self) -> Vec<&Document> {
        self.state.documents.values().collect()
    }

    /// Get a specific document by ID
    pub fn get_document(&self, document_id: &str) -> Option<&Document> {
        self.state.documents.get(document_id)
    }

    /// Get all cells for a document ordered by fractional index
    pub fn get_document_cells(&self, document_id: &str) -> Vec<&Cell> {
        self.state.get_document_cells(document_id)
    }

    /// Get a specific cell by ID
    pub fn get_cell(&self, cell_id: &str) -> Option<&Cell> {
        self.state.cells.get(cell_id)
    }

    /// Get outputs for a specific cell
    pub fn get_cell_outputs(&self, cell_id: &str) -> Vec<&CellOutput> {
        self.state.get_cell_outputs(cell_id)
    }

    /// Get the number of documents
    pub fn document_count(&self) -> usize {
        self.state.documents.len()
    }

    /// Get the total number of cells across all documents
    pub fn total_cell_count(&self) -> usize {
        self.state.cells.len()
    }
}

impl Default for DocumentProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl Projection for DocumentProjection {
    type State = DocumentProjectionState;

    fn rebuild_from_events(&mut self, events: &[Event]) -> EventResult<()> {
        let mut state = DocumentMaterializer::initial_state();

        for event in events {
            if DocumentMaterializer::handles_event_type(&event.event_type) {
                state = DocumentMaterializer::apply_event(&state, event).map_err(|e| {
                    EventError::ValidationError(format!("Materialization failed: {}", e))
                })?;
            }
        }

        self.state = state;
        Ok(())
    }

    fn get_state(&self) -> &Self::State {
        &self.state
    }

    fn last_processed_timestamp(&self) -> i64 {
        self.state.last_processed_timestamp
    }

    fn apply_new_events(&mut self, events: &[Event]) -> EventResult<()> {
        for event in events {
            if event.timestamp > self.state.last_processed_timestamp
                && DocumentMaterializer::handles_event_type(&event.event_type)
            {
                self.state =
                    DocumentMaterializer::apply_event(&self.state, event).map_err(|e| {
                        EventError::ValidationError(format!("Materialization failed: {}", e))
                    })?;
            }
        }
        Ok(())
    }
}

/// Utility functions for creating document events

/// Create a new document
pub fn create_document_event(
    document_id: String,
    title: String,
    metadata: DocumentMetadata,
    version: i64,
) -> EventResult<Event> {
    use crate::EventBuilder;

    EventBuilder::new()
        .event_type("DocumentCreated")
        .aggregate_id(document_id)
        .payload(serde_json::json!({
            "title": title,
            "metadata": metadata
        }))?
        .build(version)
}

/// Create a new cell with fractional indexing
pub fn create_cell_event(
    document_id: String,
    cell_id: String,
    cell_type: CellType,
    source: String,
    fractional_index: Option<String>,
    created_by: String,
    version: i64,
) -> EventResult<Event> {
    use crate::EventBuilder;

    let mut payload = serde_json::json!({
        "cell_id": cell_id,
        "cell_type": match cell_type {
            CellType::Code => "code",
            CellType::Markdown => "markdown",
            CellType::Sql => "sql",
            CellType::Ai => "ai",
            CellType::Raw => "raw",
        },
        "source": source,
        "created_by": created_by
    });

    if let Some(index) = fractional_index {
        payload["fractional_index"] = serde_json::Value::String(index);
    }

    EventBuilder::new()
        .event_type("CellCreated")
        .aggregate_id(document_id)
        .payload(payload)?
        .build(version)
}

/// Update a cell's source code
pub fn update_cell_source_event(
    document_id: String,
    cell_id: String,
    source: String,
    version: i64,
) -> EventResult<Event> {
    use crate::EventBuilder;

    EventBuilder::new()
        .event_type("CellSourceUpdated")
        .aggregate_id(document_id)
        .payload(serde_json::json!({
            "cell_id": cell_id,
            "source": source
        }))?
        .build(version)
}

/// Move a cell using fractional indexing
pub fn move_cell_event(
    document_id: String,
    cell_id: String,
    fractional_index: String,
    version: i64,
) -> EventResult<Event> {
    use crate::EventBuilder;

    EventBuilder::new()
        .event_type("CellMoved")
        .aggregate_id(document_id)
        .payload(serde_json::json!({
            "cell_id": cell_id,
            "fractional_index": fractional_index
        }))?
        .build(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let event = create_document_event(
            "doc-123".to_string(),
            "My Document".to_string(),
            DocumentMetadata::default(),
            1,
        )
        .unwrap();

        assert_eq!(event.event_type, "DocumentCreated");
        assert_eq!(event.aggregate_id, "doc-123");
    }

    #[test]
    fn test_cell_creation() {
        let event = create_cell_event(
            "doc-123".to_string(),
            "cell-1".to_string(),
            CellType::Code,
            "print('hello')".to_string(),
            Some("a0".to_string()),
            "user-1".to_string(),
            1,
        )
        .unwrap();

        assert_eq!(event.event_type, "CellCreated");
        assert_eq!(event.aggregate_id, "doc-123");
    }

    #[test]
    fn test_document_projection() {
        let mut projection = DocumentProjection::new();

        let doc_event = create_document_event(
            "doc-123".to_string(),
            "Test Document".to_string(),
            DocumentMetadata::default(),
            1,
        )
        .unwrap();

        let cell_event = create_cell_event(
            "doc-123".to_string(),
            "cell-1".to_string(),
            CellType::Code,
            "print('hello')".to_string(),
            Some("a0".to_string()),
            "user-1".to_string(),
            2,
        )
        .unwrap();

        projection
            .rebuild_from_events(&[doc_event, cell_event])
            .unwrap();

        let document = projection.get_document("doc-123").unwrap();
        assert_eq!(document.title, "Test Document");

        let cell = projection.get_cell("cell-1").unwrap();
        assert_eq!(cell.source, "print('hello')");
        assert_eq!(cell.fractional_index, Some("a0".to_string()));
        assert_eq!(cell.document_id, "doc-123");

        // Test that document cells are properly associated
        let document_cells = projection.get_document_cells("doc-123");
        assert_eq!(document_cells.len(), 1);
        assert_eq!(document_cells[0].id, "cell-1");
    }
}
