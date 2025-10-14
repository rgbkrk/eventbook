import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { EventBookSidebar } from "@/components/sidebar/EventBookSidebar";
import { useNotebook, useCollaboration, useCells } from "@/hooks/useNotebook";
import {
  Play,
  FileText,
  Code2,
  Hash,
  Wifi,
  WifiOff,
  Loader,
} from "lucide-react";

export function NotebookView() {
  const navigate = useNavigate();

  // Use our clean hooks
  const {
    notebookId,
    notebookState,
    title,
    isInitialized,
    isLoading,
    error,
    initialize,
    reload,
  } = useNotebook();

  const {
    isConnected,
    connectionId,
    error: wsError,
    statusText,
  } = useCollaboration();

  const {
    orderedCells,
    isEmpty,
    create: createCell,
    updateSource: updateCellSource,
    execute: executeCell,
  } = useCells();

  // Cell type icon helper
  const getCellIcon = (cellType: string) => {
    switch (cellType) {
      case "code":
        return <Code2 className="h-4 w-4" />;
      case "markdown":
        return <FileText className="h-4 w-4" />;
      case "sql":
        return <Hash className="h-4 w-4" />;
      default:
        return <FileText className="h-4 w-4" />;
    }
  };

  // Handle navigation to different notebook
  const handleNotebookNavigation = (newId: string) => {
    navigate(newId === "demo-notebook" ? "/" : `/notebook/${newId}`);
  };

  return (
    <>
      <EventBookSidebar
        notebookId={notebookId}
        notebookState={notebookState}
        onUpdate={reload}
      />

      <div className="lg:ml-12">
        <div className="container mx-auto p-6 max-w-4xl">
          {/* Header with connection status */}
          <div className="mb-6">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-3xl font-bold mb-2">EventBook</h1>
                <p className="text-muted-foreground">
                  Event-sourced notebook with local-first collaboration
                </p>
              </div>
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-1 text-sm">
                  {isConnected ? (
                    <>
                      <Wifi className="h-4 w-4 text-green-500" />
                      <span className="text-green-600">{statusText}</span>
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-4 w-4 text-red-500" />
                      <span className="text-red-600">{statusText}</span>
                    </>
                  )}
                </div>
                {connectionId && (
                  <span className="text-xs text-muted-foreground font-mono">
                    {connectionId}
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Error display */}
          {(error || wsError) && (
            <Card className="mb-4 border-destructive">
              <CardContent className="pt-6">
                {error && <p className="text-destructive">{error}</p>}
                {wsError && (
                  <p className="text-destructive">WebSocket: {wsError}</p>
                )}
              </CardContent>
            </Card>
          )}

          {/* Main notebook interface */}
          <div className="space-y-4">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between">
                <div>
                  <CardTitle>Notebook: {notebookId}</CardTitle>
                  <CardDescription>
                    {title}
                    {isLoading && (
                      <Loader className="inline-block ml-2 h-3 w-3 animate-spin" />
                    )}
                  </CardDescription>
                </div>
                <div className="flex gap-2">
                  <Input
                    defaultValue={notebookId}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        const newId = (e.target as HTMLInputElement).value;
                        handleNotebookNavigation(newId);
                      }
                    }}
                    placeholder="notebook-id"
                    className="w-40"
                  />
                  <Button onClick={reload} disabled={isLoading}>
                    Load
                  </Button>
                </div>
              </CardHeader>

              <CardContent>
                {/* Action buttons */}
                <div className="flex gap-2 mb-4">
                  {!isInitialized && (
                    <Button
                      onClick={() => initialize(`Notebook: ${notebookId}`)}
                      size="sm"
                      variant="outline"
                    >
                      Initialize
                    </Button>
                  )}
                  <Button
                    onClick={() => createCell("code")}
                    size="sm"
                    disabled={isLoading}
                  >
                    <Code2 className="h-4 w-4 mr-2" />
                    Add Code
                  </Button>
                  <Button
                    onClick={() => createCell("markdown")}
                    size="sm"
                    variant="outline"
                    disabled={isLoading}
                  >
                    <FileText className="h-4 w-4 mr-2" />
                    Add Markdown
                  </Button>
                </div>

                {/* Cell list */}
                <div className="space-y-4">
                  {isEmpty ? (
                    <div className="text-center py-8 text-muted-foreground">
                      {!isInitialized
                        ? "Initialize the notebook to get started!"
                        : "No cells yet. Add one to get started!"}
                    </div>
                  ) : (
                    orderedCells.map((cell) => (
                      <Card
                        key={cell.id}
                        className="border-l-4 border-l-primary"
                      >
                        <CardHeader className="pb-2">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              {getCellIcon(cell.cell_type)}
                              <span className="text-sm font-medium">
                                {cell.cell_type} • {cell.id}
                              </span>
                              <span
                                className={`text-xs px-2 py-1 rounded ${
                                  cell.execution_state === "running"
                                    ? "bg-yellow-100 text-yellow-800"
                                    : cell.execution_state === "completed"
                                      ? "bg-green-100 text-green-800"
                                      : "bg-gray-100 text-gray-800"
                                }`}
                              >
                                {cell.execution_state}
                              </span>
                            </div>
                            {cell.cell_type === "code" && (
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => executeCell(cell.id)}
                                disabled={cell.execution_state === "running"}
                              >
                                <Play className="h-4 w-4" />
                              </Button>
                            )}
                          </div>
                        </CardHeader>
                        <CardContent>
                          <textarea
                            value={cell.source}
                            onChange={(e) =>
                              updateCellSource(cell.id, e.target.value)
                            }
                            placeholder={`Enter ${cell.cell_type} here...`}
                            className="w-full min-h-[100px] p-3 border rounded-md font-mono text-sm resize-vertical"
                          />
                          {cell.fractional_index && (
                            <p className="text-xs text-muted-foreground mt-2">
                              Index: {cell.fractional_index}
                            </p>
                          )}
                        </CardContent>
                      </Card>
                    ))
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </>
  );
}
