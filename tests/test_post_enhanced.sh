#!/bin/bash
# test_post_enhanced.sh - Enhanced POST request echo functionality tests

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="POST Request Echo (Enhanced)"
TEST_PORT=3300

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Setup test environment
setup_post_test_env() {
    print_subheader "Setting up POST test environment"
    
    local test_dir="$TEMP_DIR/post_test_files"
    mkdir -p "$test_dir"
    
    # Create basic index page
    create_test_html "$test_dir/index.html" "POST Test Server" "<h1>POST Testing Server</h1><p>Use this server to test POST requests.</p>"
    
    # Create test files for upload testing
    echo "This is a sample text file for upload testing." > "$test_dir/sample.txt"
    echo '{"test": "json", "upload": true}' > "$test_dir/sample.json"
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xdb\x00\x00\x00\x00IEND\xaeB`\x82' > "$test_dir/sample.png"
    
    # Create a larger file for testing
    dd if=/dev/zero of="$test_dir/large_file.bin" bs=1024 count=100 >/dev/null 2>&1
    
    print_success "POST test environment created"
    echo "$test_dir"
}

# Test JSON POST requests
test_json_post() {
    print_test_start "JSON POST Requests"
    
    local tests_passed=0
    
    # Test simple JSON
    print_info "Testing simple JSON POST"
    local temp_file
    temp_file=$(http_post "$SERVER_URL/api/simple" "application/json" '{"name":"test","value":42}')
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "json_data" && validate_json_field "$temp_file" "path"; then
                local path_value
                path_value=$(jq -r '.path' "$temp_file" 2>/dev/null)
                if [[ "$path_value" == "/api/simple" ]]; then
                    print_success "Simple JSON POST works correctly"
                    tests_passed=$((tests_passed + 1))
                else
                    print_failure "JSON POST path incorrect (got: $path_value)"
                fi
            else
                print_failure "JSON POST response missing required fields"
            fi
        else
            print_failure "JSON POST response is not valid JSON"
        fi
    else
        print_failure "JSON POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test nested JSON
    print_info "Testing nested JSON POST"
    temp_file=$(http_post "$SERVER_URL/api/nested" "application/json" '{"user":{"name":"John","age":30},"data":{"items":[1,2,3],"active":true}}')
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            local nested_data
            nested_data=$(jq -r '.json_data.user.name' "$temp_file" 2>/dev/null)
            if [[ "$nested_data" == "John" ]]; then
                print_success "Nested JSON POST works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Nested JSON POST data incorrect"
            fi
        else
            print_failure "Nested JSON POST response invalid"
        fi
    else
        print_failure "Nested JSON POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test empty JSON
    print_info "Testing empty JSON POST"
    temp_file=$(http_post "$SERVER_URL/api/empty" "application/json" '{}')
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            print_success "Empty JSON POST works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Empty JSON POST response invalid"
        fi
    else
        print_failure "Empty JSON POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test invalid JSON (should still be handled)
    print_info "Testing invalid JSON POST"
    temp_file=$(http_post "$SERVER_URL/api/invalid" "application/json" '{"invalid": json}')
    
    if [[ $? -eq 0 ]]; then
        # Should still return a response, possibly with text_data field
        if validate_json "$temp_file"; then
            print_success "Invalid JSON POST handled gracefully"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Invalid JSON POST not handled properly"
        fi
    else
        print_failure "Invalid JSON POST request failed"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 4 ]]; then
        print_success "All JSON POST tests passed"
        return 0
    else
        print_failure "Some JSON POST tests failed ($tests_passed/4 passed)"
        return 1
    fi
}

# Test form data POST requests
test_form_post() {
    print_test_start "Form Data POST Requests"
    
    local tests_passed=0
    
    # Test simple form data
    print_info "Testing simple form data POST"
    local temp_file
    temp_file=$(http_post "$SERVER_URL/api/form" "application/x-www-form-urlencoded" "name=test&value=42&active=true")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "form_data"; then
                print_success "Simple form data POST works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Form data POST response missing form_data field"
            fi
        else
            print_failure "Form data POST response is not valid JSON"
        fi
    else
        print_failure "Form data POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test form data with special characters
    print_info "Testing form data with special characters"
    temp_file=$(http_post "$SERVER_URL/api/special" "application/x-www-form-urlencoded" "message=Hello%20World%21&symbols=%40%23%24%25")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            print_success "Form data with special characters works"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Form data with special characters failed"
        fi
    else
        print_failure "Form data with special characters request failed"
    fi
    rm -f "$temp_file"
    
    # Test empty form data
    print_info "Testing empty form data"
    temp_file=$(http_post "$SERVER_URL/api/empty-form" "application/x-www-form-urlencoded" "")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            print_success "Empty form data works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Empty form data response invalid"
        fi
    else
        print_failure "Empty form data request failed"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All form data POST tests passed"
        return 0
    else
        print_failure "Some form data POST tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test multipart file upload
test_multipart_upload() {
    print_test_start "Multipart File Upload"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test single file upload
    print_info "Testing single file upload"
    local temp_file
    temp_file=$(http_upload "$SERVER_URL/api/upload" "$test_dir/sample.txt" "document")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "files" && grep -q "sample.txt" "$temp_file"; then
                print_success "Single file upload works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Single file upload response incorrect"
            fi
        else
            print_failure "Single file upload response not JSON"
        fi
    else
        print_failure "Single file upload request failed"
    fi
    rm -f "$temp_file"
    
    # Test image file upload
    print_info "Testing image file upload"
    temp_file=$(http_upload "$SERVER_URL/api/upload-image" "$test_dir/sample.png" "image")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if grep -q "sample.png" "$temp_file"; then
                print_success "Image file upload works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Image file upload filename not found"
            fi
        else
            print_failure "Image file upload response not JSON"
        fi
    else
        print_failure "Image file upload request failed"
    fi
    rm -f "$temp_file"
    
    # Test multipart with form fields
    print_info "Testing multipart with form fields and file"
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -F "name=test_upload" \
        -F "description=A test file upload" \
        -F "file=@$test_dir/sample.json" \
        "$SERVER_URL/api/upload-with-fields" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "form_data" && validate_json_field "$temp_file" "files"; then
                print_success "Multipart with form fields works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Multipart with form fields missing data"
            fi
        else
            print_failure "Multipart with form fields response not JSON"
        fi
    else
        print_failure "Multipart with form fields request failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test large file upload
    print_info "Testing large file upload"
    temp_file=$(http_upload "$SERVER_URL/api/upload-large" "$test_dir/large_file.bin" "largefile")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if grep -q "large_file.bin" "$temp_file"; then
                print_success "Large file upload works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Large file upload filename not found"
            fi
        else
            print_failure "Large file upload response not JSON"
        fi
    else
        print_failure "Large file upload request failed"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 4 ]]; then
        print_success "All multipart upload tests passed"
        return 0
    else
        print_failure "Some multipart upload tests failed ($tests_passed/4 passed)"
        return 1
    fi
}

# Test plain text POST requests
test_text_post() {
    print_test_start "Plain Text POST Requests"
    
    local tests_passed=0
    
    # Test simple text
    print_info "Testing simple text POST"
    local temp_file
    temp_file=$(http_post "$SERVER_URL/api/text" "text/plain" "This is a simple text message.")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "text_data"; then
                local text_content
                text_content=$(jq -r '.text_data' "$temp_file" 2>/dev/null)
                if [[ "$text_content" == "This is a simple text message." ]]; then
                    print_success "Simple text POST works correctly"
                    tests_passed=$((tests_passed + 1))
                else
                    print_failure "Text POST content incorrect"
                fi
            else
                print_failure "Text POST response missing text_data field"
            fi
        else
            print_failure "Text POST response not JSON"
        fi
    else
        print_failure "Text POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test multiline text
    print_info "Testing multiline text POST"
    local multiline_text="Line 1\nLine 2\nLine 3 with special chars: @#$%"
    temp_file=$(http_post "$SERVER_URL/api/multiline" "text/plain" "$multiline_text")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            if validate_json_field "$temp_file" "text_data"; then
                print_success "Multiline text POST works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Multiline text POST response missing text_data"
            fi
        else
            print_failure "Multiline text POST response not JSON"
        fi
    else
        print_failure "Multiline text POST request failed"
    fi
    rm -f "$temp_file"
    
    # Test empty text
    print_info "Testing empty text POST"
    temp_file=$(http_post "$SERVER_URL/api/empty-text" "text/plain" "")
    
    if [[ $? -eq 0 ]]; then
        if validate_json "$temp_file"; then
            print_success "Empty text POST works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Empty text POST response not JSON"
        fi
    else
        print_failure "Empty text POST request failed"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All text POST tests passed"
        return 0
    else
        print_failure "Some text POST tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test binary data POST requests  
test_binary_post() {
    print_test_start "Binary Data POST Requests"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test binary data upload
    print_info "Testing binary data POST"
    local temp_file
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/octet-stream" \
        --data-binary "@$test_dir/sample.png" \
        "$SERVER_URL/api/binary" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if validate_json "$temp_file"; then
            # Should have content_type field
            local content_type
            content_type=$(jq -r '.content_type' "$temp_file" 2>/dev/null)
            if [[ "$content_type" =~ "application/octet-stream" ]]; then
                print_success "Binary data POST works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Binary data POST content type incorrect"
            fi
        else
            print_failure "Binary data POST response not JSON"
        fi
    else
        print_failure "Binary data POST request failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test custom binary content type
    print_info "Testing custom binary content type"
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/pdf" \
        --data-binary "@$test_dir/large_file.bin" \
        "$SERVER_URL/api/pdf" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if validate_json "$temp_file"; then
            local content_type
            content_type=$(jq -r '.content_type' "$temp_file" 2>/dev/null)
            if [[ "$content_type" =~ "application/pdf" ]]; then
                print_success "Custom binary content type works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Custom binary content type incorrect"
            fi
        else
            print_failure "Custom binary POST response not JSON"
        fi
    else
        print_failure "Custom binary POST request failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All binary POST tests passed"
        return 0
    else
        print_failure "Some binary POST tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test POST response format consistency
test_response_format() {
    print_test_start "POST Response Format Consistency"
    
    local tests_passed=0
    
    # Test that all responses have required fields
    local endpoints=(
        "/api/test-json:application/json:{\"test\":true}"
        "/api/test-form:application/x-www-form-urlencoded:name=test"
        "/api/test-text:text/plain:Test text"
    )
    
    for endpoint_data in "${endpoints[@]}"; do
        IFS=':' read -r endpoint content_type data <<< "$endpoint_data"
        
        local temp_file
        temp_file=$(http_post "$SERVER_URL$endpoint" "$content_type" "$data")
        
        if [[ $? -eq 0 ]]; then
            if validate_json "$temp_file"; then
                # Check for required fields
                if validate_json_field "$temp_file" "path" && validate_json_field "$temp_file" "content_type"; then
                    local returned_path
                    returned_path=$(jq -r '.path' "$temp_file" 2>/dev/null)
                    if [[ "$returned_path" == "$endpoint" ]]; then
                        tests_passed=$((tests_passed + 1))
                    else
                        print_failure "Path field incorrect for $endpoint (got: $returned_path)"
                    fi
                else
                    print_failure "Required fields missing for $endpoint"
                fi
            else
                print_failure "Response not JSON for $endpoint"
            fi
        else
            print_failure "Request failed for $endpoint"
        fi
        rm -f "$temp_file"
    done
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All response format consistency tests passed"
        return 0
    else
        print_failure "Some response format tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Main test execution
main() {
    print_header "Starting $TEST_SUITE_NAME Tests"
    
    if ! ensure_msaada_binary; then
        print_error "Failed to ensure msaada binary"
        exit 1
    fi
    
    # Setup test environment
    local test_dir
    test_dir=$(setup_post_test_env)
    
    # Start server
    if ! start_server "$TEST_PORT" "$test_dir"; then
        print_error "Failed to start server, aborting tests"
        exit 1
    fi
    
    # Run test suites
    test_json_post
    test_form_post  
    test_multipart_upload "$test_dir"
    test_text_post
    test_binary_post "$test_dir"
    test_response_format
    
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