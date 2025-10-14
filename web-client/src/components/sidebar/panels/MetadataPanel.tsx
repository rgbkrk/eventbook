import React from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { SidebarPanelProps } from "../types";
import {
  FileText,
  User,
  Tag,
  Calendar,
  Clock,
  Code2,
  Hash,
  Play,
  CheckCircle,
  XCircle,
  Loader,
  Edit3,
} from "lucide-react";

export const MetadataPanel: React.FC<SidebarPanelProps> = ({
  notebookId,
  notebookState,
  onUpdate,
}) => {
  const { document, cells } = notebookState;

  // Calculate statistics
  const stats = {
    totalCells: Object.keys(cells).length,
    codeCells: Object.values(cells).filter((c) => c.cell_type === "code")
      .length,
    markdownCells: Object.values(cells).filter(
      (c) => c.cell_type === "markdown",
    ).length,
    sqlCells: Object.values(cells).filter((c) => c.cell_type === "sql").length,
    aiCells: Object.values(cells).filter((c) => c.cell_type === "ai").length,

    // Execution stats
    idleCells: Object.values(cells).filter((c) => c.execution_state === "idle")
      .length,
    queuedCells: Object.values(cells).filter(
      (c) => c.execution_state === "queued",
    ).length,
    runningCells: Object.values(cells).filter(
      (c) => c.execution_state === "running",
    ).length,
    completedCells: Object.values(cells).filter(
      (c) => c.execution_state === "completed",
    ).length,
    errorCells: Object.values(cells).filter(
      (c) => c.execution_state === "error",
    ).length,
  };

  // Format date
  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  // Get execution state icon
  const getExecutionIcon = (state: string, count: number) => {
    if (count === 0) return null;

    switch (state) {
      case "idle":
        return <Hash className="h-4 w-4 text-gray-500" />;
      case "queued":
        return <Clock className="h-4 w-4 text-yellow-500" />;
      case "running":
        return <Loader className="h-4 w-4 text-blue-500 animate-spin" />;
      case "completed":
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "error":
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return null;
    }
  };

  return (
    <div className="space-y-4">
      {/* Document Info */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            <CardTitle className="text-sm">Document Information</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          {document ? (
            <>
              <div>
                <label className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                  Title
                </label>
                <p className="mt-1 text-sm font-medium text-gray-900">
                  {document.title || "Untitled Notebook"}
                </p>
              </div>

              {document.metadata.authors.length > 0 && (
                <div>
                  <label className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                    Authors
                  </label>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {document.metadata.authors.map((author, idx) => (
                      <span
                        key={idx}
                        className="inline-flex items-center gap-1 px-2 py-1 text-xs bg-gray-100 text-gray-800 rounded-md"
                      >
                        <User className="h-3 w-3" />
                        {author}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {document.metadata.tags.length > 0 && (
                <div>
                  <label className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                    Tags
                  </label>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {document.metadata.tags.map((tag, idx) => (
                      <span
                        key={idx}
                        className="inline-flex items-center gap-1 px-2 py-1 text-xs bg-blue-100 text-blue-800 rounded-md"
                      >
                        <Tag className="h-3 w-3" />
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              <div className="grid grid-cols-2 gap-3 pt-2 border-t text-xs">
                <div>
                  <label className="font-medium text-gray-500">Created</label>
                  <div className="flex items-center gap-1 mt-1">
                    <Calendar className="h-3 w-3 text-gray-400" />
                    <span>{formatDate(document.created_at)}</span>
                  </div>
                </div>
                <div>
                  <label className="font-medium text-gray-500">Modified</label>
                  <div className="flex items-center gap-1 mt-1">
                    <Edit3 className="h-3 w-3 text-gray-400" />
                    <span>{formatDate(document.updated_at)}</span>
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="text-center py-4 text-gray-500">
              <FileText className="h-8 w-8 mx-auto mb-2 text-gray-400" />
              <p className="text-sm">No document metadata</p>
              <p className="text-xs">Initialize the notebook to set metadata</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Cell Statistics */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Cell Statistics</CardTitle>
          <CardDescription className="text-xs">
            Overview of cells in this notebook
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {stats.totalCells > 0 ? (
            <>
              {/* Cell type breakdown */}
              <div className="grid grid-cols-2 gap-2 text-xs">
                {stats.codeCells > 0 && (
                  <div className="flex items-center justify-between p-2 bg-gray-50 rounded">
                    <div className="flex items-center gap-2">
                      <Code2 className="h-3 w-3 text-blue-600" />
                      <span>Code</span>
                    </div>
                    <span className="font-medium">{stats.codeCells}</span>
                  </div>
                )}

                {stats.markdownCells > 0 && (
                  <div className="flex items-center justify-between p-2 bg-gray-50 rounded">
                    <div className="flex items-center gap-2">
                      <FileText className="h-3 w-3 text-green-600" />
                      <span>Markdown</span>
                    </div>
                    <span className="font-medium">{stats.markdownCells}</span>
                  </div>
                )}

                {stats.sqlCells > 0 && (
                  <div className="flex items-center justify-between p-2 bg-gray-50 rounded">
                    <div className="flex items-center gap-2">
                      <Hash className="h-3 w-3 text-purple-600" />
                      <span>SQL</span>
                    </div>
                    <span className="font-medium">{stats.sqlCells}</span>
                  </div>
                )}

                {stats.aiCells > 0 && (
                  <div className="flex items-center justify-between p-2 bg-gray-50 rounded">
                    <div className="flex items-center gap-2">
                      <Play className="h-3 w-3 text-indigo-600" />
                      <span>AI</span>
                    </div>
                    <span className="font-medium">{stats.aiCells}</span>
                  </div>
                )}
              </div>

              {/* Execution state breakdown */}
              <div className="pt-2 border-t">
                <label className="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2 block">
                  Execution States
                </label>
                <div className="space-y-1">
                  {(
                    ["idle", "queued", "running", "completed", "error"] as const
                  ).map((state) => {
                    const count = stats[
                      `${state}Cells` as keyof typeof stats
                    ] as number;
                    if (count === 0) return null;

                    return (
                      <div
                        key={state}
                        className="flex items-center justify-between text-xs"
                      >
                        <div className="flex items-center gap-2">
                          {getExecutionIcon(state, count)}
                          <span className="capitalize">{state}</span>
                        </div>
                        <span className="font-medium">{count}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          ) : (
            <div className="text-center py-4 text-gray-500">
              <Code2 className="h-8 w-8 mx-auto mb-2 text-gray-400" />
              <p className="text-sm">No cells yet</p>
              <p className="text-xs">Add cells to see statistics</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Notebook Settings */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Notebook Settings</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div>
            <label className="text-xs font-medium text-gray-700 block mb-1">
              Store ID
            </label>
            <Input
              value={notebookId}
              readOnly
              className="text-xs font-mono bg-gray-50"
            />
          </div>

          {document?.metadata.kernel_spec && (
            <div>
              <label className="text-xs font-medium text-gray-700 block mb-1">
                Kernel
              </label>
              <div className="p-2 bg-gray-50 rounded text-xs">
                <div className="font-medium">
                  {document.metadata.kernel_spec.display_name}
                </div>
                <div className="text-gray-600">
                  {document.metadata.kernel_spec.language}
                </div>
              </div>
            </div>
          )}

          <div className="pt-2 border-t">
            <Button
              variant="outline"
              size="sm"
              onClick={onUpdate}
              className="w-full"
            >
              Refresh Data
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">Quick Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <Button variant="outline" size="sm" className="w-full justify-start">
            Export Notebook
          </Button>
          <Button variant="outline" size="sm" className="w-full justify-start">
            Share Notebook
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="w-full justify-start text-red-600 hover:text-red-700"
          >
            Clear All Outputs
          </Button>
        </CardContent>
      </Card>
    </div>
  );
};
