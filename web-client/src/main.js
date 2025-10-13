// EventBook WASM Event Sourcing Demo
import init, {
  EventBookClient,
  create_sample_user_payload,
  current_timestamp,
  generate_event_id,
  validate_json_payload,
} from "./wasm/eventbook_wasm.js";

class EventBookApp {
  constructor() {
    this.client = null;
    this.wasmInitialized = false;

    // Environment-aware server URL detection
    if (import.meta.env.DEV) {
      // Development: use Vite proxy
      this.serverUrl = "/api";
    } else {
      // Production: use environment variable or default
      this.serverUrl =
        import.meta.env.VITE_SERVER_URL || "http://localhost:3000";
    }

    console.log("EventBook client connecting to:", this.serverUrl);
  }

  async init() {
    try {
      // Initialize WASM module
      await init();

      // Create EventBook client with server config
      this.client = new EventBookClient(this.serverUrl);

      this.wasmInitialized = true;
      this.updateWasmStatus("success", "✅ WASM module loaded successfully");

      // Setup event handlers
      this.setupEventHandlers();

      // Initial sync
      await this.syncEventLog();
    } catch (error) {
      console.error("Failed to initialize WASM:", error);
      this.updateWasmStatus(
        "error",
        `❌ WASM initialization failed: ${error.message}`,
      );
    }
  }

  setupEventHandlers() {
    // Form submission
    document
      .getElementById("eventForm")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        await this.submitEvent();
      });

    // Global functions for buttons
    window.syncEventLog = () => this.syncEventLog();
    window.rebuildProjections = () => this.rebuildProjections();
    window.fetchEvents = () => this.fetchEvents();
    window.showMaterializedUsers = () => this.showMaterializedUsers();
    window.runDemo = () => this.runDemo();
    window.fillSampleData = () => this.fillSampleData();
  }

  updateWasmStatus(type, message) {
    const statusEl = document.getElementById("wasmStatus");
    statusEl.innerHTML = `<div class="status ${type}">${message}</div>`;
  }

  updateSyncStatus(type, message) {
    const statusEl = document.getElementById("syncStatus");
    statusEl.innerHTML = `<div class="status ${type}">${message}</div>`;
    if (type === "success") {
      setTimeout(() => (statusEl.innerHTML = ""), 3000);
    }
  }

  updateSubmitStatus(type, message) {
    const statusEl = document.getElementById("submitStatus");
    statusEl.innerHTML = `<div class="status ${type}">${message}</div>`;
    if (type === "success") {
      setTimeout(() => (statusEl.innerHTML = ""), 3000);
    }
  }

  async submitEvent() {
    if (!this.wasmInitialized) {
      this.updateSubmitStatus("error", "WASM not initialized");
      return;
    }

    try {
      const eventType = document.getElementById("eventType").value;
      const aggregateId = document.getElementById("aggregateId").value;
      const payloadText = document.getElementById("payload").value;

      // Validate payload
      try {
        validate_json_payload(payloadText);
      } catch (error) {
        this.updateSubmitStatus("error", `Invalid JSON: ${error.message}`);
        return;
      }

      // Submit event locally to WASM client
      const event = this.client.submit_event(
        eventType,
        aggregateId,
        payloadText,
      );

      this.updateSubmitStatus(
        "success",
        `✅ Event ${event.id} added locally (v${event.version})`,
      );

      // Also send to server for persistence
      await this.sendEventToServer(
        eventType,
        aggregateId,
        JSON.parse(payloadText),
      );

      // Refresh displays
      this.showLocalEvents();
      this.showMaterializedUsers();
    } catch (error) {
      console.error("Submit error:", error);
      this.updateSubmitStatus("error", `❌ ${error.message}`);
    }
  }

  async sendEventToServer(eventType, aggregateId, payload) {
    try {
      const response = await fetch(`${this.serverUrl}/events`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          event_type: eventType,
          aggregate_id: aggregateId,
          payload: payload,
        }),
      });

      if (!response.ok) {
        throw new Error(`Server error: ${response.status}`);
      }
    } catch (error) {
      console.warn("Failed to sync to server:", error.message);
    }
  }

  async syncEventLog() {
    if (!this.wasmInitialized) return;

    try {
      this.updateSyncStatus("info", "🔄 Syncing event log from server...");

      // Fetch events from server
      const response = await fetch(`${this.serverUrl}/events`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      const data = await response.json();
      const serverEvents = data.events || [];

      // Clear local store and rebuild from server events
      this.client.clear_local_store();

      // Add each server event to local WASM store
      let syncedCount = 0;
      for (const serverEvent of serverEvents) {
        try {
          this.client.submit_event(
            serverEvent.event_type,
            serverEvent.aggregate_id,
            JSON.stringify(serverEvent.payload),
          );
          syncedCount++;
        } catch (error) {
          console.warn("Failed to sync event:", serverEvent.id, error);
        }
      }

      this.updateSyncStatus(
        "success",
        `✅ Synced ${syncedCount} events from server`,
      );

      // Refresh displays
      this.showLocalEvents();
      this.showMaterializedUsers();
    } catch (error) {
      console.error("Sync failed:", error);
      this.updateSyncStatus("error", `❌ Sync failed: ${error.message}`);
    }
  }

  async rebuildProjections() {
    if (!this.wasmInitialized) return;

    try {
      const eventCount = this.client.rebuild_projections();
      this.updateSyncStatus(
        "success",
        `🔨 Rebuilt projections from ${eventCount} events`,
      );
      this.showMaterializedUsers();
    } catch (error) {
      this.updateSyncStatus("error", `❌ Rebuild failed: ${error.message}`);
    }
  }

  async fetchEvents() {
    this.showLocalEvents();
  }

  showLocalEvents() {
    if (!this.wasmInitialized) return;

    try {
      const events = this.client.get_events();
      const eventLogEl = document.getElementById("eventLog");

      if (events.length === 0) {
        eventLogEl.innerHTML =
          '<div class="loading">No events in local store</div>';
        return;
      }

      const eventsHtml = events
        .map(
          (event) => `
                <div class="event-item">
                    <div class="event-header">
                        <span class="event-type-badge">${event.event_type}</span>
                        <span class="event-meta">v${event.version} • ${event.aggregate_id}</span>
                    </div>
                    <div class="event-meta">
                        ${new Date(event.timestamp * 1000).toLocaleString()} • ${event.id}
                    </div>
                    <div class="event-payload">${event.payload}</div>
                </div>
            `,
        )
        .join("");

      eventLogEl.innerHTML = eventsHtml;
    } catch (error) {
      console.error("Failed to show events:", error);
    }
  }

  showMaterializedUsers() {
    if (!this.wasmInitialized) return;

    try {
      const users = this.client.get_materialized_users();
      const usersEl = document.getElementById("materializedUsers");
      const statsEl = document.getElementById("userStats");
      const statsTextEl = document.getElementById("statsText");

      if (users.length === 0) {
        usersEl.innerHTML = '<div class="loading">No users materialized</div>';
        statsEl.style.display = "none";
        return;
      }

      const usersHtml = users
        .map(
          (user) => `
                <div class="user-item">
                    <div class="user-info">
                        <h4>${user.name}</h4>
                        <div class="email">${user.email}</div>
                    </div>
                    <div class="user-status">
                        <span class="${user.active ? "active-badge" : "inactive-badge"}">
                            ${user.active ? "Active" : "Inactive"}
                        </span>
                        <div>Created: ${new Date(user.created_at * 1000).toLocaleDateString()}</div>
                        ${
                          user.created_at !== user.updated_at
                            ? `<div>Updated: ${new Date(user.updated_at * 1000).toLocaleDateString()}</div>`
                            : ""
                        }
                    </div>
                </div>
            `,
        )
        .join("");

      usersEl.innerHTML = usersHtml;

      // Show stats
      const totalEvents = this.client.get_event_count();
      const userCount = this.client.get_user_count();
      statsTextEl.textContent = `${userCount} users materialized from ${totalEvents} events`;
      statsEl.style.display = "block";
    } catch (error) {
      console.error("Failed to show materialized users:", error);
    }
  }

  fillSampleData() {
    const timestamp = Date.now();
    document.getElementById("eventType").value = "UserCreated";
    document.getElementById("aggregateId").value = `user-${timestamp}`;
    document.getElementById("payload").value = JSON.stringify(
      {
        name: "Alice Johnson",
        email: `alice.${timestamp}@example.com`,
      },
      null,
      2,
    );
  }

  async runDemo() {
    const demoStatusEl = document.getElementById("demoStatus");

    try {
      demoStatusEl.innerHTML =
        '<div class="status info">🎬 Running event sourcing demo...</div>';

      // Clear existing data
      this.client.clear_local_store();

      // Create a sequence of events
      const userId = `demo-user-${Date.now()}`;
      const events = [
        {
          type: "UserCreated",
          payload: { name: "Demo User", email: "demo@example.com" },
        },
        {
          type: "UserUpdated",
          payload: {
            name: "Updated Demo User",
            email: "updated.demo@example.com",
          },
        },
        {
          type: "UserUpdated",
          payload: { name: "Final Demo User" },
        },
      ];

      // Submit events with delays to show progression
      for (let i = 0; i < events.length; i++) {
        const event = events[i];
        this.client.submit_event(
          event.type,
          userId,
          JSON.stringify(event.payload),
        );

        demoStatusEl.innerHTML = `<div class="status info">📝 Added event ${i + 1}/${events.length}: ${event.type}</div>`;

        // Refresh displays
        this.showLocalEvents();
        this.showMaterializedUsers();

        // Pause between events
        if (i < events.length - 1) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
        }
      }

      demoStatusEl.innerHTML =
        '<div class="status success">🎉 Demo complete! Notice how the user state evolved from the event sequence.</div>';
      setTimeout(() => (demoStatusEl.innerHTML = ""), 5000);
    } catch (error) {
      console.error("Demo failed:", error);
      demoStatusEl.innerHTML = `<div class="status error">❌ Demo failed: ${error.message}</div>`;
    }
  }
}

// Initialize app when DOM is loaded
document.addEventListener("DOMContentLoaded", async () => {
  const app = new EventBookApp();
  await app.init();
});
