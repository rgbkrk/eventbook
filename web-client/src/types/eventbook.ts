// Core event structure
export interface Event {
  id: string;
  event_type: string;
  aggregate_id: string;
  payload: Record<string, any>;
  timestamp: number;
  version: number;
}

// Cell types matching our Rust backend
export type CellType = "code" | "markdown" | "sql" | "ai" | "raw";

export type ExecutionState =
  | "idle"
  | "queued"
  | "running"
  | "completed"
  | "error";

export type OutputType =
  | "multimedia_display"
  | "multimedia_result"
  | "terminal"
  | "markdown"
  | "error";

// Cell structure
export interface Cell {
  id: string;
  cell_type: CellType;
  source: string;
  fractional_index?: string;
  execution_count?: number;
  execution_state: ExecutionState;
  assigned_runtime_session?: string;
  last_execution_duration_ms?: number;
  sql_connection_id?: string;
  sql_result_variable?: string;
  ai_provider?: string;
  ai_model?: string;
  ai_settings?: Record<string, any>;
  source_visible: boolean;
  output_visible: boolean;
  ai_context_visible: boolean;
  created_by: string;
  document_id: string;
  created_at: number;
  updated_at: number;
}

// Output structure
export interface CellOutput {
  id: string;
  cell_id: string;
  output_type: OutputType;
  position: number;
  stream_name?: string;
  execution_count?: number;
  display_id?: string;
  data?: string;
  artifact_id?: string;
  mime_type?: string;
  metadata?: Record<string, any>;
  representations?: Record<string, MediaRepresentation>;
  created_at: number;
}

export interface MediaRepresentation {
  type: "inline" | "artifact";
  data?: any;
  artifact_id?: string;
  metadata?: Record<string, any>;
}

// Document/Notebook metadata
export interface KernelSpec {
  name: string;
  display_name: string;
  language: string;
}

export interface LanguageInfo {
  name: string;
  version: string;
  mimetype?: string;
  file_extension?: string;
}

export interface DocumentMetadata {
  kernel_spec?: KernelSpec;
  language_info?: LanguageInfo;
  authors: string[];
  tags: string[];
  custom: Record<string, string>;
}

export interface Document {
  id: string;
  title: string;
  metadata: DocumentMetadata;
  created_at: number;
  updated_at: number;
}

// API Request/Response types
export interface SubmitEventRequest {
  event_type: string;
  payload: Record<string, any>;
}

export interface SubmitEventResponse {
  event_id: string;
  version: number;
}

export interface GetEventsQuery {
  limit?: number;
  offset?: number;
  since_timestamp?: number;
}

export interface GetEventsResponse {
  events: Event[];
  total_count: number;
  store_id: string;
}

export interface StoreInfoResponse {
  store_id: string;
  event_count: number;
  latest_version: number;
  first_event_timestamp?: number;
  last_event_timestamp?: number;
}

// Notebook state (reconstructed from events)
export interface NotebookState {
  document?: Document;
  cells: Record<string, Cell>;
  outputs: Record<string, CellOutput>;
  orderedCells: Cell[];
  lastProcessedTimestamp: number;
  events: Event[];
}

// Event payload types for specific operations
export interface DocumentCreatedPayload {
  title: string;
  metadata: DocumentMetadata;
}

export interface CellCreatedPayload {
  cell_id: string;
  cell_type: CellType;
  source: string;
  fractional_index?: string;
  created_by: string;
}

export interface CellSourceUpdatedPayload {
  cell_id: string;
  source: string;
}

export interface CellExecutionStateChangedPayload {
  cell_id: string;
  execution_state: ExecutionState;
  assigned_runtime_session?: string;
  execution_duration_ms?: number;
}

export interface CellOutputCreatedPayload {
  output_id: string;
  cell_id: string;
  output_type: OutputType;
  position: number;
  stream_name?: string;
  execution_count?: number;
  display_id?: string;
  data?: string;
  artifact_id?: string;
  mime_type?: string;
  metadata?: Record<string, any>;
  representations?: Record<string, MediaRepresentation>;
}

export interface CellMovedPayload {
  cell_id: string;
  fractional_index: string;
}

export interface CellDeletedPayload {
  cell_id: string;
}

// UI state
export interface UIState {
  selectedCellId?: string;
  editingCellId?: string;
  isConnected: boolean;
  isLoading: boolean;
  error?: string;
}

// API client interface
export interface EventBookAPI {
  submitEvent(
    storeId: string,
    request: SubmitEventRequest,
  ): Promise<SubmitEventResponse>;
  getEvents(
    storeId: string,
    query?: GetEventsQuery,
  ): Promise<GetEventsResponse>;
  getStoreInfo(storeId: string): Promise<StoreInfoResponse>;
  listStores(): Promise<string[]>;
}
