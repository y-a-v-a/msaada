#!/bin/bash
# run_test.sh - Comprehensive test script for msaada's POST functionality

# Configuration
TEST_PORT=3099
TEST_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )/test_server"
MSAADA_BIN="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )/target/release/msaada"
LOG_FILE="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )/test_server/msaada_test.log"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Print header
echo -e "\n${BLUE}=======================================${NC}"
echo -e "${BLUE}   MSAADA POST FUNCTIONALITY TEST   ${NC}"
echo -e "${BLUE}=======================================${NC}\n"

# Check if test directory exists
if [ ! -d "$TEST_DIR" ]; then
    echo -e "${RED}Test directory not found: $TEST_DIR${NC}"
    exit 1
fi

# Check if msaada binary exists or build it
if [ ! -f "$MSAADA_BIN" ]; then
    echo -e "${YELLOW}Msaada binary not found, building...${NC}"
    cd "$(dirname "$MSAADA_BIN")/.." && cargo build --release
    
    if [ ! -f "$MSAADA_BIN" ]; then
        echo -e "${RED}Failed to build msaada binary!${NC}"
        exit 1
    fi
fi

echo -e "${BLUE}Test Configuration:${NC}"
echo -e "  - Test Port: ${TEST_PORT}"
echo -e "  - Test Directory: ${TEST_DIR}"
echo -e "  - Msaada Binary: ${MSAADA_BIN}"
echo -e "  - Log File: ${LOG_FILE}\n"

# Kill any existing msaada processes
pkill -f "msaada --port $TEST_PORT" >/dev/null 2>&1

# Start msaada server in the background
echo -e "${BLUE}Starting msaada server...${NC}"
"$MSAADA_BIN" --port "$TEST_PORT" --dir "$TEST_DIR" > "$LOG_FILE" 2>&1 &
MSAADA_PID=$!

# Give the server some time to start
sleep 1

# Check if server is running
if ! ps -p $MSAADA_PID > /dev/null; then
    echo -e "${RED}Failed to start msaada server!${NC}"
    cat "$LOG_FILE"
    exit 1
fi

echo -e "${GREEN}Msaada server started with PID: $MSAADA_PID${NC}"

# Function to test a POST request
test_post() {
    local name=$1
    local url=$2
    local content_type=$3
    local data=$4
    
    echo -e "\n${BLUE}Testing $name:${NC}"
    echo -e "  URL: $url"
    echo -e "  Content-Type: $content_type"
    echo -e "  Data: $data\n"
    
    # Create a temporary file for the response
    local temp_file=$(mktemp)
    
    # Send the request
    local status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -H "Content-Type: $content_type" \
        -d "$data" \
        "$url")
    
    # Check if the request was successful
    if [ "$status_code" -eq 200 ]; then
        echo -e "${GREEN}✓ Request successful (HTTP $status_code)${NC}"
        
        # Check if the response is valid JSON
        if jq -e . >/dev/null 2>&1 <<<"$(cat "$temp_file")"; then
            echo -e "${GREEN}✓ Response is valid JSON${NC}"
            
            # Output the first 200 characters of the response
            echo -e "${BLUE}Response preview:${NC}"
            jq . "$temp_file" | head -n 10
            
            # Check specific fields based on content type
            if [[ "$content_type" == "application/json" ]]; then
                jq -e '.json_data' "$temp_file" >/dev/null 2>&1
                if [ $? -eq 0 ]; then
                    echo -e "${GREEN}✓ JSON data field found in response${NC}"
                else
                    echo -e "${RED}✗ JSON data field missing from response${NC}"
                fi
            elif [[ "$content_type" == "text/plain" ]]; then
                jq -e '.text_data' "$temp_file" >/dev/null 2>&1
                if [ $? -eq 0 ]; then
                    echo -e "${GREEN}✓ Text data field found in response${NC}"
                else
                    echo -e "${RED}✗ Text data field missing from response${NC}"
                fi
            fi
        else
            echo -e "${RED}✗ Response is not valid JSON${NC}"
            echo -e "${RED}Response:${NC} $(cat "$temp_file")"
        fi
    else
        echo -e "${RED}✗ Request failed with HTTP $status_code${NC}"
        echo -e "${RED}Response:${NC} $(cat "$temp_file")"
    fi
    
    # Clean up the temporary file
    rm -f "$temp_file"
}

# Run the tests
BASE_URL="http://localhost:$TEST_PORT"

# Wait for the server to be fully ready
sleep 2

# Test 1: JSON POST
test_post "JSON POST" "$BASE_URL/api/test-json" "application/json" '{"name":"test","age":30}'

# Test 2: Form POST
test_post "Form POST" "$BASE_URL/api/test-form" "application/x-www-form-urlencoded" "name=test&value=hello"

# Test 3: Plain Text POST
test_post "Plain Text POST" "$BASE_URL/api/test-text" "text/plain" "This is a plain text test"

# Test 4: Check custom headers
echo -e "\n${BLUE}Testing Custom Headers:${NC}"
HEADERS=$(curl -s -I "$BASE_URL" | grep -E "X-(Server|Powered-By|Version):")
if [[ -n "$HEADERS" ]]; then
    echo -e "${GREEN}✓ Custom headers found:${NC}"
    echo "$HEADERS" | sed 's/^/  /'
else
    echo -e "${RED}✗ Custom headers not found${NC}"
fi

# Output final message
echo -e "\n${BLUE}=======================================${NC}"
echo -e "${BLUE}   Testing completed!   ${NC}"
echo -e "${BLUE}=======================================${NC}"
echo -e "\n${YELLOW}To manually test in browser:${NC}"
echo -e "  Open: http://localhost:$TEST_PORT"
echo -e "  The test page includes interactive forms for further testing."
echo -e "\n${YELLOW}To clean up:${NC}"
echo -e "  Press Ctrl+C to stop the server, or run:"
echo -e "  kill $MSAADA_PID"

# Keep the server running for manual testing
echo -e "\n${BLUE}Server is still running. Press Ctrl+C to stop.${NC}"
wait $MSAADA_PID