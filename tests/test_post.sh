#!/bin/bash
# test_post.sh - Script to test POST functionality of msaada

# Set default port (can be overridden)
PORT=${1:-3000}
HOST="localhost:$PORT"
BASE_URL="http://$HOST"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Helper function to print headers
print_header() {
  echo -e "\n${BLUE}==== $1 ====${NC}"
}

# Helper function to print success or failure
print_result() {
  if [ $1 -eq 0 ]; then
    echo -e "${GREEN}✓ SUCCESS:${NC} $2"
  else
    echo -e "${RED}✗ FAILED:${NC} $2"
    FAILED=1
  fi
}

# Initialize failure counter
FAILED=0

# Check if the server is running
print_header "Checking if msaada is running on port $PORT"
if curl -s "$BASE_URL" > /dev/null; then
  print_result 0 "Server is running on $BASE_URL"
else
  print_result 1 "Server is not running on $BASE_URL"
  echo "Please start msaada first with:"
  echo "  cargo run -- --port $PORT --dir ."
  exit 1
fi

# Test 1: Send JSON data
print_header "Testing JSON POST request"
JSON_RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d '{"name":"test","value":42}' \
  "$BASE_URL/api/test")

echo "$JSON_RESPONSE" | grep -q "json_data"
print_result $? "JSON data recognized correctly"

# Test 2: Send form data
print_header "Testing form-encoded POST request"
FORM_RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "name=test&value=42" \
  "$BASE_URL/api/form")

echo "$FORM_RESPONSE" | grep -q "form_data"
print_result $? "Form data recognized correctly"

# Test 3: Send multipart form with a file
print_header "Testing multipart form with file"
FILE_RESPONSE=$(curl -s -X POST \
  -F "field1=value1" \
  -F "file=@test_post.sh" \
  "$BASE_URL/api/upload")

echo "$FILE_RESPONSE" | grep -q "files"
print_result $? "File upload recognized correctly"

echo "$FILE_RESPONSE" | grep -q "test_post.sh"
print_result $? "Correct filename detected"

echo "$FILE_RESPONSE" | grep -q "form_data"
print_result $? "Form fields included with file upload"

# Test 4: Send plain text
print_header "Testing plain text POST"
TEXT_RESPONSE=$(curl -s -X POST \
  -H "Content-Type: text/plain" \
  -d "This is a plain text message" \
  "$BASE_URL/api/text")

echo "$TEXT_RESPONSE" | grep -q "text_data"
print_result $? "Plain text recognized correctly"

# Summary
print_header "Test Summary"
if [ $FAILED -eq 0 ]; then
  echo -e "${GREEN}All tests passed!${NC}"
  echo "The POST handler is working correctly."
else
  echo -e "${RED}$FAILED tests failed.${NC}"
  echo -e "${YELLOW}Please check if:"
  echo "1. The POST handler is properly registered"
  echo "2. The handler is registered before the Files service"
  echo "3. The content types are being correctly matched${NC}"
fi

exit $FAILED