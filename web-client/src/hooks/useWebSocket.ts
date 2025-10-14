import { useEffect, useRef, useState, useCallback } from "react";
import type { Event } from "@/types/eventbook";

// WebSocket message types (matching server-side)
export interface WsMessage {
  type: "event" | "store_info" | "subscribed" | "error" | "ping" | "pong";
  store_id?: string;
  event?: Event;
  event_count?: number;
  latest_version?: number;
  connection_id?: string;
  message?: string;
}

export interface ClientMessage {
  type: "subscribe" | "unsubscribe" | "ping";
  store_id?: string;
}

export interface UseWebSocketOptions {
  enabled?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  pingInterval?: number;
}

export interface UseWebSocketReturn {
  connectionStatus: "connecting" | "connected" | "disconnected" | "error";
  connectionId: string | null;
  lastEvent: Event | null;
  eventCount: number;
  isConnected: boolean;
  error: string | null;
  reconnectAttempts: number;
  connect: () => void;
  disconnect: () => void;
}

const DEFAULT_OPTIONS: Required<UseWebSocketOptions> = {
  enabled: true,
  reconnectInterval: 3000,
  maxReconnectAttempts: 5,
  pingInterval: 30000,
};

export function useWebSocket(
  storeId: string,
  options: UseWebSocketOptions = {},
): UseWebSocketReturn {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  // State
  const [connectionStatus, setConnectionStatus] = useState<
    "connecting" | "connected" | "disconnected" | "error"
  >("disconnected");
  const [connectionId, setConnectionId] = useState<string | null>(null);
  const [lastEvent, setLastEvent] = useState<Event | null>(null);
  const [eventCount, setEventCount] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [reconnectAttempts, setReconnectAttempts] = useState(0);

  // Refs
  const ws = useRef<WebSocket | null>(null);
  const pingInterval = useRef<number | null>(null);
  const reconnectTimeout = useRef<number | null>(null);
  const shouldReconnect = useRef(true);

  // WebSocket URL (convert http to ws, https to wss)
  const getWebSocketUrl = useCallback(() => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    return `${protocol}//${host}/stores/${encodeURIComponent(storeId)}/ws`;
  }, [storeId]);

  // Send message to WebSocket
  const sendMessage = useCallback((message: ClientMessage) => {
    if (ws.current && ws.current.readyState === WebSocket.OPEN) {
      try {
        ws.current.send(JSON.stringify(message));
      } catch (err) {
        console.error("Failed to send WebSocket message:", err);
      }
    }
  }, []);

  // Start ping interval
  const startPing = useCallback(() => {
    if (pingInterval.current) {
      clearInterval(pingInterval.current);
    }

    pingInterval.current = setInterval(() => {
      sendMessage({ type: "ping" });
    }, opts.pingInterval);
  }, [sendMessage, opts.pingInterval]);

  // Stop ping interval
  const stopPing = useCallback(() => {
    if (pingInterval.current) {
      clearInterval(pingInterval.current);
      pingInterval.current = null;
    }
  }, []);

  // Handle incoming messages
  const handleMessage = useCallback(
    (event: MessageEvent) => {
      try {
        const message: WsMessage = JSON.parse(event.data);

        switch (message.type) {
          case "subscribed":
            setConnectionStatus("connected");
            setConnectionId(message.connection_id || null);
            setError(null);
            setReconnectAttempts(0);
            startPing();
            console.log("WebSocket subscribed:", message.connection_id);
            break;

          case "event":
            if (message.event) {
              setLastEvent(message.event);
              console.log(
                "WebSocket event received:",
                message.event.event_type,
              );
            }
            break;

          case "store_info":
            if (typeof message.event_count === "number") {
              setEventCount(message.event_count);
            }
            break;

          case "error":
            setError(message.message || "Unknown WebSocket error");
            setConnectionStatus("error");
            console.error("WebSocket error:", message.message);
            break;

          case "ping":
            // Respond with pong
            sendMessage({ type: "ping" }); // Server expects ping back as pong
            break;

          case "pong":
            // Heartbeat received
            break;

          default:
            console.warn("Unknown WebSocket message type:", message);
        }
      } catch (err) {
        console.error("Failed to parse WebSocket message:", err);
        setError("Failed to parse server message");
      }
    },
    [sendMessage, startPing],
  );

  // Connect to WebSocket
  const connect = useCallback(() => {
    if (ws.current && ws.current.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    if (!opts.enabled) {
      return;
    }

    setConnectionStatus("connecting");
    setError(null);

    try {
      const wsUrl = getWebSocketUrl();
      console.log("Connecting to WebSocket:", wsUrl);

      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        console.log("WebSocket connection opened");
        // Connection confirmation will come via 'subscribed' message
      };

      ws.current.onmessage = handleMessage;

      ws.current.onclose = (event) => {
        console.log("WebSocket connection closed:", event.code, event.reason);
        setConnectionStatus("disconnected");
        setConnectionId(null);
        stopPing();

        // Attempt reconnection if enabled and not a normal closure
        if (
          shouldReconnect.current &&
          reconnectAttempts < opts.maxReconnectAttempts
        ) {
          setReconnectAttempts((prev) => prev + 1);
          reconnectTimeout.current = setTimeout(() => {
            connect();
          }, opts.reconnectInterval);
        }
      };

      ws.current.onerror = (error) => {
        console.error("WebSocket error:", error);
        setConnectionStatus("error");
        setError("WebSocket connection failed");
      };
    } catch (err) {
      console.error("Failed to create WebSocket connection:", err);
      setConnectionStatus("error");
      setError(err instanceof Error ? err.message : "Connection failed");
    }
  }, [
    opts.enabled,
    opts.maxReconnectAttempts,
    opts.reconnectInterval,
    getWebSocketUrl,
    handleMessage,
    reconnectAttempts,
    stopPing,
  ]);

  // Disconnect from WebSocket
  const disconnect = useCallback(() => {
    shouldReconnect.current = false;

    if (reconnectTimeout.current) {
      clearTimeout(reconnectTimeout.current);
      reconnectTimeout.current = null;
    }

    stopPing();

    if (ws.current) {
      ws.current.close(1000, "Client disconnect");
      ws.current = null;
    }

    setConnectionStatus("disconnected");
    setConnectionId(null);
    setReconnectAttempts(0);
  }, [stopPing]);

  // Effect to handle connection lifecycle
  useEffect(() => {
    if (opts.enabled && storeId) {
      shouldReconnect.current = true;
      connect();
    }

    return () => {
      disconnect();
    };
  }, [storeId, opts.enabled]); // Don't include connect/disconnect in deps to avoid loops

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      shouldReconnect.current = false;
      disconnect();
    };
  }, []);

  return {
    connectionStatus,
    connectionId,
    lastEvent,
    eventCount,
    isConnected: connectionStatus === "connected",
    error,
    reconnectAttempts,
    connect,
    disconnect,
  };
}
