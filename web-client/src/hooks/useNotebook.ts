import { useEventBook } from "@/providers/EventBookProvider";

// Focused hook for notebook-level operations
export function useNotebook() {
  const {
    notebookId,
    notebookState,
    isLoading,
    error,
    initializeNotebook,
    updateNotebookTitle,
    loadNotebook,
  } = useEventBook();

  const document = notebookState.document;
  const isInitialized = !!document;

  // Notebook statistics
  const stats = {
    cellCount: Object.keys(notebookState.cells).length,
    eventCount: notebookState.events.length,
    lastUpdated: notebookState.lastProcessedTimestamp,
    codeCells: Object.values(notebookState.cells).filter(
      (c) => c.cell_type === "code",
    ).length,
    markdownCells: Object.values(notebookState.cells).filter(
      (c) => c.cell_type === "markdown",
    ).length,
    sqlCells: Object.values(notebookState.cells).filter(
      (c) => c.cell_type === "sql",
    ).length,
    aiCells: Object.values(notebookState.cells).filter(
      (c) => c.cell_type === "ai",
    ).length,
  };

  // Execution statistics
  const executionStats = {
    idle: Object.values(notebookState.cells).filter(
      (c) => c.execution_state === "idle",
    ).length,
    queued: Object.values(notebookState.cells).filter(
      (c) => c.execution_state === "queued",
    ).length,
    running: Object.values(notebookState.cells).filter(
      (c) => c.execution_state === "running",
    ).length,
    completed: Object.values(notebookState.cells).filter(
      (c) => c.execution_state === "completed",
    ).length,
    error: Object.values(notebookState.cells).filter(
      (c) => c.execution_state === "error",
    ).length,
  };

  return {
    // Basic notebook info
    notebookId,
    document,
    isInitialized,
    isLoading,
    error,

    // Statistics
    stats,
    executionStats,

    // Actions
    initialize: initializeNotebook,
    updateTitle: updateNotebookTitle,
    reload: loadNotebook,

    // Computed properties
    title: document?.title || "Untitled",
    authors: document?.metadata.authors || [],
    tags: document?.metadata.tags || [],
    createdAt: document?.created_at,
    updatedAt: document?.updated_at,
  };
}

// Hook for collaboration/connection status
export function useCollaboration() {
  const { isConnected, connectionStatus, connectionId, wsError } =
    useEventBook();

  return {
    isConnected,
    connectionStatus,
    connectionId: connectionId?.slice(-8), // Show only last 8 chars
    error: wsError,

    // Computed status for UI
    statusText: isConnected
      ? "Connected"
      : connectionStatus === "connecting"
        ? "Connecting..."
        : "Offline",
    statusColor: isConnected
      ? "green"
      : connectionStatus === "connecting"
        ? "yellow"
        : "red",
  };
}

// Hook for cell operations
export function useCells() {
  const {
    notebookState,
    createCell,
    updateCellSource,
    executeCell,
    moveCell,
    deleteCell,
    getCellById,
    getOrderedCells,
  } = useEventBook();

  return {
    // Cell data
    cells: notebookState.cells,
    orderedCells: getOrderedCells(),

    // Cell operations
    create: createCell,
    updateSource: updateCellSource,
    execute: executeCell,
    move: moveCell,
    delete: deleteCell,
    getById: getCellById,

    // Utilities
    isEmpty: getOrderedCells().length === 0,
    count: Object.keys(notebookState.cells).length,
  };
}

// Hook for events/audit log
export function useEventLog() {
  const { notebookState } = useEventBook();

  const events = notebookState.events;

  // Group events by type for analysis
  const eventsByType = events.reduce(
    (acc, event) => {
      acc[event.event_type] = (acc[event.event_type] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>,
  );

  // Recent events (last 10)
  const recentEvents = events.slice(-10).reverse();

  return {
    events,
    recentEvents,
    eventsByType,
    totalEvents: events.length,
    lastEventTimestamp:
      events.length > 0 ? events[events.length - 1].timestamp : null,

    // Event filtering utilities
    getEventsByType: (eventType: string) =>
      events.filter((e) => e.event_type === eventType),
    getEventsByCell: (cellId: string) =>
      events.filter((e) => e.payload.cell_id === cellId),
    getEventsInRange: (startTime: number, endTime: number) =>
      events.filter((e) => e.timestamp >= startTime && e.timestamp <= endTime),
  };
}
