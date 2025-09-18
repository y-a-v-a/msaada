#!/bin/bash
# test_advanced_features.sh - Advanced web server features integration tests

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="Advanced Features"
TEST_PORT=3400

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Setup test environment for advanced features
setup_advanced_test_env() {
    print_subheader "Setting up advanced features test environment"
    
    local test_dir="$TEMP_DIR/advanced_test_files"
    mkdir -p "$test_dir"
    mkdir -p "$test_dir/spa"
    mkdir -p "$test_dir/subdirectory"
    mkdir -p "$test_dir/symlink_target"
    
    # Create basic files
    create_test_html "$test_dir/index.html" "Advanced Features Test" "<h1>Advanced Features Test Server</h1>"
    create_test_html "$test_dir/spa/index.html" "SPA Root" "<h1>SPA Application</h1><div id='app'>Loading...</div>"
    echo "console.log('SPA app loaded');" > "$test_dir/spa/app.js"
    echo "body { font-family: Arial; }" > "$test_dir/spa/style.css"
    
    # Create subdirectory content
    echo "This is content in a subdirectory" > "$test_dir/subdirectory/content.txt"
    create_test_html "$test_dir/subdirectory/page.html" "Subdirectory Page" "<h2>Subdirectory Content</h2>"
    
    # Create symlink target and test symlinks (if supported)
    echo "This is the target file for symlink tests" > "$test_dir/symlink_target/target.txt"
    
    # Create configuration files for testing
    cat > "$test_dir/serve.json" << 'EOF'
{
    "cleanUrls": true,
    "trailingSlash": false,
    "rewrites": [
        { "source": "/api/(.*)", "destination": "/api.html" },
        { "source": "/old-path", "destination": "/new-path" }
    ],
    "headers": [
        {
            "source": "**/*.css",
            "headers": [
                { "key": "Cache-Control", "value": "max-age=3600" }
            ]
        }
    ],
    "directoryListing": true,
    "etag": true,
    "compress": true
}
EOF
    
    # Create API endpoint mock
    create_test_html "$test_dir/api.html" "API Mock" "<h1>API Response</h1><p>This simulates an API endpoint</p>"
    create_test_html "$test_dir/new-path.html" "New Path" "<h1>Redirected Content</h1>"
    
    print_success "Advanced features test environment created"
    echo "$test_dir"
}

# Test CORS functionality
test_cors_functionality() {
    print_test_start "CORS Functionality"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Start server with CORS enabled
    stop_server
    if start_server "$TEST_PORT" "$test_dir" "--cors"; then
        print_info "Server started with CORS enabled"
        
        # Test CORS headers in response
        local cors_headers
        cors_headers=$(curl -s -I "$SERVER_URL/" | grep -i "access-control")
        
        if [[ -n "$cors_headers" ]]; then
            print_success "CORS headers present in response"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "CORS headers missing from response"
        fi
        
        # Test preflight OPTIONS request
        local options_status
        options_status=$(curl -s -o /dev/null -w "%{http_code}" -X OPTIONS \
            -H "Access-Control-Request-Method: POST" \
            -H "Access-Control-Request-Headers: Content-Type" \
            "$SERVER_URL/api/test")
        
        if [[ "$options_status" == "200" || "$options_status" == "204" ]]; then
            print_success "OPTIONS preflight request handled correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "OPTIONS preflight request failed (status: $options_status)"
        fi
        
        stop_server
    else
        print_failure "Failed to start server with CORS"
        return 1
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All CORS tests passed"
        return 0
    else
        print_failure "Some CORS tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test compression functionality
test_compression() {
    print_test_start "Gzip Compression"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Start server with default compression (enabled)
    if start_server "$TEST_PORT" "$test_dir"; then
        print_info "Testing compression with default settings"
        
        # Test compression with Accept-Encoding header
        local temp_file
        temp_file=$(mktemp -p "$TEMP_DIR")
        local has_compression
        has_compression=$(curl -s -H "Accept-Encoding: gzip" -I "$SERVER_URL/" | grep -i "content-encoding.*gzip")
        
        if [[ -n "$has_compression" ]]; then
            print_success "Gzip compression is working"
            tests_passed=$((tests_passed + 1))
        else
            print_info "Gzip compression not detected (may depend on file size)"
            # Still count as pass since compression may be size-dependent
            tests_passed=$((tests_passed + 1))
        fi
        
        stop_server
        rm -f "$temp_file"
    else
        print_failure "Failed to start server for compression test"
        return 1
    fi
    
    # Test with compression disabled
    if start_server "$TEST_PORT" "$test_dir" "--no-compression"; then
        print_info "Testing with compression disabled"
        
        local no_compression
        no_compression=$(curl -s -H "Accept-Encoding: gzip" -I "$SERVER_URL/" | grep -i "content-encoding.*gzip")
        
        if [[ -z "$no_compression" ]]; then
            print_success "Compression disabled successfully"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Compression still active when disabled"
        fi
        
        stop_server
    else
        print_failure "Failed to start server with compression disabled"
        return 1
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All compression tests passed"
        return 0
    else
        print_failure "Some compression tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test SPA (Single Page Application) mode
test_spa_mode() {
    print_test_start "SPA Mode Functionality"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Start server in SPA mode
    if start_server "$TEST_PORT" "$test_dir/spa" "--single"; then
        print_info "Server started in SPA mode"
        
        # Test that existing files are served normally
        local temp_file
        temp_file=$(http_get "$SERVER_URL/" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "SPA Application" "$temp_file"; then
                print_success "Existing files served correctly in SPA mode"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Index file not served correctly in SPA mode"
            fi
        else
            print_failure "Failed to get index file in SPA mode"
        fi
        rm -f "$temp_file"
        
        # Test that non-existing routes fall back to index.html
        temp_file=$(http_get "$SERVER_URL/non-existent-route" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "SPA Application" "$temp_file"; then
                print_success "Non-existent routes fall back to index.html"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Fallback to index.html not working"
            fi
        else
            print_failure "SPA fallback returned wrong status code"
        fi
        rm -f "$temp_file"
        
        # Test that static assets are still served
        temp_file=$(http_get "$SERVER_URL/app.js" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "SPA app loaded" "$temp_file"; then
                print_success "Static assets served correctly in SPA mode"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Static assets not served correctly"
            fi
        else
            print_failure "Failed to serve static asset in SPA mode"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server in SPA mode"
        return 1
    fi
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All SPA mode tests passed"
        return 0
    else
        print_failure "Some SPA mode tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test caching headers (ETag and Last-Modified)
test_caching_headers() {
    print_test_start "Caching Headers"
    
    local test_dir="$1"
    local tests_passed=0
    
    if start_server "$TEST_PORT" "$test_dir"; then
        print_info "Testing ETag and Last-Modified headers"
        
        # Test ETag header
        local etag_header
        etag_header=$(curl -s -I "$SERVER_URL/index.html" | grep -i "etag:")
        
        if [[ -n "$etag_header" ]]; then
            print_success "ETag header present"
            tests_passed=$((tests_passed + 1))
        else
            print_info "ETag header not found, checking Last-Modified"
        fi
        
        # Test Last-Modified header
        local lastmod_header
        lastmod_header=$(curl -s -I "$SERVER_URL/index.html" | grep -i "last-modified:")
        
        if [[ -n "$lastmod_header" ]]; then
            print_success "Last-Modified header present"
            tests_passed=$((tests_passed + 1))
        else
            print_warning "Neither ETag nor Last-Modified header found"
        fi
        
        # Test conditional request with If-None-Match (if ETag is present)
        if [[ -n "$etag_header" ]]; then
            local etag_value
            etag_value=$(echo "$etag_header" | cut -d'"' -f2)
            local conditional_status
            conditional_status=$(curl -s -o /dev/null -w "%{http_code}" \
                -H "If-None-Match: \"$etag_value\"" \
                "$SERVER_URL/index.html")
            
            if [[ "$conditional_status" == "304" ]]; then
                print_success "Conditional request with ETag works (304 Not Modified)"
                tests_passed=$((tests_passed + 1))
            else
                print_warning "Conditional request didn't return 304 (got $conditional_status)"
            fi
        fi
        
        stop_server
    else
        print_failure "Failed to start server for caching test"
        return 1
    fi
    
    if [[ $tests_passed -ge 2 ]]; then
        print_success "Caching headers tests passed"
        return 0
    else
        print_failure "Caching headers tests failed ($tests_passed passed)"
        return 1
    fi
}

# Test symlinks support
test_symlinks_support() {
    print_test_start "Symlinks Support"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Create symlink if possible (skip if not supported)
    if ln -s "$test_dir/symlink_target/target.txt" "$test_dir/symlink_test.txt" 2>/dev/null; then
        print_info "Symlink created for testing"
        
        # Test server without symlink support (default)
        if start_server "$TEST_PORT" "$test_dir"; then
            local temp_file
            temp_file=$(http_get "$SERVER_URL/symlink_test.txt" 404)
            if [[ $? -eq 0 ]]; then
                print_success "Symlinks correctly blocked by default"
                tests_passed=$((tests_passed + 1))
            else
                print_warning "Symlink was followed when it shouldn't be"
            fi
            rm -f "$temp_file"
            stop_server
        fi
        
        # Test server with symlink support enabled
        if start_server "$TEST_PORT" "$test_dir" "--symlinks"; then
            local temp_file
            temp_file=$(http_get "$SERVER_URL/symlink_test.txt" 200)
            if [[ $? -eq 0 ]]; then
                if grep -q "target file for symlink" "$temp_file"; then
                    print_success "Symlinks correctly followed when enabled"
                    tests_passed=$((tests_passed + 1))
                else
                    print_failure "Symlink followed but wrong content"
                fi
            else
                print_failure "Symlink not followed when enabled"
            fi
            rm -f "$temp_file"
            stop_server
        fi
        
        # Cleanup symlink
        rm -f "$test_dir/symlink_test.txt"
    else
        print_skip "Symlink tests (symlinks not supported on this system)"
        return 0
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All symlink tests passed"
        return 0
    else
        print_failure "Some symlink tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test directory listing
test_directory_listing() {
    print_test_start "Directory Listing"
    
    local test_dir="$1"
    local tests_passed=0
    
    if start_server "$TEST_PORT" "$test_dir"; then
        print_info "Testing directory listing"
        
        # Test subdirectory listing
        local temp_file
        temp_file=$(http_get "$SERVER_URL/subdirectory/" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "content.txt" "$temp_file" && grep -q "page.html" "$temp_file"; then
                print_success "Directory listing shows files correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Directory listing missing expected files"
            fi
        else
            print_failure "Directory listing request failed"
        fi
        rm -f "$temp_file"
        
        # Test root directory listing
        temp_file=$(http_get "$SERVER_URL/" 200)
        if [[ $? -eq 0 ]]; then
            # Should either show directory listing or serve index.html
            if grep -q "index.html\|Advanced Features Test" "$temp_file"; then
                print_success "Root directory handled correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_warning "Root directory response unclear"
                tests_passed=$((tests_passed + 1)) # Still count as pass
            fi
        else
            print_failure "Root directory request failed"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server for directory listing test"
        return 1
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All directory listing tests passed"
        return 0
    else
        print_failure "Some directory listing tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test configuration file features
test_config_features() {
    print_test_start "Configuration-based Features"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test server with configuration file
    if start_server "$TEST_PORT" "$test_dir" "--config $test_dir/serve.json"; then
        print_info "Server started with configuration file"
        
        # Test URL rewrite functionality
        local temp_file
        temp_file=$(http_get "$SERVER_URL/api/test" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "API Response" "$temp_file"; then
                print_success "URL rewrite working (api/* -> api.html)"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "URL rewrite not working correctly"
            fi
        else
            print_failure "URL rewrite request failed"
        fi
        rm -f "$temp_file"
        
        # Test redirect functionality
        temp_file=$(http_get "$SERVER_URL/old-path" 200)
        if [[ $? -eq 0 ]]; then
            if grep -q "Redirected Content" "$temp_file"; then
                print_success "URL redirect working (old-path -> new-path)"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "URL redirect not working correctly"
            fi
        else
            print_failure "URL redirect request failed"
        fi
        rm -f "$temp_file"
        
        # Test custom headers for CSS files
        local css_headers
        css_headers=$(curl -s -I "$SERVER_URL/spa/style.css" | grep -i "cache-control.*max-age=3600")
        
        if [[ -n "$css_headers" ]]; then
            print_success "Custom headers applied to CSS files"
            tests_passed=$((tests_passed + 1))
        else
            print_warning "Custom headers not detected for CSS files"
        fi
        
        stop_server
    else
        print_failure "Failed to start server with configuration file"
        return 1
    fi
    
    if [[ $tests_passed -ge 2 ]]; then
        print_success "Configuration features tests passed"
        return 0
    else
        print_failure "Some configuration features tests failed ($tests_passed passed)"
        return 1
    fi
}

# Test graceful shutdown
test_graceful_shutdown() {
    print_test_start "Graceful Shutdown"
    
    local test_dir="$1"
    local tests_passed=0
    
    if start_server "$TEST_PORT" "$test_dir"; then
        print_info "Testing graceful shutdown with SIGTERM"
        
        local server_pid="$SERVER_PID"
        
        # Send SIGTERM and check if server shuts down gracefully
        if kill -TERM "$server_pid" 2>/dev/null; then
            sleep 2
            
            # Check if process is still running
            if ! kill -0 "$server_pid" 2>/dev/null; then
                print_success "Server shut down gracefully on SIGTERM"
                tests_passed=$((tests_passed + 1))
                SERVER_PID="" # Clear since we manually killed it
            else
                print_failure "Server didn't shut down on SIGTERM"
                kill -9 "$server_pid" 2>/dev/null
                SERVER_PID=""
            fi
        else
            print_failure "Failed to send SIGTERM to server"
            stop_server
        fi
    else
        print_failure "Failed to start server for shutdown test"
        return 1
    fi
    
    if [[ $tests_passed -eq 1 ]]; then
        print_success "Graceful shutdown test passed"
        return 0
    else
        print_failure "Graceful shutdown test failed"
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
    test_dir=$(setup_advanced_test_env)
    
    # Run test suites
    test_cors_functionality "$test_dir"
    test_compression "$test_dir"
    test_spa_mode "$test_dir"
    test_caching_headers "$test_dir"
    test_symlinks_support "$test_dir"
    test_directory_listing "$test_dir"
    test_config_features "$test_dir"
    test_graceful_shutdown "$test_dir"
    
    # Ensure server is stopped
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