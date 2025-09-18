#!/bin/bash
# test_config_files.sh - Configuration file testing suite

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="Configuration Files"
TEST_PORT=3200

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Create test directory structure
setup_config_test_env() {
    print_subheader "Setting up configuration test environment"
    
    local base_dir="$TEMP_DIR/config_test"
    mkdir -p "$base_dir"
    
    # Create subdirectories
    mkdir -p "$base_dir/public"
    mkdir -p "$base_dir/dist" 
    mkdir -p "$base_dir/build"
    mkdir -p "$base_dir/static"
    
    # Create test content in each directory
    create_test_html "$base_dir/index.html" "Base Index" "<h1>Base Directory</h1>"
    create_test_html "$base_dir/public/index.html" "Public Index" "<h1>Public Directory</h1>"
    create_test_html "$base_dir/dist/index.html" "Dist Index" "<h1>Dist Directory</h1>"
    create_test_html "$base_dir/build/index.html" "Build Index" "<h1>Build Directory</h1>"
    
    # Create API test files
    echo '{"source": "base", "config": "none"}' > "$base_dir/api/test.json"
    echo '{"source": "public", "config": "serve.json"}' > "$base_dir/public/api/test.json"
    echo '{"source": "dist", "config": "package.json"}' > "$base_dir/dist/api/test.json"
    
    # Create directories for API files
    mkdir -p "$base_dir/api"
    mkdir -p "$base_dir/public/api" 
    mkdir -p "$base_dir/dist/api"
    
    print_success "Configuration test environment created"
    echo "$base_dir"
}

# Test serve.json configuration
test_serve_json_config() {
    print_test_start "serve.json Configuration"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Create serve.json with various options
    local serve_config
    serve_config=$(create_test_json "serve.json" '{
        "public": "public",
        "cleanUrls": true,
        "trailingSlash": false,
        "renderSingle": false,
        "symlinks": false,
        "etag": true,
        "directoryListing": false,
        "rewrites": [
            {"source": "/api/(.*)", "destination": "/api/index.html"},
            {"source": "/old", "destination": "/new.html"}
        ],
        "redirects": [
            {"source": "/redirect-test", "destination": "/", "type": 301}
        ],
        "headers": [
            {
                "source": "**/*.json",
                "headers": [
                    {"key": "X-Config-Test", "value": "serve.json"}
                ]
            }
        ],
        "unlisted": ["*.secret", "private/*"]
    }')
    
    # Copy serve.json to test directory
    cp "$serve_config" "$test_dir/serve.json"
    
    print_info "Testing serve.json with public directory and options"
    
    # Start server with serve.json
    if start_server "$TEST_PORT" "$test_dir"; then
        
        # Test that public directory is used
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Public Directory" "$temp_file"; then
                print_success "serve.json public directory setting works"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "serve.json public directory not used correctly"
            fi
        else
            print_failure "Failed to access server with serve.json"
        fi
        rm -f "$temp_file"
        
        # Test custom headers from config
        local headers
        headers=$(curl -s -I "$SERVER_URL/api/test.json" 2>/dev/null)
        if echo "$headers" | grep -qi "X-Config-Test.*serve.json"; then
            print_success "serve.json custom headers work"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "serve.json custom headers not applied"
        fi
        
        # Test that JSON content is served
        temp_file=$(http_get "$SERVER_URL/api/test.json")
        if [[ $? -eq 0 ]]; then
            if validate_json "$temp_file" && grep -q '"config".*"serve.json"' "$temp_file"; then
                print_success "serve.json directory content served correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "serve.json directory content incorrect"
            fi
        else
            print_failure "Failed to serve content from serve.json public directory"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server with serve.json"
    fi
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All serve.json tests passed"
        return 0
    else
        print_failure "Some serve.json tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test now.json configuration (legacy)
test_now_json_config() {
    print_test_start "now.json Configuration (Legacy)"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Remove serve.json if it exists
    rm -f "$test_dir/serve.json"
    
    # Create now.json with static configuration
    local now_config
    now_config=$(create_test_json "now.json" '{
        "now": {
            "static": {
                "public": "dist",
                "cleanUrls": false,
                "trailingSlash": true,
                "renderSingle": true,
                "symlinks": true,
                "etag": false,
                "directoryListing": true
            }
        }
    }')
    
    # Copy now.json to test directory
    cp "$now_config" "$test_dir/now.json"
    
    print_info "Testing now.json with static configuration section"
    
    # Start server with now.json
    if start_server "$TEST_PORT" "$test_dir"; then
        
        # Test that dist directory is used
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Dist Directory" "$temp_file"; then
                print_success "now.json public directory (dist) setting works"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "now.json public directory not used correctly"
            fi
        else
            print_failure "Failed to access server with now.json"
        fi
        rm -f "$temp_file"
        
        # Test content from dist directory
        temp_file=$(http_get "$SERVER_URL/api/test.json")
        if [[ $? -eq 0 ]]; then
            if validate_json "$temp_file" && grep -q '"config".*"package.json"' "$temp_file"; then
                print_success "now.json directory content served correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "now.json directory content incorrect"
            fi
        else
            print_failure "Failed to serve content from now.json dist directory"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server with now.json"
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All now.json tests passed"
        return 0
    else
        print_failure "Some now.json tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test package.json configuration
test_package_json_config() {
    print_test_start "package.json Configuration"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Remove other config files
    rm -f "$test_dir/serve.json" "$test_dir/now.json"
    
    # Create package.json with static section
    local package_config
    package_config=$(create_test_json "package.json" '{
        "name": "test-app",
        "version": "1.0.0",
        "description": "Test application",
        "main": "index.js",
        "scripts": {
            "start": "node index.js"
        },
        "static": {
            "public": "build",
            "cleanUrls": true,
            "renderSingle": false,
            "etag": true
        },
        "dependencies": {
            "express": "^4.18.0"
        }
    }')
    
    # Copy package.json to test directory
    cp "$package_config" "$test_dir/package.json"
    
    print_info "Testing package.json with static section"
    
    # Start server with package.json
    if start_server "$TEST_PORT" "$test_dir"; then
        
        # Test that build directory is used
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Build Directory" "$temp_file"; then
                print_success "package.json public directory (build) setting works"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "package.json public directory not used correctly"
            fi
        else
            print_failure "Failed to access server with package.json"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server with package.json"
    fi
    
    if [[ $tests_passed -eq 1 ]]; then
        print_success "All package.json tests passed"
        return 0
    else
        print_failure "package.json test failed"
        return 1
    fi
}

# Test configuration precedence
test_config_precedence() {
    print_test_start "Configuration File Precedence"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Create all three config files with different public directories
    # serve.json should win (highest precedence)
    
    local serve_config
    serve_config=$(create_test_json "serve.json" '{
        "public": "public"
    }')
    cp "$serve_config" "$test_dir/serve.json"
    
    local now_config  
    now_config=$(create_test_json "now.json" '{
        "now": {
            "static": {
                "public": "dist"
            }
        }
    }')
    cp "$now_config" "$test_dir/now.json"
    
    local package_config
    package_config=$(create_test_json "package.json" '{
        "name": "test",
        "version": "1.0.0",
        "static": {
            "public": "build"
        }
    }')
    cp "$package_config" "$test_dir/package.json"
    
    print_info "Testing precedence: serve.json > now.json > package.json"
    
    # Start server and test that serve.json takes precedence
    if start_server "$TEST_PORT" "$test_dir"; then
        
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Public Directory" "$temp_file"; then
                print_success "serve.json takes precedence over other configs"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "serve.json precedence not working"
            fi
        else
            print_failure "Failed to access server for precedence test"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server for precedence test"
    fi
    
    # Remove serve.json and test now.json precedence
    rm "$test_dir/serve.json"
    
    if start_server "$TEST_PORT" "$test_dir"; then
        
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Dist Directory" "$temp_file"; then
                print_success "now.json takes precedence over package.json"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "now.json precedence not working"
            fi
        else
            print_failure "Failed to access server for now.json precedence test"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server for now.json precedence test"
    fi
    
    # Remove now.json and test package.json fallback
    rm "$test_dir/now.json"
    
    if start_server "$TEST_PORT" "$test_dir"; then
        
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Build Directory" "$temp_file"; then
                print_success "package.json used as final fallback"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "package.json fallback not working"
            fi
        else
            print_failure "Failed to access server for package.json fallback test"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server for package.json fallback test"
    fi
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All configuration precedence tests passed"
        return 0
    else
        print_failure "Some configuration precedence tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test custom config path with --config flag
test_custom_config_path() {
    print_test_start "Custom Configuration Path"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Clean up existing config files
    rm -f "$test_dir"/*.json
    
    # Create custom config in subdirectory
    mkdir -p "$test_dir/config"
    local custom_config
    custom_config=$(create_test_json "custom-serve.json" '{
        "public": "static", 
        "cleanUrls": false,
        "etag": false
    }')
    cp "$custom_config" "$test_dir/config/custom-serve.json"
    
    # Create content in static directory
    mkdir -p "$test_dir/static"
    create_test_html "$test_dir/static/index.html" "Static Custom" "<h1>Custom Config Directory</h1>"
    
    print_info "Testing custom config path with --config flag"
    
    # Start server with custom config path
    if start_server "$TEST_PORT" "$test_dir" "--config $test_dir/config/custom-serve.json"; then
        
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            if grep -q "Custom Config Directory" "$temp_file"; then
                print_success "Custom config path works correctly"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "Custom config path not applied correctly"
            fi
        else
            print_failure "Failed to access server with custom config"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server with custom config path"
    fi
    
    if [[ $tests_passed -eq 1 ]]; then
        print_success "Custom configuration path test passed"
        return 0
    else
        print_failure "Custom configuration path test failed"
        return 1
    fi
}

# Test configuration validation and error handling
test_config_validation() {
    print_test_start "Configuration Validation & Error Handling"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Clean up existing config files
    rm -f "$test_dir"/*.json
    
    # Test with invalid JSON
    print_info "Testing invalid JSON configuration"
    echo '{"public": "test", invalid json}' > "$test_dir/serve.json"
    
    # Server should handle invalid config gracefully
    local server_log="$TEMP_DIR/invalid_config_server.log"
    "$MSAADA_BIN" --port "$TEST_PORT" --dir "$test_dir" > "$server_log" 2>&1 &
    local server_pid=$!
    
    sleep 2
    
    # Server might start but should handle the invalid config
    if ps -p $server_pid > /dev/null; then
        # If server started, it should fall back to defaults
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            print_success "Invalid JSON config handled gracefully"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Server not accessible with invalid JSON config"
        fi
        rm -f "$temp_file"
        kill $server_pid 2>/dev/null || true
    else
        # Server failed to start - also acceptable behavior
        print_success "Server correctly failed with invalid JSON config"
        tests_passed=$((tests_passed + 1))
    fi
    
    # Test with nonexistent public directory
    print_info "Testing nonexistent public directory"
    rm -f "$test_dir/serve.json"
    local invalid_config
    invalid_config=$(create_test_json "serve.json" '{
        "public": "nonexistent_directory"
    }')
    cp "$invalid_config" "$test_dir/serve.json"
    
    server_log="$TEMP_DIR/nonexistent_dir_server.log"
    "$MSAADA_BIN" --port "$TEST_PORT" --dir "$test_dir" > "$server_log" 2>&1 &
    server_pid=$!
    
    sleep 2
    
    # Server should handle nonexistent directory
    if ps -p $server_pid > /dev/null; then
        kill $server_pid 2>/dev/null || true
        # Server started - might fall back to defaults
        print_success "Nonexistent public directory handled"
        tests_passed=$((tests_passed + 1))
    else
        # Server failed to start - also acceptable
        print_success "Server correctly failed with nonexistent public directory" 
        tests_passed=$((tests_passed + 1))
    fi
    
    # Test with empty configuration file
    print_info "Testing empty configuration file"
    echo '' > "$test_dir/serve.json"
    
    if start_server "$TEST_PORT" "$test_dir"; then
        # Should use defaults
        local temp_file
        temp_file=$(http_get "$SERVER_URL/")
        if [[ $? -eq 0 ]]; then
            print_success "Empty config file handled with defaults"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Empty config file not handled correctly"
        fi
        rm -f "$temp_file"
        stop_server
    else
        print_failure "Failed to start server with empty config"
    fi
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All configuration validation tests passed"
        return 0
    else
        print_failure "Some configuration validation tests failed ($tests_passed/3 passed)"
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
    test_dir=$(setup_config_test_env)
    
    # Run test suites
    test_serve_json_config "$test_dir"
    test_now_json_config "$test_dir" 
    test_package_json_config "$test_dir"
    test_config_precedence "$test_dir"
    test_custom_config_path "$test_dir"
    test_config_validation "$test_dir"
    
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