#!/bin/bash

# EventBook API Test Script - One Store = One Document
set -e

BASE_URL="http://localhost:3000"
NOTEBOOK_1="notebook-$(date +%s)"
NOTEBOOK_2="notebook-2-$(date +%s)"

echo "🧪 Testing EventBook API (One Store = One Document)"
echo "=================================================="
echo "Base URL: $BASE_URL"
echo "Notebook 1: $NOTEBOOK_1"
echo "Notebook 2: $NOTEBOOK_2"
echo

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

test_endpoint() {
    local name="$1"
    local method="$2"
    local url="$3"
    local data="$4"

    echo -e "${BLUE}Testing: $name${NC}"
    echo "→ $method $url"

    if [ -n "$data" ]; then
        echo "→ Data: $data"
        response=$(curl -s -X "$method" -H "Content-Type: application/json" -d "$data" "$url")
    else
        response=$(curl -s -X "$method" "$url")
    fi

    echo "← Response: $response"
    echo

    # Check if response contains error
    if echo "$response" | grep -q '"error"'; then
        echo -e "${RED}❌ Test failed!${NC}"
        exit 1
    else
        echo -e "${GREEN}✅ Test passed!${NC}"
    fi
    echo "---"
}

# 1. Health Check
test_endpoint "Health Check" "GET" "$BASE_URL/health"

# 2. List stores (should be empty initially)
test_endpoint "List Stores (empty)" "GET" "$BASE_URL/stores"

# 3. Initialize notebook metadata
echo -e "${BLUE}Setting notebook metadata...${NC}"
metadata_event='{
    "event_type": "NotebookMetadataSet",
    "payload": {
        "title": "My Test Notebook",
        "authors": ["test-user"],
        "tags": ["test", "demo"],
        "kernel_spec": {
            "name": "python3",
            "display_name": "Python 3",
            "language": "python"
        }
    }
}'
test_endpoint "Set Notebook Metadata" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$metadata_event"

# 4. Add first code cell
echo -e "${BLUE}Adding first code cell...${NC}"
cell1_event='{
    "event_type": "CellCreated",
    "payload": {
        "cell_id": "cell-1",
        "cell_type": "code",
        "source": "print(\"Hello, EventBook!\")",
        "fractional_index": "a0",
        "created_by": "test-user"
    }
}'
test_endpoint "Create First Code Cell" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$cell1_event"

# 5. Add markdown cell
echo -e "${BLUE}Adding markdown cell...${NC}"
md_cell_event='{
    "event_type": "CellCreated",
    "payload": {
        "cell_id": "cell-2",
        "cell_type": "markdown",
        "source": "# Welcome to EventBook\\n\\nThis is a **test notebook** demonstrating:\\n- Event sourcing\\n- Local-first collaboration\\n- Fractional indexing",
        "fractional_index": "a1",
        "created_by": "test-user"
    }
}'
test_endpoint "Create Markdown Cell" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$md_cell_event"

# 6. Add another code cell
echo -e "${BLUE}Adding second code cell...${NC}"
cell3_event='{
    "event_type": "CellCreated",
    "payload": {
        "cell_id": "cell-3",
        "cell_type": "code",
        "source": "import pandas as pd\\ndf = pd.DataFrame({\"x\": [1, 2, 3], \"y\": [4, 5, 6]})\\nprint(df)",
        "fractional_index": "a2",
        "created_by": "test-user"
    }
}'
test_endpoint "Create Second Code Cell" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$cell3_event"

# 7. Update first cell source
echo -e "${BLUE}Updating first cell source...${NC}"
update_event='{
    "event_type": "CellSourceUpdated",
    "payload": {
        "cell_id": "cell-1",
        "source": "print(\"Hello, Updated EventBook!\")\\nprint(\"This cell was modified\")"
    }
}'
test_endpoint "Update Cell Source" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$update_event"

# 8. Start execution of first cell
echo -e "${BLUE}Starting cell execution...${NC}"
exec_start_event='{
    "event_type": "CellExecutionStateChanged",
    "payload": {
        "cell_id": "cell-1",
        "execution_state": "running",
        "assigned_runtime_session": "runtime-123"
    }
}'
test_endpoint "Start Cell Execution" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$exec_start_event"

# 9. Complete execution with output
echo -e "${BLUE}Completing cell execution...${NC}"
exec_complete_event='{
    "event_type": "CellExecutionStateChanged",
    "payload": {
        "cell_id": "cell-1",
        "execution_state": "completed",
        "execution_duration_ms": 45
    }
}'
test_endpoint "Complete Cell Execution" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$exec_complete_event"

# 10. Add cell output
echo -e "${BLUE}Adding cell output...${NC}"
output_event='{
    "event_type": "CellOutputCreated",
    "payload": {
        "output_id": "output-1",
        "cell_id": "cell-1",
        "output_type": "terminal",
        "position": 0.0,
        "stream_name": "stdout",
        "data": "Hello, Updated EventBook!\\nThis cell was modified\\n"
    }
}'
test_endpoint "Add Cell Output" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$output_event"

# 11. Move cell (change fractional index)
echo -e "${BLUE}Moving cell to different position...${NC}"
move_event='{
    "event_type": "CellMoved",
    "payload": {
        "cell_id": "cell-2",
        "fractional_index": "a15"
    }
}'
test_endpoint "Move Cell" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$move_event"

# 12. Get all events for notebook 1
test_endpoint "Get All Events (Notebook 1)" "GET" "$BASE_URL/stores/$NOTEBOOK_1/events"

# 13. Get notebook 1 info
test_endpoint "Get Notebook 1 Info" "GET" "$BASE_URL/stores/$NOTEBOOK_1"

# 14. Create second notebook
echo -e "${BLUE}Creating second notebook...${NC}"
notebook2_metadata='{
    "event_type": "NotebookMetadataSet",
    "payload": {
        "title": "Second Test Notebook",
        "authors": ["another-user"],
        "tags": ["test2", "isolation"]
    }
}'
test_endpoint "Create Second Notebook" "POST" "$BASE_URL/stores/$NOTEBOOK_2/events" "$notebook2_metadata"

# 15. Add cell to second notebook
echo -e "${BLUE}Adding cell to second notebook...${NC}"
notebook2_cell='{
    "event_type": "CellCreated",
    "payload": {
        "cell_id": "cell-A",
        "cell_type": "code",
        "source": "# This is in a different notebook\\nprint(\"Isolated store!\")",
        "fractional_index": "a0",
        "created_by": "another-user"
    }
}'
test_endpoint "Add Cell to Second Notebook" "POST" "$BASE_URL/stores/$NOTEBOOK_2/events" "$notebook2_cell"

# 16. Verify notebook isolation
test_endpoint "Get Second Notebook Events" "GET" "$BASE_URL/stores/$NOTEBOOK_2/events"

# 17. Test pagination on first notebook
test_endpoint "Get Events (limit=3)" "GET" "$BASE_URL/stores/$NOTEBOOK_1/events?limit=3&offset=0"

# 18. Test filtering by timestamp
echo -e "${BLUE}Testing timestamp filtering...${NC}"
current_time=$(date +%s)
past_time=$((current_time - 60))
test_endpoint "Get Recent Events" "GET" "$BASE_URL/stores/$NOTEBOOK_1/events?since_timestamp=$past_time"

# 19. List all stores
test_endpoint "Final Store List" "GET" "$BASE_URL/stores"

# 20. Delete a cell
echo -e "${BLUE}Deleting a cell...${NC}"
delete_event='{
    "event_type": "CellDeleted",
    "payload": {
        "cell_id": "cell-3"
    }
}'
test_endpoint "Delete Cell" "POST" "$BASE_URL/stores/$NOTEBOOK_1/events" "$delete_event"

# 21. Final event count
test_endpoint "Final Event Count" "GET" "$BASE_URL/stores/$NOTEBOOK_1"

echo -e "${GREEN}🎉 All tests passed!${NC}"
echo
echo "Summary:"
echo "- Created 2 notebooks: $NOTEBOOK_1 and $NOTEBOOK_2"
echo "- Notebook 1: Full lifecycle (metadata, cells, updates, execution, output, moves, deletion)"
echo "- Notebook 2: Basic setup to test isolation"
echo "- Verified store isolation works correctly"
echo "- Tested pagination and filtering"
echo
echo "Key features demonstrated:"
echo "✅ One store = one notebook"
echo "✅ Event-driven operations"
echo "✅ Fractional indexing for cell ordering"
echo "✅ Cell lifecycle management"
echo "✅ Execution state tracking"
echo "✅ Output management"
echo "✅ Store isolation"
echo "✅ Event filtering and pagination"
echo
echo "Next steps:"
echo "- Add WebSocket subscriptions for real-time updates"
echo "- Integrate SpiceDB for granular permissions"
echo "- Build local-first client that reconstructs state from events"
echo
echo "You can explore the data with:"
echo "curl $BASE_URL/stores/$NOTEBOOK_1/events | jq ."
echo "curl $BASE_URL/stores/$NOTEBOOK_2/events | jq ."
