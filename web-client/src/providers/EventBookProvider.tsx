import React, {
  createContext,
  useContext,
  useEffect,
  useReducer,
  useCallback,
  useMemo,
} from "react";
import { useWebSocket } from "@/hooks/useWebSocket";
import { api, eventOperations } from "@/lib/api";
import type { NotebookState, Event, Cell, CellType } from "@/types/eventbook";

// Action types for state management
type EventBookAction =
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "SET_ERROR"; payload: string | null }
  | {
      type: "LOAD_NOTEBOOK_SUCCESS";
      payload: { events: Event[]; state: NotebookState };
    }
  | { type: "APPLY_EVENT"; payload: Event }
  | { type: "RESET_STATE" };

// EventBook context interface
interface EventBookContextType {
  // State
  notebookId: string;
  notebookState: NotebookState;
  isLoading: boolean;
  error: string | null;

  // WebSocket connection state
  isConnected: boolean;
  connectionStatus: "connecting" | "connected" | "disconnected" | "error";
  connectionId: string | null;
  wsError: string | null;

  // Actions
  loadNotebook: () => Promise<void>;
  submitEvent: (eventRequest: {
    event_type: string;
    payload: any;
  }) => Promise<void>;

  // Notebook operations
  initializeNotebook: (title?: string, metadata?: any) => Promise<void>;
  updateNotebookTitle: (title: string) => Promise<void>;

  // Cell operations
  createCell: (
    cellType: CellType,
    source?: string,
    afterCellId?: string,
  ) => Promise<string>;
  updateCellSource: (cellId: string, source: string) => Promise<void>;
  executeCell: (cellId: string) => Promise<void>;
  moveCell: (cellId: string, afterCellId?: string) => Promise<void>;
  deleteCell: (cellId: string) => Promise<void>;

  // Utility
  getCellById: (cellId: string) => Cell | undefined;
  getOrderedCells: () => Cell[];
}

const EventBookContext = createContext<EventBookContextType | null>(null);

// State reducer for notebook management
function notebookReducer(
  state: NotebookState & { isLoading: boolean; error: string | null },
  action: EventBookAction,
) {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, isLoading: action.payload };

    case "SET_ERROR":
      return { ...state, error: action.payload, isLoading: false };

    case "LOAD_NOTEBOOK_SUCCESS":
      return {
        ...action.payload.state,
        isLoading: false,
        error: null,
      };

    case "APPLY_EVENT":
      return applyEventToState(state, action.payload);

    case "RESET_STATE":
      return {
        cells: {},
        outputs: {},
        orderedCells: [],
        lastProcessedTimestamp: 0,
        events: [],
        document: undefined,
        isLoading: false,
        error: null,
      };

    default:
      return state;
  }
}

// Apply an event to the current state
function applyEventToState(
  state: NotebookState & { isLoading: boolean; error: string | null },
  event: Event,
): NotebookState & { isLoading: boolean; error: string | null } {
  const newState = { ...state };

  switch (event.event_type) {
    case "NotebookMetadataSet":
      newState.document = {
        id: event.aggregate_id,
        title: event.payload.title || "Untitled",
        metadata: {
          authors: event.payload.authors || [],
          tags: event.payload.tags || [],
          custom: event.payload.custom || {},
        },
        created_at: event.timestamp,
        updated_at: event.timestamp,
      };
      break;

    case "CellCreated":
      const cell: Cell = {
        id: event.payload.cell_id,
        cell_type: event.payload.cell_type,
        source: event.payload.source || "",
        fractional_index: event.payload.fractional_index,
        execution_state: "idle",
        source_visible: true,
        output_visible: true,
        ai_context_visible: true,
        created_by: event.payload.created_by || "user",
        document_id: event.aggregate_id,
        created_at: event.timestamp,
        updated_at: event.timestamp,
      };
      newState.cells = { ...newState.cells, [cell.id]: cell };
      break;

    case "CellSourceUpdated":
      if (newState.cells[event.payload.cell_id]) {
        newState.cells = {
          ...newState.cells,
          [event.payload.cell_id]: {
            ...newState.cells[event.payload.cell_id],
            source: event.payload.source,
            updated_at: event.timestamp,
          },
        };
      }
      break;

    case "CellExecutionStateChanged":
      if (newState.cells[event.payload.cell_id]) {
        newState.cells = {
          ...newState.cells,
          [event.payload.cell_id]: {
            ...newState.cells[event.payload.cell_id],
            execution_state: event.payload.execution_state,
            assigned_runtime_session: event.payload.assigned_runtime_session,
            last_execution_duration_ms: event.payload.execution_duration_ms,
            updated_at: event.timestamp,
          },
        };
      }
      break;

    case "CellDeleted":
      const { [event.payload.cell_id]: deletedCell, ...remainingCells } =
        newState.cells;
      newState.cells = remainingCells;
      // Also remove associated outputs
      newState.outputs = Object.fromEntries(
        Object.entries(newState.outputs).filter(
          ([_, output]) => output.cell_id !== event.payload.cell_id,
        ),
      );
      break;

    case "CellMoved":
      if (newState.cells[event.payload.cell_id]) {
        newState.cells = {
          ...newState.cells,
          [event.payload.cell_id]: {
            ...newState.cells[event.payload.cell_id],
            fractional_index: event.payload.fractional_index,
            updated_at: event.timestamp,
          },
        };
      }
      break;
  }

  // Update timestamp and add event to history
  newState.lastProcessedTimestamp = Math.max(
    newState.lastProcessedTimestamp,
    event.timestamp,
  );
  newState.events = [...newState.events, event];

  // Recompute ordered cells
  newState.orderedCells = Object.values(newState.cells).sort((a, b) => {
    const aIndex = a.fractional_index || "z";
    const bIndex = b.fractional_index || "z";
    return aIndex.localeCompare(bIndex);
  });

  return newState;
}

// Generate a simple fractional index (temporary implementation)
function generateFractionalIndex(
  afterCellId?: string,
  _beforeCellId?: string,
  cells?: Record<string, Cell>,
): string {
  if (!cells || Object.keys(cells).length === 0) {
    return "a0";
  }

  const orderedCells = Object.values(cells).sort((a, b) => {
    const aIndex = a.fractional_index || "z";
    const bIndex = b.fractional_index || "z";
    return aIndex.localeCompare(bIndex);
  });

  if (afterCellId) {
    const afterCell = cells[afterCellId];
    const afterIndex = afterCell?.fractional_index || "a0";
    const afterPos = orderedCells.findIndex((c) => c.id === afterCellId);

    if (afterPos >= 0 && afterPos < orderedCells.length - 1) {
      // Simple midpoint - in production, use proper fractional indexing
      return afterIndex + "m";
    } else {
      return afterIndex + "z";
    }
  }

  // Default: add at end
  const lastCell = orderedCells[orderedCells.length - 1];
  const lastIndex = lastCell?.fractional_index || "a";
  return lastIndex + "z";
}

// Provider component
interface EventBookProviderProps {
  children: React.ReactNode;
  notebookId: string;
}

export function EventBookProvider({
  children,
  notebookId,
}: EventBookProviderProps) {
  const [state, dispatch] = useReducer(notebookReducer, {
    cells: {},
    outputs: {},
    orderedCells: [],
    lastProcessedTimestamp: 0,
    events: [],
    document: undefined,
    isLoading: false,
    error: null,
  });

  // WebSocket connection for real-time collaboration
  const {
    connectionStatus,
    connectionId,
    lastEvent,
    isConnected,
    error: wsError,
  } = useWebSocket(notebookId, { enabled: true });

  // Handle real-time events from WebSocket
  useEffect(() => {
    if (lastEvent) {
      console.log("Received real-time event:", lastEvent.event_type);
      dispatch({ type: "APPLY_EVENT", payload: lastEvent });
    }
  }, [lastEvent]);

  // Load notebook on mount or when notebookId changes
  const loadNotebook = useCallback(async () => {
    if (!notebookId?.trim()) return;

    dispatch({ type: "SET_LOADING", payload: true });
    dispatch({ type: "SET_ERROR", payload: null });

    try {
      const response = await api.getEvents(notebookId);

      // Reconstruct state from events
      let reconstructedState: NotebookState = {
        cells: {},
        outputs: {},
        orderedCells: [],
        lastProcessedTimestamp: 0,
        events: response.events,
      };

      // Apply all events to reconstruct state
      for (const event of response.events) {
        reconstructedState = applyEventToState(
          { ...reconstructedState, isLoading: false, error: null },
          event,
        );
      }

      dispatch({
        type: "LOAD_NOTEBOOK_SUCCESS",
        payload: { events: response.events, state: reconstructedState },
      });
    } catch (err) {
      dispatch({
        type: "SET_ERROR",
        payload: err instanceof Error ? err.message : "Failed to load notebook",
      });
    }
  }, [notebookId]);

  // Submit event to server
  const submitEvent = useCallback(
    async (eventRequest: { event_type: string; payload: any }) => {
      try {
        dispatch({ type: "SET_ERROR", payload: null });
        await api.submitEvent(notebookId, eventRequest);
        // State will be updated via WebSocket or we can optimistically apply
      } catch (err) {
        dispatch({
          type: "SET_ERROR",
          payload:
            err instanceof Error ? err.message : "Failed to submit event",
        });
        throw err;
      }
    },
    [notebookId],
  );

  // Notebook operations
  const initializeNotebook = useCallback(
    async (title = "Untitled Notebook", metadata = {}) => {
      const event = eventOperations.createDocument(`${title}`, {
        authors: ["user"],
        tags: [],
        ...metadata,
      });
      await submitEvent(event);
    },
    [submitEvent],
  );

  const updateNotebookTitle = useCallback(
    async (title: string) => {
      const event = {
        event_type: "NotebookMetadataSet",
        payload: {
          title,
          ...(state.document?.metadata || {}),
        },
      };
      await submitEvent(event);
    },
    [submitEvent, state.document],
  );

  // Cell operations
  const createCell = useCallback(
    async (
      cellType: CellType,
      source = "",
      afterCellId?: string,
    ): Promise<string> => {
      const cellId = `cell-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      const fractionalIndex = generateFractionalIndex(
        afterCellId,
        undefined,
        state.cells,
      );

      const event = eventOperations.createCell(
        cellId,
        cellType,
        source,
        fractionalIndex,
        "user",
      );

      await submitEvent(event);
      return cellId;
    },
    [submitEvent, state.cells],
  );

  const updateCellSource = useCallback(
    async (cellId: string, source: string) => {
      const event = eventOperations.updateCellSource(cellId, source);
      await submitEvent(event);
    },
    [submitEvent],
  );

  const executeCell = useCallback(
    async (cellId: string) => {
      // Start execution
      await submitEvent(
        eventOperations.updateExecutionState(cellId, "running"),
      );

      // Simulate execution completion (in real app, this would be handled by runtime)
      setTimeout(async () => {
        await submitEvent(
          eventOperations.updateExecutionState(
            cellId,
            "completed",
            "runtime-1",
            42,
          ),
        );
      }, 1000);
    },
    [submitEvent],
  );

  const moveCell = useCallback(
    async (cellId: string, afterCellId?: string) => {
      const fractionalIndex = generateFractionalIndex(
        afterCellId,
        undefined,
        state.cells,
      );
      const event = eventOperations.moveCell(cellId, fractionalIndex);
      await submitEvent(event);
    },
    [submitEvent, state.cells],
  );

  const deleteCell = useCallback(
    async (cellId: string) => {
      const event = eventOperations.deleteCell(cellId);
      await submitEvent(event);
    },
    [submitEvent],
  );

  // Utility functions
  const getCellById = useCallback(
    (cellId: string) => {
      return state.cells[cellId];
    },
    [state.cells],
  );

  const getOrderedCells = useCallback(() => {
    return state.orderedCells;
  }, [state.orderedCells]);

  // Load notebook on mount or notebookId change
  useEffect(() => {
    loadNotebook();
  }, [loadNotebook]);

  // Memoized context value
  const contextValue = useMemo(
    (): EventBookContextType => ({
      // State
      notebookId,
      notebookState: state,
      isLoading: state.isLoading,
      error: state.error,

      // WebSocket state
      isConnected,
      connectionStatus,
      connectionId,
      wsError,

      // Actions
      loadNotebook,
      submitEvent,

      // Notebook operations
      initializeNotebook,
      updateNotebookTitle,

      // Cell operations
      createCell,
      updateCellSource,
      executeCell,
      moveCell,
      deleteCell,

      // Utility
      getCellById,
      getOrderedCells,
    }),
    [
      notebookId,
      state,
      isConnected,
      connectionStatus,
      connectionId,
      wsError,
      loadNotebook,
      submitEvent,
      initializeNotebook,
      updateNotebookTitle,
      createCell,
      updateCellSource,
      executeCell,
      moveCell,
      deleteCell,
      getCellById,
      getOrderedCells,
    ],
  );

  return (
    <EventBookContext.Provider value={contextValue}>
      {children}
    </EventBookContext.Provider>
  );
}

// Hook to use EventBook context
export function useEventBook(): EventBookContextType {
  const context = useContext(EventBookContext);
  if (!context) {
    throw new Error("useEventBook must be used within an EventBookProvider");
  }
  return context;
}
