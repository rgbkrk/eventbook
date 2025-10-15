import type {
  EventBookAPI,
  SubmitEventRequest,
  SubmitEventResponse,
  GetEventsQuery,
  GetEventsResponse,
  StoreInfoResponse,
} from "@/types/eventbook";

class EventBookAPIClient implements EventBookAPI {
  private baseUrl: string;

  constructor(baseUrl = "") {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;

    const response = await fetch(url, {
      headers: {
        "Content-Type": "application/json",
        ...options.headers,
      },
      ...options,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(
        `API request failed: ${response.status} ${response.statusText}. ${errorText}`,
      );
    }

    return response.json();
  }

  async submitEvent(
    storeId: string,
    request: SubmitEventRequest,
  ): Promise<SubmitEventResponse> {
    return this.request<SubmitEventResponse>(
      `/stores/${encodeURIComponent(storeId)}/events`,
      {
        method: "POST",
        body: JSON.stringify(request),
      },
    );
  }

  async getEvents(
    storeId: string,
    query: GetEventsQuery = {},
  ): Promise<GetEventsResponse> {
    const params = new URLSearchParams();

    if (query.limit !== undefined) {
      params.append("limit", query.limit.toString());
    }
    if (query.offset !== undefined) {
      params.append("offset", query.offset.toString());
    }
    if (query.since_timestamp !== undefined) {
      params.append("since_timestamp", query.since_timestamp.toString());
    }

    const queryString = params.toString();
    const endpoint = `/stores/${encodeURIComponent(storeId)}/events${
      queryString ? `?${queryString}` : ""
    }`;

    return this.request<GetEventsResponse>(endpoint);
  }

  async getStoreInfo(storeId: string): Promise<StoreInfoResponse> {
    return this.request<StoreInfoResponse>(
      `/stores/${encodeURIComponent(storeId)}`,
    );
  }

  async listStores(): Promise<string[]> {
    return this.request<string[]>("/stores");
  }

  async healthCheck(): Promise<{ status: string; timestamp: number }> {
    return this.request<{ status: string; timestamp: number }>("/health");
  }
}

// Create a singleton instance
export const api = new EventBookAPIClient();

// Export the class for testing or custom instances
export { EventBookAPIClient };

// Utility functions for common operations
export const eventOperations = {
  createDocument: (title: string, metadata = {}) => ({
    event_type: "NotebookMetadataSet",
    payload: {
      title,
      authors: [],
      tags: [],
      ...metadata,
    },
  }),

  createCell: (
    cellId: string,
    cellType: "code" | "markdown" | "sql" | "ai" | "raw",
    source: string = "",
    fractionalIndex: string = "a0",
    createdBy: string,
  ) => ({
    event_type: "CellCreated",
    payload: {
      cell_id: cellId,
      cell_type: cellType,
      source,
      fractional_index: fractionalIndex,
      created_by: createdBy,
    },
  }),

  updateCellSource: (cellId: string, source: string) => ({
    event_type: "CellSourceUpdated",
    payload: {
      cell_id: cellId,
      source,
    },
  }),

  moveCell: (cellId: string, fractionalIndex: string) => ({
    event_type: "CellMoved",
    payload: {
      cell_id: cellId,
      fractional_index: fractionalIndex,
    },
  }),

  deleteCell: (cellId: string) => ({
    event_type: "CellDeleted",
    payload: {
      cell_id: cellId,
    },
  }),

  updateExecutionState: (
    cellId: string,
    executionState: "idle" | "queued" | "running" | "completed" | "error",
    runtimeSession?: string,
    durationMs?: number,
  ) => ({
    event_type: "CellExecutionStateChanged",
    payload: {
      cell_id: cellId,
      execution_state: executionState,
      ...(runtimeSession && { assigned_runtime_session: runtimeSession }),
      ...(durationMs && { execution_duration_ms: durationMs }),
    },
  }),

  addOutput: (
    outputId: string,
    cellId: string,
    outputType:
      | "multimedia_display"
      | "multimedia_result"
      | "terminal"
      | "markdown"
      | "error",
    data: string,
    position: number = 0,
  ) => ({
    event_type: "CellOutputCreated",
    payload: {
      output_id: outputId,
      cell_id: cellId,
      output_type: outputType,
      position,
      data,
    },
  }),
};

// Helper to generate unique IDs
export const generateId = (prefix: string = "id") => {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
};
