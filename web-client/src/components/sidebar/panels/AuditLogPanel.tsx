import React, { useState } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { SidebarPanelProps } from "../types";

import { useEventLog } from "@/hooks/useNotebook";
import { Clock, Hash, Filter, Search, RefreshCw } from "lucide-react";

export const AuditLogPanel: React.FC<SidebarPanelProps> = ({
  notebookId,
  onUpdate,
}) => {
  const [searchTerm, setSearchTerm] = useState("");
  const [filterEventType, setFilterEventType] = useState("");
  const [showPayload, setShowPayload] = useState<Record<string, boolean>>({});

  // Use the event log hook
  const { events, totalEvents, eventsByType } = useEventLog();

  // Filter events based on search and filter criteria
  const filteredEvents = events.filter((event) => {
    const matchesSearch =
      searchTerm === "" ||
      event.event_type.toLowerCase().includes(searchTerm.toLowerCase()) ||
      event.id.toLowerCase().includes(searchTerm.toLowerCase()) ||
      JSON.stringify(event.payload)
        .toLowerCase()
        .includes(searchTerm.toLowerCase());

    const matchesFilter =
      filterEventType === "" || event.event_type === filterEventType;

    return matchesSearch && matchesFilter;
  });

  // Toggle payload visibility
  const togglePayload = (eventId: string) => {
    setShowPayload((prev) => ({
      ...prev,
      [eventId]: !prev[eventId],
    }));
  };

  // Format timestamp
  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  // Get event type badge color
  const getEventTypeBadgeColor = (eventType: string) => {
    const colors = {
      NotebookMetadataSet: "bg-blue-100 text-blue-800",
      CellCreated: "bg-green-100 text-green-800",
      CellSourceUpdated: "bg-yellow-100 text-yellow-800",
      CellExecutionStateChanged: "bg-purple-100 text-purple-800",
      CellOutputCreated: "bg-indigo-100 text-indigo-800",
      CellMoved: "bg-orange-100 text-orange-800",
      CellDeleted: "bg-red-100 text-red-800",
    };
    return (
      colors[eventType as keyof typeof colors] || "bg-gray-100 text-gray-800"
    );
  };

  return (
    <div className="space-y-4">
      {/* Header with stats */}
      <div className="flex items-center justify-between">
        <div>
          <h4 className="font-medium text-gray-900">Event History</h4>
          <p className="text-sm text-gray-600">
            {filteredEvents.length} of {totalEvents} events
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={onUpdate}
          className="flex items-center gap-2"
        >
          <RefreshCw className="h-4 w-4" />
          Refresh
        </Button>
      </div>

      {/* Search and filter controls */}
      <div className="space-y-2">
        <div className="relative">
          <Search className="absolute left-3 top-3 h-4 w-4 text-gray-400" />
          <Input
            placeholder="Search events..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-9"
          />
        </div>

        <select
          value={filterEventType}
          onChange={(e) => setFilterEventType(e.target.value)}
          className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">All event types</option>
          {Object.keys(eventsByType)
            .sort()
            .map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
        </select>
      </div>

      {/* Events list */}
      <div className="space-y-3 max-h-96 overflow-y-auto">
        {filteredEvents.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            {totalEvents === 0 ? (
              <div>
                <Clock className="h-8 w-8 mx-auto mb-2 text-gray-400" />
                <p>No events yet</p>
                <p className="text-sm">
                  Events will appear here as you interact with the notebook
                </p>
              </div>
            ) : (
              <div>
                <Filter className="h-8 w-8 mx-auto mb-2 text-gray-400" />
                <p>No events match your filters</p>
              </div>
            )}
          </div>
        ) : (
          filteredEvents
            .slice()
            .reverse()
            .map((event) => (
              <Card key={event.id} className="border-l-2 border-l-blue-500">
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span
                        className={`px-2 py-1 text-xs font-medium rounded-full ${getEventTypeBadgeColor(event.event_type)}`}
                      >
                        {event.event_type}
                      </span>
                      <div className="flex items-center gap-1 text-xs text-gray-500">
                        <Hash className="h-3 w-3" />
                        <span>v{event.version}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-1 text-xs text-gray-500">
                      <Clock className="h-3 w-3" />
                      <span>{formatTimestamp(event.timestamp)}</span>
                    </div>
                  </div>
                  <CardDescription className="text-xs font-mono">
                    ID: {event.id}
                  </CardDescription>
                </CardHeader>

                <CardContent className="pt-0">
                  {/* Key payload info */}
                  <div className="space-y-1 mb-2">
                    {event.payload.cell_id && (
                      <div className="text-xs text-gray-600">
                        <span className="font-medium">Cell:</span>{" "}
                        {event.payload.cell_id}
                      </div>
                    )}
                    {event.payload.title && (
                      <div className="text-xs text-gray-600">
                        <span className="font-medium">Title:</span>{" "}
                        {event.payload.title}
                      </div>
                    )}
                    {event.payload.source && (
                      <div className="text-xs text-gray-600">
                        <span className="font-medium">Source:</span>{" "}
                        {event.payload.source.length > 50
                          ? `${event.payload.source.substring(0, 50)}...`
                          : event.payload.source}
                      </div>
                    )}
                  </div>

                  {/* Expandable payload */}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => togglePayload(event.id)}
                    className="h-6 px-2 text-xs"
                  >
                    {showPayload[event.id] ? "Hide" : "Show"} payload
                  </Button>

                  {showPayload[event.id] && (
                    <pre className="mt-2 text-xs bg-gray-50 p-2 rounded border overflow-x-auto">
                      {JSON.stringify(event.payload, null, 2)}
                    </pre>
                  )}
                </CardContent>
              </Card>
            ))
        )}
      </div>

      {/* Footer stats */}
      {totalEvents > 0 && (
        <div className="pt-2 border-t text-xs text-gray-500 space-y-1">
          <div className="flex justify-between">
            <span>Total Events:</span>
            <span>{totalEvents}</span>
          </div>
          <div className="flex justify-between">
            <span>Latest Version:</span>
            <span>
              {events.length > 0
                ? Math.max(...events.map((e) => e.version))
                : 0}
            </span>
          </div>
          <div className="flex justify-between">
            <span>Store ID:</span>
            <span className="font-mono">{notebookId}</span>
          </div>
        </div>
      )}
    </div>
  );
};
