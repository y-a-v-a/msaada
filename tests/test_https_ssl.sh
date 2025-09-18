#!/bin/bash
# test_https_ssl.sh - HTTPS/SSL integration tests

# Source the test utilities
source "$(dirname "${BASH_SOURCE[0]}")/test_utils.sh"

# Test configuration
TEST_SUITE_NAME="HTTPS/SSL Integration"
TEST_PORT=3443

# Initialize test suite
init_test_utils "$TEST_SUITE_NAME"

# Check OpenSSL availability
check_openssl() {
    if ! command -v openssl >/dev/null 2>&1; then
        print_error "OpenSSL not found. SSL tests will be skipped."
        return 1
    fi
    return 0
}

# Create test directory with content
setup_ssl_test_env() {
    print_subheader "Setting up SSL test environment"
    
    local test_dir="$TEMP_DIR/ssl_test_files"
    mkdir -p "$test_dir"
    
    # Create a simple HTML page for HTTPS testing
    create_test_html "$test_dir/index.html" "HTTPS Test" "<h1>Secure Connection</h1><p>This page is served over HTTPS.</p>"
    
    # Create JSON API endpoint content
    echo '{"status": "secure", "protocol": "https", "message": "SSL working"}' > "$test_dir/api.json"
    
    print_success "SSL test environment created"
    echo "$test_dir"
}

# Test PEM certificate format
test_pem_certificates() {
    print_test_start "PEM Certificate Format"
    
    if ! check_openssl; then
        print_skip "OpenSSL not available"
        return 0
    fi
    
    local test_dir="$1"
    local cert_files
    cert_files=$(create_test_certificate "pem_test")
    
    if [[ $? -ne 0 ]]; then
        print_failure "Failed to create test certificates"
        return 1
    fi
    
    local cert_path key_path
    cert_path=$(echo "$cert_files" | cut -d' ' -f1)
    key_path=$(echo "$cert_files" | cut -d' ' -f2)
    
    print_info "Testing with cert: $cert_path, key: $key_path"
    
    # Start HTTPS server with PEM certificates
    local https_port
    https_port=$((TEST_PORT + 1))
    local https_url="https://localhost:$https_port"
    
    print_info "Starting HTTPS server on port $https_port"
    local server_log="$TEMP_DIR/https_pem_server.log"
    "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
        --ssl-cert "$cert_path" --ssl-key "$key_path" > "$server_log" 2>&1 &
    local server_pid=$!
    
    # Wait for server to start
    sleep 2
    
    # Check if server started successfully
    if ! ps -p $server_pid > /dev/null; then
        print_failure "HTTPS server failed to start with PEM certificates"
        print_info "Server log:"
        cat "$server_log"
        return 1
    fi
    
    local tests_passed=0
    
    # Test HTTPS connection (with -k flag to ignore self-signed cert)
    local temp_file
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -k -s -o "$temp_file" -w "%{http_code}" "$https_url/" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if grep -q "Secure Connection" "$temp_file"; then
            print_success "HTTPS with PEM certificates works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "HTTPS PEM response content incorrect"
        fi
    else
        print_failure "HTTPS with PEM certificates failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test TLS handshake details
    local tls_info
    tls_info=$(openssl s_client -connect "localhost:$https_port" -servername localhost </dev/null 2>/dev/null | grep -E "(subject=|issuer=|Protocol|Cipher)")
    
    if [[ -n "$tls_info" ]]; then
        print_success "TLS handshake successful"
        print_info "TLS Details: $(echo "$tls_info" | tr '\n' ' ')"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "TLS handshake failed"
    fi
    
    # Test HTTP headers over HTTPS
    local headers
    headers=$(curl -k -s -I "$https_url/" 2>/dev/null)
    if echo "$headers" | grep -qi "HTTP.*200"; then
        print_success "HTTPS headers received correctly"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "HTTPS headers incorrect"
    fi
    
    # Stop the server
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All PEM certificate tests passed"
        return 0
    else
        print_failure "Some PEM certificate tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test PKCS12 certificate format
test_pkcs12_certificates() {
    print_test_start "PKCS12 Certificate Format"
    
    if ! check_openssl; then
        print_skip "OpenSSL not available"
        return 0
    fi
    
    local test_dir="$1"
    local p12_files
    p12_files=$(create_test_pkcs12 "p12_test" "testpass123")
    
    if [[ $? -ne 0 ]]; then
        print_failure "Failed to create PKCS12 test certificates"
        return 1
    fi
    
    local p12_path pass_path
    p12_path=$(echo "$p12_files" | cut -d' ' -f1)
    pass_path=$(echo "$p12_files" | cut -d' ' -f2)
    
    print_info "Testing with PKCS12: $p12_path, passphrase: $pass_path"
    
    # Start HTTPS server with PKCS12 certificates
    local https_port
    https_port=$((TEST_PORT + 2))
    local https_url="https://localhost:$https_port"
    
    print_info "Starting HTTPS server on port $https_port"
    local server_log="$TEMP_DIR/https_p12_server.log"
    "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
        --ssl-cert "$p12_path" --ssl-pass "$pass_path" > "$server_log" 2>&1 &
    local server_pid=$!
    
    # Wait for server to start
    sleep 2
    
    # Check if server started successfully
    if ! ps -p $server_pid > /dev/null; then
        print_failure "HTTPS server failed to start with PKCS12 certificates"
        print_info "Server log:"
        cat "$server_log"
        return 1
    fi
    
    local tests_passed=0
    
    # Test HTTPS connection
    local temp_file
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -k -s -o "$temp_file" -w "%{http_code}" "$https_url/" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if grep -q "Secure Connection" "$temp_file"; then
            print_success "HTTPS with PKCS12 certificates works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "HTTPS PKCS12 response content incorrect"
        fi
    else
        print_failure "HTTPS with PKCS12 certificates failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Test JSON API over HTTPS
    temp_file=$(mktemp -p "$TEMP_DIR")
    status_code=$(curl -k -s -o "$temp_file" -w "%{http_code}" "$https_url/api.json" 2>/dev/null)
    
    if [[ "$status_code" == "200" ]]; then
        if validate_json "$temp_file" && grep -q '"protocol".*"https"' "$temp_file"; then
            print_success "JSON API over HTTPS works correctly"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "JSON API over HTTPS content incorrect"
        fi
    else
        print_failure "JSON API over HTTPS failed (status: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Stop the server
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true
    
    if [[ $tests_passed -eq 2 ]]; then
        print_success "All PKCS12 certificate tests passed"
        return 0
    else
        print_failure "Some PKCS12 certificate tests failed ($tests_passed/2 passed)"
        return 1
    fi
}

# Test SSL/TLS security features
test_ssl_security() {
    print_test_start "SSL/TLS Security Features"
    
    if ! check_openssl; then
        print_skip "OpenSSL not available"
        return 0
    fi
    
    local test_dir="$1"
    local cert_files
    cert_files=$(create_test_certificate "security_test")
    
    if [[ $? -ne 0 ]]; then
        print_failure "Failed to create test certificates for security testing"
        return 1
    fi
    
    local cert_path key_path
    cert_path=$(echo "$cert_files" | cut -d' ' -f1)
    key_path=$(echo "$cert_files" | cut -d' ' -f2)
    
    # Start HTTPS server
    local https_port
    https_port=$((TEST_PORT + 3))
    local https_url="https://localhost:$https_port"
    
    print_info "Starting HTTPS server for security testing on port $https_port"
    local server_log="$TEMP_DIR/https_security_server.log"
    "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
        --ssl-cert "$cert_path" --ssl-key "$key_path" > "$server_log" 2>&1 &
    local server_pid=$!
    
    sleep 2
    
    if ! ps -p $server_pid > /dev/null; then
        print_failure "HTTPS server failed to start for security testing"
        return 1
    fi
    
    local tests_passed=0
    
    # Test TLS version support
    local tls_version
    tls_version=$(openssl s_client -connect "localhost:$https_port" -tls1_2 </dev/null 2>/dev/null | grep "Protocol.*TLSv1")
    
    if [[ -n "$tls_version" ]]; then
        print_success "TLS version negotiation works"
        print_info "Detected: $(echo "$tls_version" | tr -d '\n')"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "TLS version negotiation failed"
    fi
    
    # Test cipher suite negotiation
    local cipher_info
    cipher_info=$(openssl s_client -connect "localhost:$https_port" -cipher 'HIGH:!aNULL:!eNULL:!EXPORT:!DES:!RC4:!MD5:!PSK:!SRP:!CAMELLIA' </dev/null 2>/dev/null | grep "Cipher.*:")
    
    if [[ -n "$cipher_info" ]]; then
        print_success "Secure cipher suite negotiated"
        print_info "Cipher: $(echo "$cipher_info" | tr -d '\n')"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Cipher suite negotiation failed"
    fi
    
    # Test that HTTP redirects to HTTPS are NOT automatic (msaada serves HTTPS only on HTTPS port)
    local http_url="http://localhost:$https_port"
    local temp_file
    temp_file=$(mktemp -p "$TEMP_DIR")
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$http_url/" 2>/dev/null || echo "000")
    
    # Should fail to connect or return error since we're trying HTTP on HTTPS port
    if [[ "$status_code" == "000" ]] || [[ "$status_code" =~ ^(400|403|404|5[0-9][0-9])$ ]]; then
        print_success "HTTP on HTTPS port correctly rejected"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "HTTP on HTTPS port not handled correctly (got: $status_code)"
    fi
    rm -f "$temp_file"
    
    # Stop the server
    kill $server_pid 2>/dev/null || true
    wait $server_pid 2>/dev/null || true
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All SSL security tests passed"
        return 0
    else
        print_failure "Some SSL security tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Test SSL error conditions
test_ssl_error_conditions() {
    print_test_start "SSL Error Conditions"
    
    if ! check_openssl; then
        print_skip "OpenSSL not available"
        return 0
    fi
    
    local test_dir="$1"
    local tests_passed=0
    
    # Test with invalid certificate path
    print_info "Testing invalid certificate path"
    local server_log="$TEMP_DIR/https_invalid_cert.log"
    local https_port
    https_port=$((TEST_PORT + 4))
    
    "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
        --ssl-cert "/nonexistent/cert.pem" --ssl-key "/nonexistent/key.pem" > "$server_log" 2>&1 &
    local server_pid=$!
    
    sleep 2
    
    # Server should fail to start
    if ! ps -p $server_pid > /dev/null; then
        print_success "Server correctly failed with invalid certificate path"
        tests_passed=$((tests_passed + 1))
    else
        print_failure "Server should have failed with invalid certificate path"
        kill $server_pid 2>/dev/null || true
    fi
    
    # Test with mismatched cert and key
    print_info "Testing mismatched certificate and key"
    local cert_files1 cert_files2
    cert_files1=$(create_test_certificate "mismatch1")
    cert_files2=$(create_test_certificate "mismatch2")
    
    if [[ $? -eq 0 ]]; then
        local cert1_path key2_path
        cert1_path=$(echo "$cert_files1" | cut -d' ' -f1)
        key2_path=$(echo "$cert_files2" | cut -d' ' -f2)
        
        https_port=$((TEST_PORT + 5))
        server_log="$TEMP_DIR/https_mismatch.log"
        
        "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
            --ssl-cert "$cert1_path" --ssl-key "$key2_path" > "$server_log" 2>&1 &
        server_pid=$!
        
        sleep 2
        
        # Server should fail to start or clients should fail to connect
        local connection_works=false
        if ps -p $server_pid > /dev/null; then
            # If server started, test if connections work
            if curl -k -s "https://localhost:$https_port/" > /dev/null 2>&1; then
                connection_works=true
            fi
            kill $server_pid 2>/dev/null || true
        fi
        
        if ! $connection_works; then
            print_success "Mismatched certificate and key correctly rejected"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Mismatched certificate and key should have been rejected"
        fi
    else
        print_skip "Could not create certificates for mismatch test"
    fi
    
    # Test PKCS12 with wrong passphrase
    print_info "Testing PKCS12 with wrong passphrase"
    local p12_files
    p12_files=$(create_test_pkcs12 "wrong_pass_test" "correctpass")
    
    if [[ $? -eq 0 ]]; then
        local p12_path
        p12_path=$(echo "$p12_files" | cut -d' ' -f1)
        
        # Create wrong passphrase file
        echo "wrongpassword" > "$TEMP_DIR/wrong_pass.txt"
        
        https_port=$((TEST_PORT + 6))
        server_log="$TEMP_DIR/https_wrong_pass.log"
        
        "$MSAADA_BIN" --port "$https_port" --dir "$test_dir" \
            --ssl-cert "$p12_path" --ssl-pass "$TEMP_DIR/wrong_pass.txt" > "$server_log" 2>&1 &
        server_pid=$!
        
        sleep 2
        
        # Server should fail to start
        if ! ps -p $server_pid > /dev/null; then
            print_success "Wrong PKCS12 passphrase correctly rejected"
            tests_passed=$((tests_passed + 1))
        else
            print_failure "Wrong PKCS12 passphrase should have been rejected"
            kill $server_pid 2>/dev/null || true
        fi
    else
        print_skip "Could not create PKCS12 for passphrase test"
    fi
    
    if [[ $tests_passed -eq 3 ]]; then
        print_success "All SSL error condition tests passed"
        return 0
    else
        print_failure "Some SSL error condition tests failed ($tests_passed/3 passed)"
        return 1
    fi
}

# Main test execution
main() {
    print_header "Starting $TEST_SUITE_NAME Tests"
    
    # Check prerequisites
    if ! check_openssl; then
        print_error "OpenSSL is required for SSL/TLS tests"
        exit 1
    fi
    
    if ! ensure_msaada_binary; then
        print_error "Failed to ensure msaada binary"
        exit 1
    fi
    
    # Setup test environment
    local test_dir
    test_dir=$(setup_ssl_test_env)
    
    # Run test suites
    test_pem_certificates "$test_dir"
    test_pkcs12_certificates "$test_dir"
    test_ssl_security "$test_dir"
    test_ssl_error_conditions "$test_dir"
    
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