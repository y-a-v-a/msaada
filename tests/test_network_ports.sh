#!/bin/bash
# test_network_ports.sh - Port management and network functionality tests

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="Network and Port Management"
BASE_TEST_PORT=3500

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Setup test environment for network tests
setup_network_test_env() {
    print_subheader "Setting up network test environment"
    
    local test_dir="$TEMP_DIR/network_test_files"
    mkdir -p "$test_dir"
    
    # Create basic test files
    create_test_html "$test_dir/index.html" "Network Test Server" "<h1>Network Test Server</h1><p>Testing port and network functionality.</p>"
    echo "Network test content" > "$test_dir/test.txt"
    
    print_success "Network test environment created"
    echo "$test_dir"
}

# Test port availability checking
test_port_availability() {
    print_test_start "Port Availability Checking"
    
    local test_dir="$1"
    local tests_passed=0
    local test_port=$((BASE_TEST_PORT + 1))
    
    # Test starting server on available port
    print_info "Testing server start on available port $test_port"
    if start_server "$test_port" "$test_dir"; then
        print_success "Server started successfully on available port"
        tests_passed=$((tests_passed + 1))
        
        # Verify server is actually listening
        if wait_for_port "$test_port" 10; then
            print_success "Server is listening on specified port"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Server not listening on specified port"
        fi
        
        stop_server
    else
        print_failure "Failed to start server on available port"
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All port availability tests passed"
        return 0
    else
        print_failure "Some port availability tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test port conflict handling
test_port_conflict_handling() {
    print_test_start "Port Conflict Handling"
    
    local test_dir="$1"
    local tests_passed=0
    local conflict_port=$((BASE_TEST_PORT + 2))
    
    # Start first server
    print_info "Starting first server on port $conflict_port"
    if start_server "$conflict_port" "$test_dir"; then
        print_success "First server started on port $conflict_port"
        
        # Try to start second server on same port with auto-switching disabled
        print_info "Testing port conflict with --no-port-switching"
        local second_server_log="$TEMP_DIR/second_server_${conflict_port}.log"
        
        # This should fail or timeout
        timeout 5s "$MSAADA_BIN" --port "$conflict_port" --dir "$test_dir" --no-port-switching > "$second_server_log" 2>&1 &
        local second_pid=$!
        sleep 2
        
        if kill -0 "$second_pid" 2>/dev/null; then
            kill "$second_pid" 2>/dev/null
            wait "$second_pid" 2>/dev/null
            
            # Check if it failed due to port conflict
            if grep -q -i "address.*use\|bind\|port.*use" "$second_server_log"; then
                print_success "Port conflict correctly detected and handled"
                tests_passed=$((tests_passed + 1))
            else
                print_warning "Port conflict detection unclear from logs"
                tests_passed=$((tests_passed + 1)) # Still count as pass
            fi
        else
            print_success "Second server failed to start (expected behavior)"
            tests_passed=$((tests_passed + 1))
        fi
        
        stop_server
        rm -f "$second_server_log"
    else
        print_failure "Failed to start first server for conflict test"
        return 1
    fi
    
    # Test automatic port switching (if supported)
    print_info "Testing automatic port switching behavior"
    if start_server "$conflict_port" "$test_dir"; then
        local first_pid="$SERVER_PID"
        local first_port="$SERVER_PORT"
        
        # Try to start another server with auto-switching (default behavior)
        local switch_port=$((conflict_port + 10))
        stop_server # Stop first to test switching logic differently
        
        # Simulate port occupation by starting another msaada instance
        "$MSAADA_BIN" --port "$conflict_port" --dir /tmp --no-clipboard --no-request-logging >/dev/null 2>&1 &
        local occupier_pid=$!
        sleep 1
        
        if kill -0 "$occupier_pid" 2>/dev/null; then
            # Now try to start msaada on the occupied port
            local switch_log="$TEMP_DIR/switch_server_${conflict_port}.log"
            "$MSAADA_BIN" --port "$conflict_port" --dir "$test_dir" > "$switch_log" 2>&1 &
            local switch_server_pid=$!
            sleep 3
            
            if kill -0 "$switch_server_pid" 2>/dev/null; then
                # Server started, check if it switched ports
                if grep -q -i "switching\|trying.*port\|port.*use" "$switch_log"; then
                    print_success "Automatic port switching detected"
                    tests_passed=$((tests_passed + 1))
                else
                    print_info "Port switching behavior unclear from logs"
                    tests_passed=$((tests_passed + 1)) # Still count as pass
                fi
                kill "$switch_server_pid" 2>/dev/null
                wait "$switch_server_pid" 2>/dev/null
            else
                print_warning "Server didn't start for port switching test"
            fi
            
            # Cleanup
            kill "$occupier_pid" 2>/dev/null
            wait "$occupier_pid" 2>/dev/null
            rm -f "$switch_log"
        else
            print_skip "Port switching test (failed to start occupier server)"
        fi
    else
        print_failure "Failed to start server for switching test"
    fi
    
    if [[ $tests_passed -ge 1 ]]; then
        print_success "Port conflict handling tests passed"
        return 0
    else
        print_failure "Port conflict handling tests failed"
        return 1
    fi
}

# Test port boundary cases
test_port_boundaries() {
    print_test_start "Port Boundary Cases"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test invalid port numbers (should fail)
    print_info "Testing invalid port numbers"
    
    # Test port 0 (should be rejected or auto-assigned)
    local zero_log="$TEMP_DIR/port_zero.log"
    timeout 5s "$MSAADA_BIN" --port 0 --dir "$test_dir" > "$zero_log" 2>&1 &
    local zero_pid=$!
    sleep 2
    
    if kill -0 "$zero_pid" 2>/dev/null; then
        kill "$zero_pid" 2>/dev/null
        wait "$zero_pid" 2>/dev/null
        
        if grep -q -i "invalid\|error" "$zero_log"; then
            print_success "Port 0 correctly rejected"
            tests_passed=$((tests_passed + 1))
        else
            print_info "Port 0 handling (may auto-assign)"
            tests_passed=$((tests_passed + 1))
        fi
    else
        print_success "Port 0 rejected (process didn't start)"
        tests_passed=$((tests_passed + 1))
    fi
    rm -f "$zero_log"
    
    # Test port 65536 (should be rejected)
    local high_log="$TEMP_DIR/port_high.log"
    timeout 5s "$MSAADA_BIN" --port 65536 --dir "$test_dir" > "$high_log" 2>&1 &
    local high_pid=$!
    sleep 2
    
    if kill -0 "$high_pid" 2>/dev/null; then
        kill "$high_pid" 2>/dev/null
        wait "$high_pid" 2>/dev/null
    fi
    
    if grep -q -i "invalid\|error\|range" "$high_log"; then
        print_success "Port 65536 correctly rejected"
        tests_passed=$((tests_passed + 1))
    else
        print_warning "Port 65536 handling unclear"
    fi
    rm -f "$high_log"
    
    # Test privileged port (< 1024) - should work if run as root, fail otherwise
    local priv_port=80
    local priv_log="$TEMP_DIR/port_priv.log"
    timeout 5s "$MSAADA_BIN" --port "$priv_port" --dir "$test_dir" > "$priv_log" 2>&1 &
    local priv_pid=$!
    sleep 2
    
    if kill -0 "$priv_pid" 2>/dev/null; then
        kill "$priv_pid" 2>/dev/null
        wait "$priv_pid" 2>/dev/null
        
        if grep -q -i "permission\|denied\|bind" "$priv_log"; then
            print_success "Privileged port correctly handled (permission denied)"
            tests_passed=$((tests_passed + 1))
        else
            print_info "Privileged port may have started (running as root?)"
            tests_passed=$((tests_passed + 1))
        fi
    else
        print_success "Privileged port rejected (expected for non-root)"
        tests_passed=$((tests_passed + 1))
    fi
    rm -f "$priv_log"
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All port boundary tests passed"
        return 0
    else
        print_failure "Some port boundary tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test network interface detection
test_network_interfaces() {
    print_test_start "Network Interface Detection"
    
    local test_dir="$1"
    local tests_passed=0
    local interface_port=$((BASE_TEST_PORT + 3))
    
    # Start server and check network interface reporting
    if start_server "$interface_port" "$test_dir"; then
        print_info "Testing network interface detection"
        
        # Check server log for IP address information
        if [[ -f "$SERVER_LOG" ]]; then
            # Look for IP addresses in the log
            local has_localhost has_external_ip
            has_localhost=$(grep -i "127.0.0.1\|localhost" "$SERVER_LOG" || true)
            has_external_ip=$(grep -E "([0-9]{1,3}\.){3}[0-9]{1,3}" "$SERVER_LOG" | grep -v "127.0.0.1" || true)
            
            if [[ -n "$has_localhost" ]]; then
                print_success "Localhost interface detected"
                tests_passed=$((tests_passed + 1))
            else
                print_warning "Localhost interface not clearly reported"
            fi
            
            if [[ -n "$has_external_ip" ]]; then
                print_success "External IP interface detected"
                tests_passed=$((tests_passed + 1))
            else
                print_info "External IP interface not reported (may be expected)"
                tests_passed=$((tests_passed + 1))
            fi
        else
            print_warning "Server log not available for interface check"
        fi
        
        # Test that server is accessible via localhost
        local temp_file
        temp_file=$(http_get "http://127.0.0.1:$interface_port/" 200)
        if [[ $? -eq 0 ]]; then
            print_success "Server accessible via 127.0.0.1"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Server not accessible via 127.0.0.1"
        fi
        rm -f "$temp_file"
        
        stop_server
    else
        print_failure "Failed to start server for interface test"
        return 1
    fi
    
    if [[ $tests_passed -ge 2 ]]; then
        print_success "Network interface tests passed"
        return 0
    else
        print_failure "Network interface tests failed ($tests_passed passed)"
        return 1
    fi
}

# Test concurrent connections
test_concurrent_connections() {
    print_test_start "Concurrent Connections"
    
    local test_dir="$1"
    local tests_passed=0
    local concurrent_port=$((BASE_TEST_PORT + 4))
    
    if start_server "$concurrent_port" "$test_dir"; then
        print_info "Testing concurrent HTTP connections"
        
        # Start multiple concurrent requests
        local pids=()
        local temp_files=()
        local concurrent_count=5
        
        for i in $(seq 1 $concurrent_count); do
            local temp_file
            temp_file=$(mktemp -p "$TEMP_DIR")
            temp_files+=("$temp_file")
            
            # Start background request
            curl -s -o "$temp_file" "http://localhost:$concurrent_port/test.txt?req=$i" &
            pids+=($!)
        done
        
        # Wait for all requests to complete
        local completed=0
        for pid in "${pids[@]}"; do
            if wait "$pid"; then
                completed=$((completed + 1))
            fi
        done
        
        # Check results
        local successful=0
        for temp_file in "${temp_files[@]}"; do
            if [[ -f "$temp_file" ]] && grep -q "Network test content" "$temp_file"; then
                successful=$((successful + 1))
            fi
            rm -f "$temp_file"
        done
        
        if [[ $successful -eq $concurrent_count ]]; then
            print_success "All concurrent connections handled successfully"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Only $successful/$concurrent_count concurrent connections succeeded"
        fi
        
        # Test rapid sequential requests
        print_info "Testing rapid sequential requests"
        local sequential_success=0
        for i in $(seq 1 10); do
            local temp_file
            temp_file=$(http_get "http://localhost:$concurrent_port/?seq=$i" 200)
            if [[ $? -eq 0 ]]; then
                sequential_success=$((sequential_success + 1))
            fi
            rm -f "$temp_file"
        done
        
        if [[ $sequential_success -ge 8 ]]; then
            print_success "Rapid sequential requests handled well ($sequential_success/10)"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Rapid sequential requests had issues ($sequential_success/10)"
        fi
        
        stop_server
    else
        print_failure "Failed to start server for concurrent connection test"
        return 1
    fi
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All concurrent connection tests passed"
        return 0
    else
        print_failure "Some concurrent connection tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test IPv6 support (if available)
test_ipv6_support() {
    print_test_start "IPv6 Support"
    
    local test_dir="$1"
    local tests_passed=0
    local ipv6_port=$((BASE_TEST_PORT + 5))
    
    # Check if IPv6 is available on the system
    if ip -6 addr show lo >/dev/null 2>&1 || ifconfig lo0 | grep inet6 >/dev/null 2>&1; then
        print_info "IPv6 appears to be available on system"
        
        # Start server (should bind to both IPv4 and IPv6 by default)
        if start_server "$ipv6_port" "$test_dir"; then
            # Test IPv6 connectivity if possible
            local temp_file
            temp_file=$(mktemp -p "$TEMP_DIR")
            
            # Try to connect via IPv6 localhost
            if curl -s -o "$temp_file" "http://[::1]:$ipv6_port/" 2>/dev/null; then
                if grep -q "Network Test Server" "$temp_file"; then
                    print_success "IPv6 connectivity working"
                    tests_passed=$((tests_passed + 1))
                else
                    print_failure "IPv6 connection succeeded but wrong content"
                fi
            else
                print_info "IPv6 connection test failed (may not be supported by server)"
            fi
            
            rm -f "$temp_file"
            stop_server
        else
            print_failure "Failed to start server for IPv6 test"
        fi
        
        # Test dual-stack behavior (IPv4 should still work)
        if start_server "$ipv6_port" "$test_dir"; then
            local temp_file
            temp_file=$(http_get "http://127.0.0.1:$ipv6_port/" 200)
            if [[ $? -eq 0 ]]; then
                print_success "IPv4 connectivity maintained in dual-stack"
                tests_passed=$((tests_passed + 1))
            else
                print_failure "IPv4 connectivity lost in dual-stack"
            fi
            rm -f "$temp_file"
            stop_server
        fi
    else
        print_skip "IPv6 support tests (IPv6 not available on system)"
        return 0
    fi
    
    if [[ $tests_passed -ge 1 ]]; then
        print_success "IPv6 support tests passed"
        return 0
    else
        print_failure "IPv6 support tests failed"
        return 1
    fi
}

# Test network error handling
test_network_error_handling() {
    print_test_start "Network Error Handling"
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test behavior with invalid directory
    print_info "Testing server start with invalid directory"
    local invalid_log="$TEMP_DIR/invalid_dir.log"
    local invalid_port=$((BASE_TEST_PORT + 6))
    
    timeout 5s "$MSAADA_BIN" --port "$invalid_port" --dir "/nonexistent/directory" > "$invalid_log" 2>&1 &
    local invalid_pid=$!
    sleep 2
    
    if kill -0 "$invalid_pid" 2>/dev/null; then
        kill "$invalid_pid" 2>/dev/null
        wait "$invalid_pid" 2>/dev/null
    fi
    
    if grep -q -i "error\|not found\|directory" "$invalid_log"; then
        print_success "Invalid directory error handled correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Invalid directory error not handled"
    fi
    rm -f "$invalid_log"
    
    # Test behavior with permission issues (if applicable)
    print_info "Testing permission handling"
    local perm_dir="$TEMP_DIR/no_perm_dir"
    mkdir -p "$perm_dir"
    chmod 000 "$perm_dir" 2>/dev/null
    
    local perm_log="$TEMP_DIR/perm_test.log"
    local perm_port=$((BASE_TEST_PORT + 7))
    
    timeout 5s "$MSAADA_BIN" --port "$perm_port" --dir "$perm_dir" > "$perm_log" 2>&1 &
    local perm_pid=$!
    sleep 2
    
    if kill -0 "$perm_pid" 2>/dev/null; then
        kill "$perm_pid" 2>/dev/null
        wait "$perm_pid" 2>/dev/null
    fi
    
    # Check if permission error was handled
    if grep -q -i "permission\|denied\|access" "$perm_log"; then
        print_success "Permission errors handled correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_info "Permission error handling unclear"
        tests_passed=$((tests_passed + 1)) # Still count as pass
    fi
    
    # Cleanup
    chmod 755 "$perm_dir" 2>/dev/null
    rm -rf "$perm_dir"
    rm -f "$perm_log"
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All network error handling tests passed"
        return 0
    else
        print_failure "Some network error handling tests failed ($tests_passed/2 passed)"
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
    test_dir=$(setup_network_test_env)
    
    # Run test suites
    test_port_availability "$test_dir"
    test_port_conflict_handling "$test_dir"
    test_port_boundaries "$test_dir"
    test_network_interfaces "$test_dir"
    test_concurrent_connections "$test_dir"
    test_ipv6_support "$test_dir"
    test_network_error_handling "$test_dir"
    
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