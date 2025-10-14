import React, { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { SidebarPanelProps } from "../types";
import { api } from "@/lib/api";
import {
  Server,
  Database,
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Activity,
  Zap,
  Bug,
  Trash2,
} from "lucide-react";

interface SystemHealth {
  isConnected: boolean;
  serverStatus: "healthy" | "error" | "unknown";
  lastPing: number | null;
  responseTime: number | null;
  apiEndpoint: string;
}

export const DebugPanel: React.FC<SidebarPanelProps> = ({
  notebookId,
  notebookState,
  onUpdate,
}) => {
  const [health, setHealth] = useState<SystemHealth>({
    isConnected: false,
    serverStatus: "unknown",
    lastPing: null,
    responseTime: null,
    apiEndpoint: window.location.origin,
  });
  const [isChecking, setIsChecking] = useState(false);

  // Check server health
  const checkServerHealth = async () => {
    setIsChecking(true);
    const startTime = Date.now();

    try {
      await api.healthCheck();
      const responseTime = Date.now() - startTime;

      setHealth((prev) => ({
        ...prev,
        isConnected: true,
        serverStatus: "healthy",
        lastPing: Date.now(),
        responseTime,
      }));
    } catch (error) {
      setHealth((prev) => ({
        ...prev,
        isConnected: false,
        serverStatus: "error",
        lastPing: Date.now(),
        responseTime: null,
      }));
    } finally {
      setIsChecking(false);
    }
  };

  // Auto-check health on mount
  useEffect(() => {
    checkServerHealth();
  }, []);

  // Format timestamp
  const formatTimestamp = (timestamp: number | null) => {
    if (!timestamp) return "Never";
    return new Date(timestamp).toLocaleTimeString();
  };

  // Get connection status indicator
  const getConnectionStatus = () => {
    if (isChecking) {
      return {
        icon: <RefreshCw className="h-4 w-4 animate-spin" />,
        text: "Checking...",
        color: "text-gray-500",
      };
    }

    if (health.isConnected && health.serverStatus === "healthy") {
      return {
        icon: <CheckCircle className="h-4 w-4" />,
        text: "Connected",
        color: "text-green-600",
      };
    }

    return {
      icon: <AlertCircle className="h-4 w-4" />,
      text: "Disconnected",
      color: "text-red-600",
    };
  };

  const connectionStatus = getConnectionStatus();

  return (
    <div className="space-y-4">
      {/* Connection Status */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4" />
              <CardTitle className="text-sm">Connection Status</CardTitle>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={checkServerHealth}
              disabled={isChecking}
              className="h-6 w-6 p-0"
            >
              <RefreshCw
                className={`h-3 w-3 ${isChecking ? "animate-spin" : ""}`}
              />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              {connectionStatus.icon}
              <span className={`text-sm font-medium ${connectionStatus.color}`}>
                {connectionStatus.text}
              </span>
            </div>
            {health.responseTime && (
              <span className="text-xs text-gray-500">
                {health.responseTime}ms
              </span>
            )}
          </div>

          <div className="space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-gray-600">Server:</span>
              <span className="font-mono">{health.apiEndpoint}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Last Check:</span>
              <span>{formatTimestamp(health.lastPing)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Status:</span>
              <span
                className={`font-medium ${
                  health.serverStatus === "healthy"
                    ? "text-green-600"
                    : health.serverStatus === "error"
                      ? "text-red-600"
                      : "text-gray-500"
                }`}
              >
                {health.serverStatus}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Notebook State Debug */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            <CardTitle className="text-sm">Local State</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="p-2 bg-gray-50 rounded">
              <div className="font-medium text-gray-600">Store ID</div>
              <div className="font-mono text-gray-900 truncate">
                {notebookId}
              </div>
            </div>
            <div className="p-2 bg-gray-50 rounded">
              <div className="font-medium text-gray-600">Cells</div>
              <div className="font-mono text-gray-900">
                {Object.keys(notebookState.cells).length}
              </div>
            </div>
            <div className="p-2 bg-gray-50 rounded">
              <div className="font-medium text-gray-600">Outputs</div>
              <div className="font-mono text-gray-900">
                {Object.keys(notebookState.outputs).length}
              </div>
            </div>
            <div className="p-2 bg-gray-50 rounded">
              <div className="font-medium text-gray-600">Last Sync</div>
              <div className="font-mono text-gray-900">
                {notebookState.lastProcessedTimestamp
                  ? formatTimestamp(notebookState.lastProcessedTimestamp * 1000)
                  : "Never"}
              </div>
            </div>
          </div>

          {/* Document status */}
          <div className="pt-2 border-t">
            <div className="flex items-center justify-between text-xs">
              <span className="text-gray-600">Document:</span>
              <span
                className={`font-medium ${notebookState.document ? "text-green-600" : "text-red-600"}`}
              >
                {notebookState.document ? "Loaded" : "Missing"}
              </span>
            </div>
            {notebookState.document && (
              <div className="mt-1 text-xs text-gray-600">
                {notebookState.document.title || "Untitled"}
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Performance Metrics */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <Zap className="h-4 w-4" />
            <CardTitle className="text-sm">Performance</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-gray-600">Memory Usage:</span>
              <span className="font-mono">
                {(performance as any).memory
                  ? `${Math.round((performance as any).memory.usedJSHeapSize / 1024 / 1024)}MB`
                  : "N/A"}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Page Load:</span>
              <span className="font-mono">
                {Math.round(
                  performance.timing.loadEventEnd -
                    performance.timing.navigationStart,
                )}
                ms
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">User Agent:</span>
              <span
                className="font-mono truncate text-right max-w-32"
                title={navigator.userAgent}
              >
                {navigator.userAgent.split(" ")[0]}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Debug Actions */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <Bug className="h-4 w-4" />
            <CardTitle className="text-sm">Debug Actions</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-2">
          <Button
            variant="outline"
            size="sm"
            onClick={checkServerHealth}
            className="w-full justify-start"
            disabled={isChecking}
          >
            <Server className="h-4 w-4 mr-2" />
            Test Connection
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={onUpdate}
            className="w-full justify-start"
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            Reload State
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              localStorage.clear();
              sessionStorage.clear();
              window.location.reload();
            }}
            className="w-full justify-start text-orange-600 hover:text-orange-700"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Clear Cache
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              const debugInfo = {
                notebookId,
                notebookState,
                health,
                userAgent: navigator.userAgent,
                timestamp: new Date().toISOString(),
              };
              console.log("EventBook Debug Info:", debugInfo);
              navigator.clipboard?.writeText(
                JSON.stringify(debugInfo, null, 2),
              );
            }}
            className="w-full justify-start"
          >
            <Bug className="h-4 w-4 mr-2" />
            Copy Debug Info
          </Button>
        </CardContent>
      </Card>

      {/* Environment Info */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Environment</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-xs">
          <div className="flex justify-between">
            <span className="text-gray-600">Mode:</span>
            <span className="font-mono">{import.meta.env.MODE}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-600">Dev Tools:</span>
            <span className="font-mono">
              {import.meta.env.DEV ? "Yes" : "No"}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-600">Base URL:</span>
            <span className="font-mono">{import.meta.env.BASE_URL}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-600">Timestamp:</span>
            <span className="font-mono">{new Date().toLocaleTimeString()}</span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};
