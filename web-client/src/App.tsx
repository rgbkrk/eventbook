import { useParams } from "react-router-dom";
import { EventBookProvider } from "@/providers/EventBookProvider";
import { NotebookView } from "@/components/NotebookView";

function App() {
  const { notebookId: paramNotebookId } = useParams<{ notebookId?: string }>();
  const notebookId = paramNotebookId || "demo-notebook";

  return (
    <EventBookProvider notebookId={notebookId}>
      <NotebookView />
    </EventBookProvider>
  );
}

export default App;
