import React, { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { api, eventOperations, generateId } from '@/lib/api';
import type { Event, NotebookState, Cell } from '@/types/eventbook';
import { Play, Plus, Save, FileText, Code2, Hash } from 'lucide-react';

function App() {
  const [notebookId, setNotebookId] = useState('demo-notebook');
  const [notebookState, setNotebookState] = useState<NotebookState>({
    cells: {},
    outputs: {},
    orderedCells: [],
    lastProcessedTimestamp: 0,
  });
  const [events, setEvents] = useState<Event[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load events and reconstruct notebook state
  const loadNotebook = async () => {
    if (!notebookId.trim()) return;

    setIsLoading(true);
    setError(null);

    try {
      const response = await api.getEvents(notebookId);
      setEvents(response.events);

      // Simple state reconstruction (bare minimum)
      const newState: NotebookState = {
        cells: {},
        outputs: {},
        orderedCells: [],
        lastProcessedTimestamp: 0,
      };

      // Process events to rebuild state
      for (const event of response.events) {
        switch (event.event_type) {
          case 'NotebookMetadataSet':
            newState.document = {
              id: event.aggregate_id,
              title: event.payload.title || 'Untitled',
              metadata: {
                authors: event.payload.authors || [],
                tags: event.payload.tags || [],
                custom: {},
              },
              created_at: event.timestamp,
              updated_at: event.timestamp,
            };
            break;

          case 'CellCreated':
            const cell: Cell = {
              id: event.payload.cell_id,
              cell_type: event.payload.cell_type,
              source: event.payload.source || '',
              fractional_index: event.payload.fractional_index,
              execution_state: 'idle',
              source_visible: true,
              output_visible: true,
              ai_context_visible: true,
              created_by: event.payload.created_by || 'user',
              document_id: event.aggregate_id,
              created_at: event.timestamp,
              updated_at: event.timestamp,
            };
            newState.cells[cell.id] = cell;
            break;

          case 'CellSourceUpdated':
            if (newState.cells[event.payload.cell_id]) {
              newState.cells[event.payload.cell_id].source = event.payload.source;
              newState.cells[event.payload.cell_id].updated_at = event.timestamp;
            }
            break;

          case 'CellExecutionStateChanged':
            if (newState.cells[event.payload.cell_id]) {
              newState.cells[event.payload.cell_id].execution_state = event.payload.execution_state;
              newState.cells[event.payload.cell_id].updated_at = event.timestamp;
            }
            break;
        }
        newState.lastProcessedTimestamp = Math.max(newState.lastProcessedTimestamp, event.timestamp);
      }

      // Order cells by fractional index
      newState.orderedCells = Object.values(newState.cells).sort((a, b) => {
        const aIndex = a.fractional_index || 'z';
        const bIndex = b.fractional_index || 'z';
        return aIndex.localeCompare(bIndex);
      });

      setNotebookState(newState);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load notebook');
    } finally {
      setIsLoading(false);
    }
  };

  // Submit an event
  const submitEvent = async (eventRequest: { event_type: string; payload: any }) => {
    try {
      setError(null);
      await api.submitEvent(notebookId, eventRequest);
      // Reload to see the changes
      await loadNotebook();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit event');
    }
  };

  // Initialize notebook
  const initializeNotebook = async () => {
    const event = eventOperations.createDocument(`Notebook: ${notebookId}`, {
      authors: ['demo-user'],
      tags: ['demo'],
    });
    await submitEvent(event);
  };

  // Add a new cell
  const addCell = async (cellType: 'code' | 'markdown') => {
    const cellId = generateId('cell');
    const fractionalIndex = `a${Date.now()}`;
    const event = eventOperations.createCell(cellId, cellType, '', fractionalIndex, 'demo-user');
    await submitEvent(event);
  };

  // Update cell source
  const updateCellSource = async (cellId: string, source: string) => {
    const event = eventOperations.updateCellSource(cellId, source);
    await submitEvent(event);
  };

  // Execute cell (mock)
  const executeCell = async (cellId: string) => {
    // Start execution
    await submitEvent(eventOperations.updateExecutionState(cellId, 'running'));

    // Simulate execution completion
    setTimeout(async () => {
      await submitEvent(eventOperations.updateExecutionState(cellId, 'completed', 'runtime-1', 42));
    }, 1000);
  };

  useEffect(() => {
    loadNotebook();
  }, []);

  const getCellIcon = (cellType: string) => {
    switch (cellType) {
      case 'code': return <Code2 className="h-4 w-4" />;
      case 'markdown': return <FileText className="h-4 w-4" />;
      case 'sql': return <Hash className="h-4 w-4" />;
      default: return <FileText className="h-4 w-4" />;
    }
  };

  return (
    <div className="container mx-auto p-6 max-w-6xl">
      <div className="mb-6">
        <h1 className="text-3xl font-bold mb-2">EventBook</h1>
        <p className="text-muted-foreground">Event-sourced notebook with local-first collaboration</p>
      </div>

      {error && (
        <Card className="mb-4 border-destructive">
          <CardContent className="pt-6">
            <p className="text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Notebook */}
        <div className="lg:col-span-2 space-y-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle>Notebook: {notebookId}</CardTitle>
                <CardDescription>
                  {notebookState.document?.title || 'No title set'}
                </CardDescription>
              </div>
              <div className="flex gap-2">
                <Input
                  value={notebookId}
                  onChange={(e) => setNotebookId(e.target.value)}
                  placeholder="notebook-id"
                  className="w-40"
                />
                <Button onClick={loadNotebook} disabled={isLoading}>
                  Load
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 mb-4">
                <Button onClick={initializeNotebook} size="sm" variant="outline">
                  Initialize
                </Button>
                <Button onClick={() => addCell('code')} size="sm">
                  <Code2 className="h-4 w-4 mr-2" />
                  Add Code
                </Button>
                <Button onClick={() => addCell('markdown')} size="sm" variant="outline">
                  <FileText className="h-4 w-4 mr-2" />
                  Add Markdown
                </Button>
              </div>

              {/* Cells */}
              <div className="space-y-4">
                {notebookState.orderedCells.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    No cells yet. Add one to get started!
                  </div>
                ) : (
                  notebookState.orderedCells.map((cell) => (
                    <Card key={cell.id} className="border-l-4 border-l-primary">
                      <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            {getCellIcon(cell.cell_type)}
                            <span className="text-sm font-medium">
                              {cell.cell_type} • {cell.id}
                            </span>
                            <span className={`text-xs px-2 py-1 rounded ${
                              cell.execution_state === 'running' ? 'bg-yellow-100 text-yellow-800' :
                              cell.execution_state === 'completed' ? 'bg-green-100 text-green-800' :
                              'bg-gray-100 text-gray-800'
                            }`}>
                              {cell.execution_state}
                            </span>
                          </div>
                          {cell.cell_type === 'code' && (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => executeCell(cell.id)}
                              disabled={cell.execution_state === 'running'}
                            >
                              <Play className="h-4 w-4" />
                            </Button>
                          )}
                        </div>
                      </CardHeader>
                      <CardContent>
                        <textarea
                          value={cell.source}
                          onChange={(e) => updateCellSource(cell.id, e.target.value)}
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

        {/* Event Log */}
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Event Log</CardTitle>
              <CardDescription>{events.length} events</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2 max-h-96 overflow-y-auto">
                {events.length === 0 ? (
                  <p className="text-muted-foreground text-sm">No events yet</p>
                ) : (
                  events.slice().reverse().map((event, idx) => (
                    <Card key={event.id} className="p-3">
                      <div className="flex justify-between items-start mb-2">
                        <span className="text-xs font-medium bg-primary/10 text-primary px-2 py-1 rounded">
                          {event.event_type}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          v{event.version}
                        </span>
                      </div>
                      <pre className="text-xs text-muted-foreground bg-muted p-2 rounded overflow-hidden">
                        {JSON.stringify(event.payload, null, 2)}
                      </pre>
                    </Card>
                  ))
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Stats</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>Cells:</span>
                <span>{Object.keys(notebookState.cells).length}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Events:</span>
                <span>{events.length}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span>Last Updated:</span>
                <span>
                  {notebookState.lastProcessedTimestamp
                    ? new Date(notebookState.lastProcessedTimestamp * 1000).toLocaleTimeString()
                    : 'Never'
                  }
                </span>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

export default App;
