# EventBook: Rust Event Sourcing Architecture Plan

## Current State Analysis

EventBook currently has:
- ✅ Rust core with NAPI bindings to TypeScript
- ✅ WebSocket real-time collaboration working
- ✅ React frontend with hook-based state management
- ✅ SQLite storage with basic cell/notebook operations
- ❌ Complex parent state propagation causing render cascades
- ❌ No proper event sourcing or materializers
- ❌ All state management in TypeScript hooks

## Target Architecture: Full Rust Event Sourcing

### Core Principles
1. **Rust-First**: All event sourcing logic, materializers, and queries in Rust
2. **Domain-Specific**: Purpose-built for notebook collaboration, not generic
3. **Fine-Grained Reactivity**: Components subscribe to specific queries, not global state
4. **Server-Client Parity**: Same event types and materializers on both sides
5. **Type Safety**: Full compile-time guarantees from Rust through to TypeScript

### Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   React App     │    │   Rust Server   │    │   Other Clients │
│                 │    │                 │    │                 │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │TypeScript │  │    │  │Event Store│  │    │  │Event Store│  │
│  │Bindings   │  │    │  │+ Runtime  │  │    │  │           │  │
│  └─────┬─────┘  │    │  └───────────┘  │    │  └───────────┘  │
│        │        │    │                 │    │                 │
│  ┌─────▼─────┐  │    │                 │    │                 │
│  │NAPI Bridge│  │    │                 │    │                 │
│  └─────┬─────┘  │    │                 │    │                 │
│        │        │    │                 │    │                 │
│  ┌─────▼─────┐  │    │                 │    │                 │
│  │Rust Core  │  │    │                 │    │                 │
│  │Event Store│  │    │                 │    │                 │
│  └───────────┘  │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────▼─────────────┐
                    │    WebSocket Sync Layer    │
                    │   (Event Broadcasting)     │
                    └───────────────────────────┘
```

## Phase 1: Core Event Sourcing Foundation (Week 1-2)

### 1.1 Define Event Schema in Rust

```rust
// core/src/events.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum NotebookEvent {
    // Notebook-level events
    NotebookInitialized {
        notebook_id: NotebookId,
        title: String,
        created_at: i64,
    },
    NotebookTitleChanged {
        title: String,
        changed_at: i64,
    },
    
    // Cell management events
    CellCreated {
        cell_id: CellId,
        cell_type: CellType,
        fractional_index: String,
        after_cell_id: Option<CellId>,
        created_at: i64,
    },
    CellDeleted {
        cell_id: CellId,
        deleted_at: i64,
    },
    CellMoved {
        cell_id: CellId,
        new_fractional_index: String,
        moved_at: i64,
    },
    
    // Content events
    CellSourceChanged {
        cell_id: CellId,
        source: String,
        changed_at: i64,
        change_id: ChangeId, // For operational transform
    },
    
    // Execution events
    CellExecutionRequested {
        cell_id: CellId,
        execution_id: ExecutionId,
        requested_at: i64,
    },
    CellExecutionStarted {
        cell_id: CellId,
        execution_id: ExecutionId,
        started_at: i64,
    },
    CellExecutionCompleted {
        cell_id: CellId,
        execution_id: ExecutionId,
        outputs: Vec<CellOutput>,
        completed_at: i64,
        duration_ms: u64,
    },
    CellExecutionFailed {
        cell_id: CellId,
        execution_id: ExecutionId,
        error: ExecutionError,
        failed_at: i64,
    },
    
    // Collaboration events
    CursorPositionChanged {
        user_id: UserId,
        cell_id: Option<CellId>,
        position: CursorPosition,
        changed_at: i64,
    },
}
```

### 1.2 Implement Event Store

```rust
// core/src/event_store.rs
pub struct EventStore {
    db: SqliteConnection,
    subscribers: HashMap<QueryKey, Vec<Sender<QueryResult>>>,
}

impl EventStore {
    // Core event methods
    pub fn append_event(&mut self, event: NotebookEvent) -> Result<EventId>;
    pub fn get_events_since(&self, since: EventId) -> Result<Vec<NotebookEvent>>;
    pub fn replay_events(&mut self) -> Result<()>;
    
    // Materialization
    pub fn materialize_event(&mut self, event: &NotebookEvent) -> Result<()>;
    
    // Reactive queries
    pub fn subscribe_to_query<T>(&mut self, query: Query<T>) -> Receiver<T>;
    pub fn query_cell_source(&self, cell_id: &CellId) -> Result<Option<String>>;
    pub fn query_ordered_cells(&self) -> Result<Vec<Cell>>;
    pub fn query_cell_execution_state(&self, cell_id: &CellId) -> Result<ExecutionState>;
    pub fn query_notebook_metadata(&self) -> Result<NotebookMetadata>;
}
```

### 1.3 Build Materializers

```rust
// core/src/materializers.rs
impl EventStore {
    fn materialize_event(&mut self, event: &NotebookEvent) -> Result<()> {
        match event {
            NotebookEvent::CellCreated { cell_id, cell_type, fractional_index, created_at, .. } => {
                sqlx::query!(
                    "INSERT INTO cells (id, cell_type, fractional_index, created_at, source) 
                     VALUES (?, ?, ?, ?, '')",
                    cell_id, cell_type, fractional_index, created_at
                ).execute(&mut self.db)?;
            },
            
            NotebookEvent::CellSourceChanged { cell_id, source, changed_at, .. } => {
                sqlx::query!(
                    "UPDATE cells SET source = ?, updated_at = ? WHERE id = ?",
                    source, changed_at, cell_id
                ).execute(&mut self.db)?;
            },
            
            NotebookEvent::CellExecutionCompleted { cell_id, execution_id, outputs, completed_at, .. } => {
                // Update cell execution state
                sqlx::query!(
                    "UPDATE cells SET execution_state = 'completed', last_execution_at = ? WHERE id = ?",
                    completed_at, cell_id
                ).execute(&mut self.db)?;
                
                // Store execution record
                sqlx::query!(
                    "INSERT INTO executions (id, cell_id, status, completed_at, outputs_json) 
                     VALUES (?, ?, 'completed', ?, ?)",
                    execution_id, cell_id, completed_at, serde_json::to_string(outputs)?
                ).execute(&mut self.db)?;
                
                // Store individual outputs in artifacts table for large data
                for output in outputs {
                    if output.size_bytes() > 1024 * 100 { // 100KB threshold
                        self.store_output_artifact(execution_id, output)?;
                    }
                }
            },
            
            // ... other materializers
        }
        
        // Notify subscribers of query changes
        self.notify_query_subscribers(event)?;
        Ok(())
    }
}
```

## Phase 2: NAPI Integration Layer (Week 2-3)

### 2.1 NAPI Bindings

```rust
// src/lib.rs (NAPI)
use eventbook_core::*;

#[napi]
pub struct EventBookStore {
    inner: Arc<Mutex<EventStore>>,
    runtime: tokio::runtime::Runtime,
}

#[napi]
impl EventBookStore {
    #[napi(constructor)]
    pub fn new(notebook_id: String, db_path: Option<String>) -> Result<Self> {
        let store = EventStore::new(notebook_id, db_path)?;
        let runtime = tokio::runtime::Runtime::new()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(store)),
            runtime,
        })
    }
    
    #[napi]
    pub fn commit_event(&self, event_json: String) -> Result<()> {
        let event: NotebookEvent = serde_json::from_str(&event_json)?;
        let mut store = self.inner.lock().unwrap();
        store.append_event(event)?;
        Ok(())
    }
    
    #[napi]
    pub fn query_cell_source(&self, cell_id: String) -> Result<Option<String>> {
        let store = self.inner.lock().unwrap();
        store.query_cell_source(&CellId::new(cell_id))
    }
    
    #[napi]
    pub fn query_ordered_cells(&self) -> Result<String> {
        let store = self.inner.lock().unwrap();
        let cells = store.query_ordered_cells()?;
        Ok(serde_json::to_string(&cells)?)
    }
    
    #[napi]
    pub fn subscribe_to_cell_source(&self, cell_id: String, callback: JsFunction) -> Result<u32> {
        // Set up subscription that calls JavaScript callback when cell source changes
        let query = Query::CellSource(CellId::new(cell_id));
        let subscription_id = self.setup_subscription(query, callback)?;
        Ok(subscription_id)
    }
    
    #[napi]
    pub fn unsubscribe(&self, subscription_id: u32) -> Result<()> {
        let mut store = self.inner.lock().unwrap();
        store.remove_subscription(subscription_id)
    }
}
```

### 2.2 TypeScript Type Definitions

Generate TypeScript definitions from Rust:

```typescript
// Generated types
export interface NotebookEvent {
  type: 'NotebookInitialized' | 'CellCreated' | 'CellSourceChanged' | /* ... */;
  payload: NotebookEventPayload;
}

export interface Cell {
  id: string;
  cell_type: 'code' | 'markdown' | 'sql' | 'ai';
  source: string;
  fractional_index: string;
  execution_state: 'idle' | 'queued' | 'running' | 'completed' | 'error';
  created_at: number;
  updated_at?: number;
}

export class EventBookStore {
  constructor(notebookId: string, dbPath?: string);
  commitEvent(eventJson: string): void;
  queryCellSource(cellId: string): string | null;
  queryOrderedCells(): string; // JSON-serialized Cell[]
  subscribeToCellSource(cellId: string, callback: (source: string) => void): number;
  unsubscribe(subscriptionId: number): void;
}
```

## Phase 3: React Integration Layer (Week 3-4)

### 3.1 EventBook Provider

```typescript
// src/providers/EventBookProvider.tsx
import { EventBookStore } from '@eventbook/core';

interface EventBookContextType {
  store: EventBookStore;
  commitEvent: (event: NotebookEvent) => void;
}

const EventBookContext = createContext<EventBookContextType | null>(null);

export function EventBookProvider({ 
  notebookId, 
  children 
}: { 
  notebookId: string;
  children: ReactNode;
}) {
  const [store] = useState(() => new EventBookStore(notebookId));
  
  const commitEvent = useCallback((event: NotebookEvent) => {
    store.commitEvent(JSON.stringify(event));
  }, [store]);
  
  return (
    <EventBookContext.Provider value={{ store, commitEvent }}>
      {children}
    </EventBookContext.Provider>
  );
}
```

### 3.2 Fine-Grained React Hooks

```typescript
// src/hooks/useCell.ts
export function useCellSource(cellId: string): [string, (source: string) => void] {
  const { store, commitEvent } = useEventBook();
  const [source, setSource] = useState('');
  
  useEffect(() => {
    // Get initial value
    const initialSource = store.queryCellSource(cellId);
    if (initialSource) setSource(initialSource);
    
    // Subscribe to changes
    const subscriptionId = store.subscribeToCellSource(cellId, (newSource) => {
      setSource(newSource);
    });
    
    return () => store.unsubscribe(subscriptionId);
  }, [cellId, store]);
  
  const updateSource = useCallback((newSource: string) => {
    commitEvent({
      type: 'CellSourceChanged',
      payload: {
        cell_id: cellId,
        source: newSource,
        changed_at: Date.now(),
        change_id: generateChangeId(),
      }
    });
  }, [cellId, commitEvent]);
  
  return [source, updateSource];
}

export function useCellExecution(cellId: string) {
  const { store, commitEvent } = useEventBook();
  const [executionState, setExecutionState] = useState<ExecutionState>('idle');
  
  useEffect(() => {
    const subscriptionId = store.subscribeToCellExecutionState(cellId, setExecutionState);
    return () => store.unsubscribe(subscriptionId);
  }, [cellId, store]);
  
  const executeCell = useCallback(() => {
    const executionId = generateExecutionId();
    commitEvent({
      type: 'CellExecutionRequested',
      payload: {
        cell_id: cellId,
        execution_id: executionId,
        requested_at: Date.now(),
      }
    });
  }, [cellId, commitEvent]);
  
  return { executionState, executeCell };
}

export function useOrderedCells(): Cell[] {
  const { store } = useEventBook();
  const [cells, setCells] = useState<Cell[]>([]);
  
  useEffect(() => {
    const subscriptionId = store.subscribeToOrderedCells((newCells) => {
      setCells(JSON.parse(newCells));
    });
    
    return () => store.unsubscribe(subscriptionId);
  }, [store]);
  
  return cells;
}
```

### 3.3 Component Refactoring

```typescript
// src/components/Cell/CellEditor.tsx
export function CellEditor({ cellId }: { cellId: string }) {
  const [source, updateSource] = useCellSource(cellId);
  const { executionState, executeCell } = useCellExecution(cellId);
  
  return (
    <div className="cell-editor">
      <textarea
        value={source}
        onChange={(e) => updateSource(e.target.value)}
        disabled={executionState === 'running'}
      />
      <button 
        onClick={executeCell}
        disabled={executionState === 'running'}
      >
        {executionState === 'running' ? 'Running...' : 'Run'}
      </button>
    </div>
  );
}

// This component ONLY re-renders when this specific cell's source changes!
// No more parent state cascade issues.
```

## Phase 4: Server-Side Rust Integration (Week 4-5)

### 4.1 Server Event Store

```rust
// server/src/notebook_runtime.rs
pub struct NotebookRuntime {
    event_store: EventStore,
    executor: CellExecutor,
    websocket_broadcaster: WebSocketBroadcaster,
}

impl NotebookRuntime {
    pub async fn handle_event(&mut self, event: NotebookEvent) -> Result<()> {
        // Materialize event
        self.event_store.append_event(event.clone())?;
        
        // Handle execution requests
        match &event {
            NotebookEvent::CellExecutionRequested { cell_id, execution_id, .. } => {
                self.start_cell_execution(cell_id, execution_id).await?;
            },
            _ => {}
        }
        
        // Broadcast to connected clients
        self.websocket_broadcaster.broadcast_event(event).await?;
        
        Ok(())
    }
    
    async fn start_cell_execution(&mut self, cell_id: &CellId, execution_id: &ExecutionId) -> Result<()> {
        // Emit execution started event
        let started_event = NotebookEvent::CellExecutionStarted {
            cell_id: cell_id.clone(),
            execution_id: execution_id.clone(),
            started_at: chrono::Utc::now().timestamp_millis(),
        };
        self.event_store.append_event(started_event.clone())?;
        self.websocket_broadcaster.broadcast_event(started_event).await?;
        
        // Get cell source for execution
        let source = self.event_store.query_cell_source(cell_id)?
            .ok_or_else(|| anyhow!("Cell not found"))?;
        
        // Execute asynchronously
        let cell_id = cell_id.clone();
        let execution_id = execution_id.clone();
        let executor = self.executor.clone();
        let event_store = self.event_store.clone();
        let broadcaster = self.websocket_broadcaster.clone();
        
        tokio::spawn(async move {
            match executor.execute_cell(&source).await {
                Ok(outputs) => {
                    let completed_event = NotebookEvent::CellExecutionCompleted {
                        cell_id,
                        execution_id,
                        outputs,
                        completed_at: chrono::Utc::now().timestamp_millis(),
                        duration_ms: 0, // TODO: track duration
                    };
                    event_store.append_event(completed_event.clone())?;
                    broadcaster.broadcast_event(completed_event).await?;
                },
                Err(error) => {
                    let failed_event = NotebookEvent::CellExecutionFailed {
                        cell_id,
                        execution_id,
                        error: ExecutionError::from(error),
                        failed_at: chrono::Utc::now().timestamp_millis(),
                    };
                    event_store.append_event(failed_event.clone())?;
                    broadcaster.broadcast_event(failed_event).await?;
                }
            }
        });
        
        Ok(())
    }
}
```

### 4.2 WebSocket Event Broadcasting

```rust
// server/src/websocket.rs
impl WebSocketManager {
    pub async fn handle_client_event(&mut self, client_id: ClientId, event: NotebookEvent) -> Result<()> {
        // Validate and process event through runtime
        self.runtime.handle_event(event).await?;
        Ok(())
    }
    
    pub async fn broadcast_event(&self, event: NotebookEvent) -> Result<()> {
        let message = serde_json::to_string(&event)?;
        
        for connection in self.connections.values() {
            if let Err(e) = connection.send(Message::Text(message.clone())).await {
                warn!("Failed to send event to client: {}", e);
            }
        }
        
        Ok(())
    }
}
```

## Phase 5: Performance and Polish (Week 5-6)

### 5.1 Query Optimization

- Add SQLite indexes for common query patterns
- Implement query result caching in Rust
- Add query batching to reduce subscription overhead
- Profile and optimize hot paths

### 5.2 Operational Transform Integration

- Implement proper operational transform for concurrent text editing
- Handle conflict resolution for cell reordering
- Add undo/redo event generation

### 5.3 Artifact Storage

- Move large execution outputs to artifact storage
- Implement on-demand loading for large outputs
- Add cleanup for old artifacts

## Migration Strategy

### Step 1: Parallel Implementation
- Keep current system running
- Implement new Rust event store alongside
- Add feature flag to switch between systems

### Step 2: Component-by-Component Migration  
- Start with read-only components (output display)
- Move to simple editors (cell source)
- Finally migrate complex interactions (cell creation, execution)

### Step 3: Full Cutover
- Remove old hook-based state management  
- Remove EventBookProvider complexity
- Clean up unused TypeScript state code

## Success Metrics

1. **Performance**: Cell editor typing latency < 16ms
2. **Scalability**: 1000+ cell notebooks load in < 2s
3. **Reliability**: Zero state synchronization bugs
4. **Developer Experience**: Component queries are self-documenting
5. **Type Safety**: 100% compile-time guarantee from Rust to React

## Risk Mitigation

- **NAPI Learning Curve**: Start with simple queries, iterate
- **Performance Concerns**: Profile early and often, benchmark against current system  
- **Migration Complexity**: Use feature flags and gradual rollout
- **WebSocket Reliability**: Implement reconnection and event replay logic
- **State Debugging**: Build SQLite inspection tools early

This architecture provides the fine-grained reactivity you want while keeping all the complex event sourcing logic in Rust with full type safety.