#!/bin/bash
# test_utils.sh - Shared utility functions for msaada feature testing

# Configuration constants
DEFAULT_TEST_PORT=3099
MSAADA_BIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MSAADA_BIN="$MSAADA_BIN_DIR/target/release/msaada"
TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_SERVER_DIR="$TESTS_DIR/test_server"
TEMP_DIR="$TESTS_DIR/.tmp"

# Color constants
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[0;33m'
export BLUE='\033[0;34m'
export PURPLE='\033[0;35m'
export CYAN='\033[0;36m'
export WHITE='\033[0;37m'
export BOLD='\033[1m'
export NC='\033[0m' # No Color

# Test state tracking
TEST_COUNT=0
PASSED_COUNT=0
FAILED_COUNT=0
SKIPPED_COUNT=0
CURRENT_TEST_SUITE=""

# Server management
SERVER_PID=""
SERVER_PORT=""
SERVER_URL=""
SERVER_LOG=""

# Clean up test environment before starting
cleanup_test_environment() {
    local verbose="${1:-false}"

    # Remove any lingering test processes
    local test_pids=$(pgrep -f "msaada.*--port" 2>/dev/null || true)
    if [[ -n "$test_pids" ]]; then
        if [[ "$verbose" == "true" ]]; then
            print_info "Cleaning up lingering test processes: $test_pids"
        fi
        echo "$test_pids" | xargs kill 2>/dev/null || true
        sleep 0.5
        # Force kill if still running
        local remaining_pids=$(pgrep -f "msaada.*--port" 2>/dev/null || true)
        if [[ -n "$remaining_pids" ]]; then
            echo "$remaining_pids" | xargs kill -9 2>/dev/null || true
        fi
    fi

    # Clean up temp directory
    if [[ -d "$TEMP_DIR" ]]; then
        local file_count=$(find "$TEMP_DIR" -type f 2>/dev/null | wc -l | tr -d ' ')
        if [[ $file_count -gt 0 ]]; then
            if [[ "$verbose" == "true" ]]; then
                print_info "Cleaning up $file_count temporary files"
            fi
            rm -rf "$TEMP_DIR" 2>/dev/null || true
        fi
    fi

    # Recreate clean temp directory
    mkdir -p "$TEMP_DIR"

    # Remove any lock files in the tests directory
    find "$TESTS_DIR" -maxdepth 2 -name "*.pid" -o -name "*.lock" -o -name ".lock*" -type f -delete 2>/dev/null || true
}

# Initialize test utilities
init_test_utils() {
    local suite_name="$1"
    local clean_first="${2:-true}"  # Clean by default, pass false to skip
    CURRENT_TEST_SUITE="$suite_name"

    # Clean up test environment first (unless explicitly disabled)
    if [[ "$clean_first" != "false" ]]; then
        cleanup_test_environment "false"  # Quiet cleanup during init
    else
        # Just ensure temp directory exists
        mkdir -p "$TEMP_DIR"
    fi

    # Reset counters
    TEST_COUNT=0
    PASSED_COUNT=0
    FAILED_COUNT=0
    SKIPPED_COUNT=0

    print_header "Initializing $suite_name Test Suite"
    echo -e "Tests Directory: ${CYAN}$TESTS_DIR${NC}"
    echo -e "Temp Directory: ${CYAN}$TEMP_DIR${NC}"
    echo -e "Msaada Binary: ${CYAN}$MSAADA_BIN${NC}"
}

# Print functions
print_header() {
    echo -e "\n${BLUE}${BOLD}===============================================${NC}" >&2
    echo -e "${BLUE}${BOLD} $1 ${NC}" >&2
    echo -e "${BLUE}${BOLD}===============================================${NC}\n" >&2
}

print_subheader() {
    echo -e "\n${CYAN}${BOLD}--- $1 ---${NC}" >&2
}

print_test_start() {
    local test_name="$1"
    echo -e "${YELLOW}▶ Testing: ${test_name}${NC}" >&2
    TEST_COUNT=$((TEST_COUNT + 1))
}

print_success() {
    echo -e "  ${GREEN}✓ SUCCESS:${NC} $1" >&2
    PASSED_COUNT=$((PASSED_COUNT + 1))
}

print_failure() {
    echo -e "  ${RED}✗ FAILED:${NC} $1" >&2
    FAILED_COUNT=$((FAILED_COUNT + 1))
}

print_skip() {
    echo -e "  ${YELLOW}⊘ SKIPPED:${NC} $1" >&2
    SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
}

print_info() {
    echo -e "  ${BLUE}ℹ INFO:${NC} $1" >&2
}

print_warning() {
    echo -e "  ${YELLOW}⚠ WARNING:${NC} $1" >&2
}

print_error() {
    echo -e "  ${RED}❌ ERROR:${NC} $1" >&2
}

# Binary management
ensure_msaada_binary() {
    if [[ ! -f "$MSAADA_BIN" ]]; then
        print_info "Msaada binary not found, building..."
        cd "$MSAADA_BIN_DIR" && cargo build --release
        
        if [[ ! -f "$MSAADA_BIN" ]]; then
            print_error "Failed to build msaada binary!"
            return 1
        fi
        print_success "Built msaada binary"
    fi
    return 0
}

# Server management functions
start_server() {
    local port="${1:-$DEFAULT_TEST_PORT}"
    local dir="${2:-$TEST_SERVER_DIR}"
    local extra_args="$3"
    
    # Ensure binary exists
    ensure_msaada_binary || return 1
    
    # Kill any existing server on this port
    stop_server_on_port "$port"
    
    SERVER_PORT="$port"
    SERVER_URL="http://localhost:$port"
    SERVER_LOG="$TEMP_DIR/server_${port}.log"
    
    print_info "Starting server on port $port with directory $dir"
    
    # Start server in background
    "$MSAADA_BIN" --port "$port" --dir "$dir" $extra_args > "$SERVER_LOG" 2>&1 &
    SERVER_PID=$!
    
    # Wait for server to start
    local max_attempts=10
    local attempt=0
    
    while [[ $attempt -lt $max_attempts ]]; do
        if curl -s "$SERVER_URL" > /dev/null 2>&1; then
            print_success "Server started with PID $SERVER_PID"
            return 0
        fi
        sleep 0.5
        attempt=$((attempt + 1))
    done
    
    print_error "Server failed to start within ${max_attempts} attempts"
    print_error "Server log:"
    cat "$SERVER_LOG"
    return 1
}

stop_server() {
    if [[ -n "$SERVER_PID" ]]; then
        print_info "Stopping server (PID: $SERVER_PID)"
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
        SERVER_PID=""
        SERVER_PORT=""
        SERVER_URL=""
        SERVER_LOG=""
    fi
}

stop_server_on_port() {
    local port="$1"
    pkill -f "msaada.*--port $port" >/dev/null 2>&1 || true
    sleep 0.5
}

# HTTP testing functions
http_get() {
    local url="$1"
    local expected_status="${2:-200}"
    local temp_file
    temp_file="$(mktemp -p "$TEMP_DIR")"
    
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" "$url" 2>/dev/null)
    
    echo "$temp_file" # Return the temp file path
    
    if [[ "$status_code" == "$expected_status" ]]; then
        return 0
    else
        return 1
    fi
}

http_post() {
    local url="$1"
    local content_type="$2"
    local data="$3"
    local expected_status="${4:-200}"
    local temp_file
    temp_file="$(mktemp -p "$TEMP_DIR")"
    
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -H "Content-Type: $content_type" \
        -d "$data" \
        "$url" 2>/dev/null)
    
    echo "$temp_file" # Return the temp file path
    
    if [[ "$status_code" == "$expected_status" ]]; then
        return 0
    else
        return 1
    fi
}

http_upload() {
    local url="$1"
    local file_path="$2"
    local field_name="${3:-file}"
    local expected_status="${4:-200}"
    local temp_file
    temp_file="$(mktemp -p "$TEMP_DIR")"
    
    local status_code
    status_code=$(curl -s -o "$temp_file" -w "%{http_code}" \
        -X POST \
        -F "$field_name=@$file_path" \
        "$url" 2>/dev/null)
    
    echo "$temp_file" # Return the temp file path
    
    if [[ "$status_code" == "$expected_status" ]]; then
        return 0
    else
        return 1
    fi
}

# Validation functions
validate_json() {
    local file="$1"
    jq -e . "$file" >/dev/null 2>&1
}

validate_json_field() {
    local file="$1"
    local field="$2"
    jq -e ".$field" "$file" >/dev/null 2>&1
}

validate_header() {
    local url="$1"
    local header_name="$2"
    local expected_value="$3"
    
    local actual_value
    actual_value=$(curl -s -I "$url" | grep -i "^$header_name:" | cut -d' ' -f2- | tr -d '\r\n')
    
    if [[ "$actual_value" == "$expected_value" ]]; then
        return 0
    else
        return 1
    fi
}

validate_content_type() {
    local url="$1"
    local expected_type="$2"
    
    local content_type
    content_type=$(curl -s -I "$url" | grep -i "^content-type:" | cut -d' ' -f2- | tr -d '\r\n')
    
    if [[ "$content_type" =~ $expected_type ]]; then
        return 0
    else
        return 1
    fi
}

# File creation utilities
create_test_file() {
    local filepath="$1"
    local content="$2"
    
    # If filepath is not absolute, prepend TEMP_DIR
    if [[ "$filepath" != /* ]]; then
        filepath="$TEMP_DIR/$filepath"
    fi
    
    echo "$content" > "$filepath"
    # Only echo filepath if TEST_UTILS_RETURN_PATHS is set
    if [[ "$TEST_UTILS_RETURN_PATHS" == "true" ]]; then
        echo "$filepath"
    fi
}

create_test_json() {
    local filepath="$1"
    local json_content="$2"
    
    # If filepath is not absolute, prepend TEMP_DIR
    if [[ "$filepath" != /* ]]; then
        filepath="$TEMP_DIR/$filepath"
    fi
    
    echo "$json_content" | jq . > "$filepath" 2>/dev/null || {
        echo "$json_content" > "$filepath"
    }
    # Only echo filepath if TEST_UTILS_RETURN_PATHS is set
    if [[ "$TEST_UTILS_RETURN_PATHS" == "true" ]]; then
        echo "$filepath"
    fi
}

create_test_html() {
    local filepath="$1"
    local title="${2:-Test Page}"
    local body="${3:-<h1>Test Content</h1>}"
    
    # If filepath is not absolute, prepend TEMP_DIR
    if [[ "$filepath" != /* ]]; then
        filepath="$TEMP_DIR/$filepath"
    fi
    
    cat > "$filepath" << EOF
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>$title</title>
</head>
<body>
    $body
</body>
</html>
EOF
    # Only echo filepath if TEST_UTILS_RETURN_PATHS is set
    if [[ "$TEST_UTILS_RETURN_PATHS" == "true" ]]; then
        echo "$filepath"
    fi
}

# SSL/TLS utilities
create_test_certificate() {
    local cert_name="${1:-test}"
    local cert_path="$TEMP_DIR/${cert_name}.pem"
    local key_path="$TEMP_DIR/${cert_name}-key.pem"
    
    # Generate self-signed certificate for testing
    openssl req -x509 -newkey rsa:2048 -keyout "$key_path" -out "$cert_path" \
        -days 1 -nodes -subj "/CN=localhost" >/dev/null 2>&1
    
    if [[ -f "$cert_path" && -f "$key_path" ]]; then
        echo "$cert_path $key_path"
        return 0
    else
        return 1
    fi
}

create_test_pkcs12() {
    local cert_name="${1:-test}"
    local password="${2:-testpass}"
    local p12_path="$TEMP_DIR/${cert_name}.p12"
    local pass_path="$TEMP_DIR/${cert_name}-pass.txt"
    
    # First create PEM certificates
    local cert_files
    cert_files=$(create_test_certificate "$cert_name")
    if [[ $? -ne 0 ]]; then
        return 1
    fi
    
    local cert_path key_path
    cert_path=$(echo "$cert_files" | cut -d' ' -f1)
    key_path=$(echo "$cert_files" | cut -d' ' -f2)
    
    # Convert to PKCS12
    echo "$password" > "$pass_path"
    openssl pkcs12 -export -out "$p12_path" -inkey "$key_path" -in "$cert_path" \
        -passout "pass:$password" >/dev/null 2>&1
    
    if [[ -f "$p12_path" ]]; then
        echo "$p12_path $pass_path"
        return 0
    else
        return 1
    fi
}

# Port utilities
find_free_port() {
    local start_port="${1:-3000}"
    local max_attempts=100
    
    for ((i=0; i<max_attempts; i++)); do
        local port=$((start_port + i))
        if ! netstat -ln 2>/dev/null | grep -q ":$port "; then
            echo "$port"
            return 0
        fi
    done
    return 1
}

wait_for_port() {
    local port="$1"
    local max_attempts="${2:-20}"
    local attempt=0
    
    while [[ $attempt -lt $max_attempts ]]; do
        if netstat -ln 2>/dev/null | grep -q ":$port "; then
            return 0
        fi
        sleep 0.1
        attempt=$((attempt + 1))
    done
    return 1
}

# Test reporting
print_test_summary() {
    print_header "Test Summary for $CURRENT_TEST_SUITE"
    
    echo -e "Total Tests: ${CYAN}$TEST_COUNT${NC}"
    echo -e "Passed: ${GREEN}$PASSED_COUNT${NC}"
    echo -e "Failed: ${RED}$FAILED_COUNT${NC}"
    echo -e "Skipped: ${YELLOW}$SKIPPED_COUNT${NC}"
    
    if [[ $FAILED_COUNT -eq 0 ]]; then
        echo -e "\n${GREEN}${BOLD}✅ ALL TESTS PASSED!${NC}"
        return 0
    else
        echo -e "\n${RED}${BOLD}❌ SOME TESTS FAILED!${NC}"
        return 1
    fi
}

# Cleanup functions
cleanup_test_utils() {
    stop_server
    
    # Clean up temp files older than 1 hour
    find "$TEMP_DIR" -type f -mtime +1h -delete 2>/dev/null || true
}

# Trap to ensure cleanup on exit
trap cleanup_test_utils EXIT

# Export all functions for use in other scripts
export -f init_test_utils cleanup_test_utils cleanup_test_environment
export -f print_header print_subheader print_test_start
export -f print_success print_failure print_skip print_info print_warning print_error
export -f ensure_msaada_binary
export -f start_server stop_server stop_server_on_port
export -f http_get http_post http_upload
export -f validate_json validate_json_field validate_header validate_content_type
export -f create_test_file create_test_json create_test_html
export -f create_test_certificate create_test_pkcs12
export -f find_free_port wait_for_port
export -f print_test_summary