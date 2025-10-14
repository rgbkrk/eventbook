use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use eventbook_core::Event;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Message types sent over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// New event was added to a store
    #[serde(rename = "event")]
    Event { store_id: String, event: Event },
    /// Store information update
    #[serde(rename = "store_info")]
    StoreInfo {
        store_id: String,
        event_count: usize,
        latest_version: i64,
    },
    /// Client successfully subscribed to a store
    #[serde(rename = "subscribed")]
    Subscribed {
        store_id: String,
        connection_id: String,
    },
    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
    /// Heartbeat/ping message
    #[serde(rename = "ping")]
    Ping,
    /// Heartbeat/pong response
    #[serde(rename = "pong")]
    Pong,
}

/// Client messages received over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to events for a specific store
    #[serde(rename = "subscribe")]
    Subscribe { store_id: String },
    /// Unsubscribe from a store
    #[serde(rename = "unsubscribe")]
    Unsubscribe { store_id: String },
    /// Heartbeat ping
    #[serde(rename = "ping")]
    Ping,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub sender: broadcast::Sender<WsMessage>,
}

/// WebSocket connection manager
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    /// Map of store_id -> list of connections subscribed to that store
    connections: Arc<RwLock<HashMap<String, Vec<Connection>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a connection to a store
    pub async fn subscribe(&self, store_id: String, connection: Connection) {
        let mut connections = self.connections.write().await;
        connections
            .entry(store_id.clone())
            .or_insert_with(Vec::new)
            .push(connection.clone());

        info!(
            "Connection {} subscribed to store {}",
            connection.id, store_id
        );
    }

    /// Remove a connection from a store
    pub async fn unsubscribe(&self, store_id: &str, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(store_connections) = connections.get_mut(store_id) {
            store_connections.retain(|conn| conn.id != connection_id);
            if store_connections.is_empty() {
                connections.remove(store_id);
            }
        }

        info!(
            "Connection {} unsubscribed from store {}",
            connection_id, store_id
        );
    }

    /// Remove a connection from all stores
    pub async fn disconnect(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        let mut stores_to_remove = Vec::new();

        for (store_id, store_connections) in connections.iter_mut() {
            store_connections.retain(|conn| conn.id != connection_id);
            if store_connections.is_empty() {
                stores_to_remove.push(store_id.clone());
            }
        }

        for store_id in stores_to_remove {
            connections.remove(&store_id);
        }

        info!("Connection {} disconnected from all stores", connection_id);
    }

    /// Broadcast an event to all connections subscribed to a store
    pub async fn broadcast_event(&self, store_id: String, event: Event) {
        let message = WsMessage::Event {
            store_id: store_id.clone(),
            event,
        };

        let mut disconnected = Vec::new();
        let mut connection_count = 0;

        // Limit scope of read lock
        {
            let connections = self.connections.read().await;
            if let Some(store_connections) = connections.get(&store_id) {
                connection_count = store_connections.len();
                for connection in store_connections {
                    if let Err(_) = connection.sender.send(message.clone()) {
                        // Connection is closed, mark for removal
                        disconnected.push(connection.id.clone());
                    }
                }
            }
        }

        // Clean up disconnected connections (lock is dropped here)
        for connection_id in disconnected {
            self.unsubscribe(&store_id, &connection_id).await;
        }

        info!(
            "Broadcasted event to {} connections for store {}",
            connection_count, store_id
        );
    }

    /// Get connection count for a store
    pub async fn get_connection_count(&self, store_id: &str) -> usize {
        let connections = self.connections.read().await;
        connections
            .get(store_id)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    /// Get total connection count across all stores
    pub async fn get_total_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections.values().map(|conns| conns.len()).sum()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle WebSocket upgrade request
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(store_id): Path<String>,
    State(app_state): State<crate::AppState>,
) -> Response {
    let manager = app_state.connection_manager.clone();
    ws.on_upgrade(move |socket| handle_socket(socket, store_id, manager))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, store_id: String, manager: Arc<ConnectionManager>) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();

    // Create broadcast channel for this connection
    let (tx, mut rx) = broadcast::channel::<WsMessage>(100);

    // Create connection object
    let connection = Connection {
        id: connection_id.clone(),
        sender: tx,
    };

    // Subscribe to the store
    manager.subscribe(store_id.clone(), connection).await;

    // Send subscription confirmation
    let confirm_msg = WsMessage::Subscribed {
        store_id: store_id.clone(),
        connection_id: connection_id.clone(),
    };

    if let Ok(msg_json) = serde_json::to_string(&confirm_msg) {
        if sender.send(Message::Text(msg_json.into())).await.is_err() {
            error!("Failed to send subscription confirmation");
            return;
        }
    }

    info!(
        "WebSocket connection {} established for store {}",
        connection_id, store_id
    );

    // Spawn task to handle outgoing messages
    let mut send_task = {
        let connection_id = connection_id.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if let Ok(msg_json) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(msg_json.into())).await.is_err() {
                        error!("Failed to send message to connection {}", connection_id);
                        break;
                    }
                } else {
                    error!(
                        "Failed to serialize message for connection {}",
                        connection_id
                    );
                }
            }
        })
    };

    // Spawn task to handle incoming messages
    let mut recv_task = {
        let manager = Arc::clone(&manager);
        let store_id = store_id.clone();
        let connection_id = connection_id.clone();

        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) =
                            handle_client_message(&text, &manager, &store_id, &connection_id).await
                        {
                            warn!("Error handling client message: {}", e);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection {} closed", connection_id);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error for connection {}: {}", connection_id, e);
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }
        })
    };

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        },
    }

    // Clean up connection
    manager.disconnect(&connection_id).await;
    info!("WebSocket connection {} cleaned up", connection_id);
}

/// Handle client messages
async fn handle_client_message(
    text: &str,
    manager: &ConnectionManager,
    current_store_id: &str,
    connection_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_msg: ClientMessage = serde_json::from_str(text)?;

    match client_msg {
        ClientMessage::Subscribe { store_id } => {
            // For now, we only support subscribing to the store specified in the URL
            if store_id != current_store_id {
                warn!(
                    "Connection {} tried to subscribe to {} but is connected to {}",
                    connection_id, store_id, current_store_id
                );
            }
            // Already subscribed during connection setup
        }
        ClientMessage::Unsubscribe { store_id } => {
            manager.unsubscribe(&store_id, connection_id).await;
        }
        ClientMessage::Ping => {
            // Pong will be sent automatically by the broadcast system
            // if we had the connection's sender here
        }
    }

    Ok(())
}
