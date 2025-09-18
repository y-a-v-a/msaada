#!/bin/bash
# test_http_server.sh - Core HTTP server functionality tests

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="Core HTTP Server"
TEST_PORT=3100

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Setup test environment
setup_test_env() {
    print_subheader "Setting up test environment"
    
    # Create test files with various content types
    local test_dir="$TEMP_DIR/http_test_files"
    mkdir -p "$test_dir"
    
    # HTML file
    create_test_html "$test_dir/index.html" "Test Index" "<h1>Welcome</h1><p>This is a test page.</p>"
    
    # CSS file
    echo "body { background-color: #f0f0f0; font-family: Arial; }" > "$test_dir/style.css"
    
    # JavaScript file
    echo "console.log('Test JavaScript file loaded');" > "$test_dir/script.js"
    
    # JSON file
    echo '{"name": "test", "value": 42, "active": true}' > "$test_dir/data.json"
    
    # Plain text file
    echo "This is a plain text file for testing." > "$test_dir/readme.txt"
    
    # XML file
    echo '<?xml version="1.0" encoding="UTF-8"?><root><item>test</item></root>' > "$test_dir/data.xml"
    
    # Image placeholder (create a small binary file)
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xdb\x00\x00\x00\x00IEND\xaeB`\x82' > "$test_dir/test.png"
    
    # Large file for testing (1MB)
    dd if=/dev/zero of="$test_dir/large.txt" bs=1024 count=1024 >/dev/null 2>&1
    
    # Empty file
    touch "$test_dir/empty.txt"
    
    # Hidden file
    echo "Hidden content" > "$test_dir/.hidden"
    
    # File with special characters in name
    echo "Special content" > "$test_dir/file with spaces.txt"
    echo "Unicode content" > "$test_dir/café.txt"
    
    print_success "Test environment created at $test_dir"
    echo "$test_dir"
}

# Test basic file serving
test_basic_file_serving() {
    print_test_start "Basic File Serving"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test HTML file
    local response
    response=$(http_get "$SERVER_URL/index.html")
    if [[ $? -eq 0 ]]; then
        if grep -q "Welcome" "$response"; then
            print_success "HTML file served correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "HTML content not correct"
        fi
    else
        print_failure "Failed to serve HTML file"
    fi
    rm -f "$response" 2>/dev/null
    
    # Test CSS file
    response=$(http_get "$SERVER_URL/style.css")
    if [[ $? -eq 0 ]]; then
        if grep -q "background-color" "$response"; then
            print_success "CSS file served correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "CSS content not correct"
        fi
    else
        print_failure "Failed to serve CSS file"
    fi
    rm -f "$response" 2>/dev/null
    
    # Test JavaScript file
    response=$(http_get "$SERVER_URL/script.js")
    if [[ $? -eq 0 ]]; then
        if grep -q "console.log" "$response"; then
            print_success "JavaScript file served correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "JavaScript content not correct"
        fi
    else
        print_failure "Failed to serve JavaScript file"
    fi
    rm -f "$response" 2>/dev/null
    
    # Test JSON file
    response=$(http_get "$SERVER_URL/data.json")
    if [[ $? -eq 0 ]]; then
        if validate_json "$response" && grep -q '"name".*"test"' "$response"; then
            print_success "JSON file served correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "JSON content not valid or correct"
        fi
    else
        print_failure "Failed to serve JSON file"
    fi
    rm -f "$response" 2>/dev/null
    
    if [[ $tests_passed -eq 4 ]]; then
        print_success "All basic file serving tests passed"
    else
        print_failure "Some basic file serving tests failed ($tests_passed/4 passed)"
    fi
}

# Test HTTP content types
test_content_types() {
    print_test_start "HTTP Content Types"
    
    local tests_passed=0
    
    # Test HTML content type
    if validate_content_type "$SERVER_URL/index.html" "text/html"; then
        print_success "HTML content type correct"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "HTML content type incorrect"
    fi
    
    # Test CSS content type
    if validate_content_type "$SERVER_URL/style.css" "text/css"; then
        print_success "CSS content type correct"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "CSS content type incorrect"
    fi
    
    # Test JavaScript content type
    if validate_content_type "$SERVER_URL/script.js" "text/javascript"; then
        print_success "JavaScript content type correct"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "JavaScript content type incorrect"
    fi
    
    # Test JSON content type
    if validate_content_type "$SERVER_URL/data.json" "application/json"; then
        print_success "JSON content type correct"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "JSON content type incorrect"
    fi
    
    # Test PNG content type
    if validate_content_type "$SERVER_URL/test.png" "image/png"; then
        print_success "PNG content type correct"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "PNG content type incorrect"
    fi
    
    if [[ $tests_passed -eq 5 ]]; then
        print_success "All content type tests passed"
    else
        print_failure "Some content type tests failed ($tests_passed/5 passed)"
    fi
}

# Test HTTP response headers
test_response_headers() {
    print_test_start "HTTP Response Headers"
    
    local tests_passed=0
    
    # Test custom headers (X-Server, X-Powered-By, X-Version from main.rs)
    local headers
    headers=$(curl -s -I "$SERVER_URL/" 2>/dev/null)
    
    if echo "$headers" | grep -qi "X-Server:"; then
        print_success "X-Server header present"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "X-Server header missing"
    fi
    
    if echo "$headers" | grep -qi "X-Powered-By:"; then
        print_success "X-Powered-By header present"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "X-Powered-By header missing"
    fi
    
    if echo "$headers" | grep -qi "X-Version:"; then
        print_success "X-Version header present"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "X-Version header missing"
    fi
    
    # Test Content-Length header
    if echo "$headers" | grep -qi "Content-Length:"; then
        print_success "Content-Length header present"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Content-Length header missing"
    fi
    
    # Test Date header
    if echo "$headers" | grep -qi "Date:"; then
        print_success "Date header present"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Date header missing"
    fi
    
    if [[ $tests_passed -eq 5 ]]; then
        print_success "All response header tests passed"
    else
        print_failure "Some response header tests failed ($tests_passed/5 passed)"
    fi
}

# Test HTTP methods
test_http_methods() {
    print_test_start "HTTP Methods"
    
    local tests_passed=0
    local temp_file
    
    # Test GET method
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/index.html" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        print_success "GET method works correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "GET method failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test HEAD method
    headers_file=$(mktemp -p "$TEMP_DIR")
    body_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -X HEAD -D "$headers_file" -o "$body_file" -w "%{http_code}" "$SERVER_URL/index.html" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        # HEAD should return headers but no body
        headers_size=$(wc -c < "$headers_file" 2>/dev/null || echo "0")
        body_size=$(wc -c < "$body_file" 2>/dev/null || echo "0")

        if [[ "$headers_size" -gt 0 && "$body_size" -eq 0 ]]; then
            print_success "HEAD method works correctly (headers: ${headers_size} bytes, body: ${body_size} bytes)"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "HEAD method incorrect (headers: ${headers_size} bytes, body: ${body_size} bytes)"
        fi
    else
        print_failure "HEAD method failed (status: $status_code)"
    fi
    rm -f "$headers_file" "$body_file"
    
    # Test OPTIONS method (should be handled)
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" -X OPTIONS "$SERVER_URL/" 2>/dev/null)
    # OPTIONS might return 200, 404, or 405 depending on implementation
    if [[ "$status_code" =~ ^(200|404|405)$ ]]; then
        print_success "OPTIONS method handled appropriately (status: $status_code)"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "OPTIONS method returned unexpected status: $status_code"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All HTTP method tests passed"
    else
        print_failure "Some HTTP method tests failed ($tests_passed/3 passed)"
    fi
}

# Test error handling
test_error_handling() {
    print_test_start "Error Handling"
    
    local tests_passed=0
    local temp_file
    local status_code
    
    # Test 404 Not Found
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/nonexistent.html" 2>/dev/null)
    if [[ "$status_code" == "404" ]]; then
        print_success "404 Not Found handled correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "404 Not Found not handled correctly (got: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test directory traversal protection
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/../../../etc/passwd" 2>/dev/null)
    if [[ "$status_code" =~ ^(404|403)$ ]]; then
        print_success "Directory traversal protection works (status: $status_code)"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Directory traversal protection failed (got: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test invalid URLs
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/%00%00invalid" 2>/dev/null)
    if [[ "$status_code" =~ ^(400|404)$ ]]; then
        print_success "Invalid URL handling works (status: $status_code)"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Invalid URL handling failed (got: $status_code)"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All error handling tests passed"
    else
        print_failure "Some error handling tests failed ($tests_passed/3 passed)"
    fi
}

# Test special files
test_special_files() {
    print_test_start "Special File Handling"
    
    local tests_passed=0
    local temp_file
    local status_code
    
    # Test empty file
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/empty.txt" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        print_success "Empty file served correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Empty file not served correctly (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test large file
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/large.txt" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        local file_size
        file_size=$(wc -c < "$temp_file" 2>/dev/null)
        if [[ "$file_size" -gt 1000000 ]]; then  # Should be ~1MB
            print_success "Large file served correctly (size: $file_size bytes)"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Large file size incorrect (got: $file_size bytes)"
        fi
    else
        print_failure "Large file not served correctly (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test hidden files (should be 400 or 404 by default - blocked)
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/.hidden" 2>/dev/null)
    if [[ "$status_code" =~ ^(400|404)$ ]]; then
        print_success "Hidden files correctly blocked (status: $status_code)"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Hidden files not blocked (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test files with spaces (URL encoded)
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/file%20with%20spaces.txt" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        if grep -q "Special content" "$temp_file"; then
            print_success "Files with spaces served correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Files with spaces content incorrect"
        fi
    else
        print_failure "Files with spaces not served correctly (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 4 ]]; then
        print_success "All special file tests passed"
    else
        print_failure "Some special file tests failed ($tests_passed/4 passed)"
    fi
}

# Test directory index handling  
test_directory_index() {
    print_test_start "Directory Index Handling"
    
    local tests_passed=0
    local temp_file
    local status_code
    
    # Test root directory access (should serve index.html)
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        if grep -q "Welcome" "$temp_file"; then
            print_success "Root directory serves index.html correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Root directory index.html content incorrect"
        fi
    else
        print_failure "Root directory not accessible (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test direct index.html access
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$SERVER_URL/index.html" 2>/dev/null)
    if [[ "$status_code" == "200" ]]; then
        if grep -q "Welcome" "$temp_file"; then
            print_success "Direct index.html access works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Direct index.html content incorrect"
        fi
    else
        print_failure "Direct index.html access failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All directory index tests passed"
    else
        print_failure "Some directory index tests failed ($tests_passed/2 passed)"
    fi
}

# Main test execution
main() {
    print_header "Starting $TEST_SUITE_NAME Tests"
    
    # Setup test environment
    local test_dir
    test_dir=$(setup_test_env)
    
    # Start server
    if ! start_server "$TEST_PORT" "$test_dir"; then
        print_error "Failed to start server, aborting tests"
        exit 1
    fi
    
    # Run test suites
    test_basic_file_serving "$test_dir"
    test_content_types
    test_response_headers
    test_http_methods
    test_error_handling
    test_special_files
    test_directory_index
    
    # Stop server
    stop_server
    
    # Print results
    print_test_summary
    
    # Return appropriate exit code
    if [[ $FAILED_COUNT -eq 0 ]]; then
        return 0
    else
        return 1
    fi
}

# Run tests if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi