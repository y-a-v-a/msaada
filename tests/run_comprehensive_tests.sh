#!/bin/bash
# run_comprehensive_tests.sh - Comprehensive test runner for msaada feature tests

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the test utilities
source "$SCRIPT_DIR/test_utils.sh"

# Test runner configuration
TEST_RUNNER_NAME="Comprehensive msaada Test Suite"
OVERALL_START_TIME=""
OVERALL_END_TIME=""

# Test suite definitions
declare -a TEST_SUITES=(
    "test_http_server.sh:Core HTTP Server:Basic HTTP functionality and file serving"
    "test_https_ssl.sh:HTTPS/SSL Integration:SSL/TLS certificate and security features"
    "test_config_files.sh:Configuration Files:JSON configuration and precedence"
    "test_post_enhanced.sh:POST Request Echo:Enhanced POST request handling"
    "test_advanced_features.sh:Advanced Features:CORS, compression, SPA, caching"
    "test_network_ports.sh:Network & Ports:Port management and network functionality"
)

# Global test tracking
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0
SKIPPED_SUITES=0
OVERALL_TEST_COUNT=0
OVERALL_PASSED_COUNT=0
OVERALL_FAILED_COUNT=0

# Configuration options
RUN_PARALLEL=false
VERBOSE_OUTPUT=false
QUICK_MODE=false
SELECTIVE_SUITES=""
DRY_RUN=false
CONTINUE_ON_FAILURE=true
CLEAN_FIRST=true
FORCE_CLEAN=false

# Help function
show_help() {
    cat << EOF
Usage: $0 [OPTIONS] [SUITE_NAMES...]

Comprehensive test runner for msaada HTTP server functionality.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -q, --quick            Quick mode (skip some slower tests)
    -p, --parallel         Run test suites in parallel (experimental)
    -s, --selective SUITES List of specific suites to run (comma-separated)
    --dry-run              Show what would be executed without running tests
    --stop-on-failure      Stop execution on first suite failure
    --list-suites          List all available test suites
    --clean                Force cleanup before starting tests
    --no-clean             Skip automatic cleanup (not recommended)
    --force-clean          Force cleanup without confirmation prompts

SUITE_NAMES:
    http                   Core HTTP server functionality
    https                  HTTPS/SSL integration tests
    config                 Configuration file tests
    post                   POST request echo tests
    advanced               Advanced features (CORS, SPA, etc.)
    network                Network and port management tests

EXAMPLES:
    $0                                    # Run all test suites (with cleanup)
    $0 --verbose                          # Run all with verbose output
    $0 http post                          # Run only HTTP and POST tests
    $0 --selective http,https             # Run HTTP and HTTPS tests
    $0 --quick --parallel                 # Quick parallel execution
    $0 --dry-run --verbose               # Show execution plan
    $0 --force-clean --verbose           # Force cleanup with details
    $0 --no-clean                        # Skip cleanup (for debugging)

EXIT CODES:
    0    All tests passed
    1    Some tests failed
    2    Test setup/configuration error
    3    User requested stop

EOF
}

# List available test suites
list_suites() {
    print_header "Available Test Suites"
    
    local i=1
    for suite in "${TEST_SUITES[@]}"; do
        IFS=':' read -r script name description <<< "$suite"
        echo -e "${CYAN}$i.${NC} ${BOLD}$name${NC}"
        echo -e "   Script: ${YELLOW}$script${NC}"
        echo -e "   Description: $description"
        echo
        i=$((i + 1))
    done
    
    echo -e "Total: ${BOLD}${#TEST_SUITES[@]}${NC} test suites available"
}

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -v|--verbose)
                VERBOSE_OUTPUT=true
                shift
                ;;
            -q|--quick)
                QUICK_MODE=true
                shift
                ;;
            -p|--parallel)
                RUN_PARALLEL=true
                shift
                ;;
            -s|--selective)
                SELECTIVE_SUITES="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --stop-on-failure)
                CONTINUE_ON_FAILURE=false
                shift
                ;;
            --list-suites)
                list_suites
                exit 0
                ;;
            --clean)
                CLEAN_FIRST=true
                FORCE_CLEAN=true
                shift
                ;;
            --no-clean)
                CLEAN_FIRST=false
                shift
                ;;
            --force-clean)
                FORCE_CLEAN=true
                CLEAN_FIRST=true
                shift
                ;;
            http|https|config|post|advanced|network)
                if [[ -n "$SELECTIVE_SUITES" ]]; then
                    SELECTIVE_SUITES="$SELECTIVE_SUITES,$1"
                else
                    SELECTIVE_SUITES="$1"
                fi
                shift
                ;;
            *)
                echo "Unknown option: $1" >&2
                echo "Use --help for usage information" >&2
                exit 2
                ;;
        esac
    done
}

# Initialize test runner
init_test_runner() {
    print_header "$TEST_RUNNER_NAME"

    OVERALL_START_TIME=$(date +%s)

    # Handle cleanup if requested
    if [[ "$CLEAN_FIRST" == "true" ]]; then
        print_subheader "Pre-Test Cleanup"
        local cleanup_script="$SCRIPT_DIR/cleanup_tests.sh"

        if [[ -f "$cleanup_script" ]]; then
            local cleanup_args=""
            if [[ "$VERBOSE_OUTPUT" == "true" ]]; then
                cleanup_args="$cleanup_args --verbose"
            fi
            if [[ "$FORCE_CLEAN" == "true" ]]; then
                cleanup_args="$cleanup_args --force"
            fi
            if [[ "$DRY_RUN" == "true" ]]; then
                cleanup_args="$cleanup_args --dry-run"
            fi

            print_info "Running cleanup before tests..."
            if ! "$cleanup_script" $cleanup_args; then
                print_warning "Cleanup had issues but continuing with tests"
            else
                print_success "Pre-test cleanup completed"
            fi
        else
            print_warning "Cleanup script not found, using basic cleanup"
            # Fallback to basic cleanup
            source "$SCRIPT_DIR/test_utils.sh"
            cleanup_test_environment "$VERBOSE_OUTPUT"
        fi
        echo
    fi

    echo -e "Test Configuration:"
    echo -e "  Working Directory: ${CYAN}$SCRIPT_DIR${NC}"
    echo -e "  Parallel Execution: ${CYAN}$RUN_PARALLEL${NC}"
    echo -e "  Verbose Output: ${CYAN}$VERBOSE_OUTPUT${NC}"
    echo -e "  Quick Mode: ${CYAN}$QUICK_MODE${NC}"
    echo -e "  Continue on Failure: ${CYAN}$CONTINUE_ON_FAILURE${NC}"
    echo -e "  Cleanup First: ${CYAN}$CLEAN_FIRST${NC}"
    
    if [[ -n "$SELECTIVE_SUITES" ]]; then
        echo -e "  Selected Suites: ${YELLOW}$SELECTIVE_SUITES${NC}"
    else
        echo -e "  Running: ${GREEN}All Test Suites${NC}"
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "  ${YELLOW}DRY RUN MODE - No tests will be executed${NC}"
    fi
    
    echo
    
    # Ensure msaada binary exists
    if ! ensure_msaada_binary; then
        print_error "Failed to ensure msaada binary exists"
        exit 2
    fi
    
    # Create comprehensive temp directory
    mkdir -p "$TEMP_DIR/comprehensive_tests"
}

# Check if suite should be run
should_run_suite() {
    local script="$1"
    local name="$2"
    
    # If no selective suites specified, run all
    if [[ -z "$SELECTIVE_SUITES" ]]; then
        return 0
    fi
    
    # Check if script name or suite name matches selective criteria
    local suite_key=""
    case "$script" in
        *http_server*) suite_key="http" ;;
        *https_ssl*) suite_key="https" ;;
        *config*) suite_key="config" ;;
        *post*) suite_key="post" ;;
        *advanced*) suite_key="advanced" ;;
        *network*) suite_key="network" ;;
    esac
    
    if [[ "$SELECTIVE_SUITES" =~ $suite_key ]]; then
        return 0
    fi
    
    return 1
}

# Execute a single test suite
execute_test_suite() {
    local script="$1"
    local name="$2"
    local description="$3"
    
    TOTAL_SUITES=$((TOTAL_SUITES + 1))
    
    local script_path="$SCRIPT_DIR/$script"
    
    # Check if test script exists
    if [[ ! -f "$script_path" ]]; then
        print_error "Test script not found: $script_path"
        FAILED_SUITES=$((FAILED_SUITES + 1))
        return 1
    fi
    
    # Check if script is executable
    if [[ ! -x "$script_path" ]]; then
        print_warning "Making test script executable: $script"
        chmod +x "$script_path"
    fi
    
    print_header "Running Test Suite: $name"
    echo -e "Description: $description"
    echo -e "Script: ${CYAN}$script${NC}"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_info "DRY RUN: Would execute $script_path"
        PASSED_SUITES=$((PASSED_SUITES + 1))
        return 0
    fi
    
    # Set up logging
    local suite_log="$TEMP_DIR/comprehensive_tests/${script%.sh}.log"
    local start_time end_time duration
    start_time=$(date +%s)
    
    # Execute test suite
    if [[ "$VERBOSE_OUTPUT" == "true" ]]; then
        # Show output in real-time
        if "$script_path"; then
            local exit_code=0
        else
            local exit_code=1
        fi
    else
        # Capture output to log file
        if "$script_path" > "$suite_log" 2>&1; then
            local exit_code=0
        else
            local exit_code=1
        fi
    fi
    
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    
    # Report results
    if [[ $exit_code -eq 0 ]]; then
        print_success "Test Suite '$name' PASSED (${duration}s)"
        PASSED_SUITES=$((PASSED_SUITES + 1))
        
        if [[ "$VERBOSE_OUTPUT" == "false" && -f "$suite_log" ]]; then
            # Extract summary info from log
            local suite_tests suite_passed
            suite_tests=$(grep -o "Total Tests: [0-9]*" "$suite_log" | cut -d' ' -f3)
            suite_passed=$(grep -o "Passed: [0-9]*" "$suite_log" | cut -d' ' -f2)
            
            if [[ -n "$suite_tests" && -n "$suite_passed" ]]; then
                print_info "Suite Results: $suite_passed/$suite_tests tests passed"
                OVERALL_TEST_COUNT=$((OVERALL_TEST_COUNT + ${suite_tests:-0}))
                OVERALL_PASSED_COUNT=$((OVERALL_PASSED_COUNT + ${suite_passed:-0}))
            fi
        fi
    else
        print_failure "Test Suite '$name' FAILED (${duration}s)"
        FAILED_SUITES=$((FAILED_SUITES + 1))
        
        if [[ "$VERBOSE_OUTPUT" == "false" && -f "$suite_log" ]]; then
            print_error "Last 10 lines of failed suite log:"
            tail -10 "$suite_log" | sed 's/^/  /' >&2
        fi
        
        if [[ "$CONTINUE_ON_FAILURE" == "false" ]]; then
            print_error "Stopping execution due to suite failure (--stop-on-failure)"
            return 3
        fi
    fi
    
    echo
    return $exit_code
}

# Execute all test suites sequentially
run_sequential_tests() {
    print_subheader "Running Test Suites Sequentially"
    
    for suite in "${TEST_SUITES[@]}"; do
        IFS=':' read -r script name description <<< "$suite"
        
        if should_run_suite "$script" "$name"; then
            if ! execute_test_suite "$script" "$name" "$description"; then
                if [[ $? -eq 3 ]]; then # Stop on failure requested
                    return 3
                fi
            fi
        else
            print_skip "Test Suite '$name' (not selected)"
            SKIPPED_SUITES=$((SKIPPED_SUITES + 1))
        fi
    done
}

# Execute all test suites in parallel (experimental)
run_parallel_tests() {
    print_subheader "Running Test Suites in Parallel (Experimental)"
    print_warning "Parallel execution may produce interleaved output"
    
    local pids=()
    local suite_results=()
    
    # Start all test suites in background
    for suite in "${TEST_SUITES[@]}"; do
        IFS=':' read -r script name description <<< "$suite"
        
        if should_run_suite "$script" "$name"; then
            local script_path="$SCRIPT_DIR/$script"
            local suite_log="$TEMP_DIR/comprehensive_tests/parallel_${script%.sh}.log"
            
            if [[ "$DRY_RUN" == "true" ]]; then
                print_info "DRY RUN: Would execute $script_path in parallel"
                continue
            fi
            
            print_info "Starting '$name' in background..."
            
            # Execute in background
            (
                if "$script_path" > "$suite_log" 2>&1; then
                    echo "$name:PASSED:$suite_log"
                else
                    echo "$name:FAILED:$suite_log"
                fi
            ) &
            
            pids+=($!)
            suite_results+=("$name:RUNNING:$suite_log")
        else
            print_skip "Test Suite '$name' (not selected)"
            SKIPPED_SUITES=$((SKIPPED_SUITES + 1))
        fi
    done
    
    if [[ "$DRY_RUN" == "true" ]]; then
        PASSED_SUITES=$((PASSED_SUITES + ${#pids[@]}))
        return 0
    fi
    
    # Wait for all to complete and collect results
    for pid in "${pids[@]}"; do
        wait "$pid"
        local exit_code=$?
        
        # Results are handled by the background processes
        TOTAL_SUITES=$((TOTAL_SUITES + 1))
        if [[ $exit_code -eq 0 ]]; then
            PASSED_SUITES=$((PASSED_SUITES + 1))
        else
            FAILED_SUITES=$((FAILED_SUITES + 1))
        fi
    done
    
    # Report parallel results
    print_subheader "Parallel Execution Results"
    for result_file in "$TEMP_DIR/comprehensive_tests"/parallel_*.log; do
        if [[ -f "$result_file" ]]; then
            local suite_name
            suite_name=$(basename "$result_file" .log | sed 's/parallel_//')
            
            if grep -q "ALL TESTS PASSED" "$result_file"; then
                print_success "Suite '$suite_name' PASSED"
            else
                print_failure "Suite '$suite_name' FAILED"
                if [[ "$VERBOSE_OUTPUT" == "true" ]]; then
                    tail -5 "$result_file" | sed 's/^/  /'
                fi
            fi
        fi
    done
}

# Generate final comprehensive report
generate_final_report() {
    OVERALL_END_TIME=$(date +%s)
    local total_duration=$((OVERALL_END_TIME - OVERALL_START_TIME))
    
    print_header "Comprehensive Test Results Summary"
    
    echo -e "Execution Summary:"
    echo -e "  Total Duration: ${CYAN}${total_duration}s${NC}"
    echo -e "  Execution Mode: ${CYAN}$(if [[ "$RUN_PARALLEL" == "true" ]]; then echo "Parallel"; else echo "Sequential"; fi)${NC}"
    
    echo -e "\nTest Suite Summary:"
    echo -e "  Total Suites: ${CYAN}$TOTAL_SUITES${NC}"
    echo -e "  Passed Suites: ${GREEN}$PASSED_SUITES${NC}"
    echo -e "  Failed Suites: ${RED}$FAILED_SUITES${NC}"
    echo -e "  Skipped Suites: ${YELLOW}$SKIPPED_SUITES${NC}"
    
    if [[ $OVERALL_TEST_COUNT -gt 0 ]]; then
        echo -e "\nOverall Test Summary:"
        echo -e "  Total Tests: ${CYAN}$OVERALL_TEST_COUNT${NC}"
        echo -e "  Passed Tests: ${GREEN}$OVERALL_PASSED_COUNT${NC}"
        echo -e "  Failed Tests: ${RED}$((OVERALL_TEST_COUNT - OVERALL_PASSED_COUNT))${NC}"
    fi
    
    # Calculate success rate
    if [[ $TOTAL_SUITES -gt 0 ]]; then
        local success_rate=$(( (PASSED_SUITES * 100) / TOTAL_SUITES ))
        echo -e "\nSuccess Rate: ${BOLD}$success_rate%${NC}"
    fi
    
    # Final status
    if [[ $FAILED_SUITES -eq 0 ]]; then
        echo -e "\n${GREEN}${BOLD}🎉 ALL TEST SUITES PASSED! 🎉${NC}"
        echo -e "${GREEN}The msaada HTTP server is functioning correctly across all tested features.${NC}"
        return 0
    else
        echo -e "\n${RED}${BOLD}❌ SOME TEST SUITES FAILED ❌${NC}"
        echo -e "${RED}Please review the failed suites and fix the issues before release.${NC}"
        
        # Show which suites failed
        echo -e "\nFailed test suite logs are available in:"
        echo -e "  ${CYAN}$TEMP_DIR/comprehensive_tests/${NC}"
        
        return 1
    fi
}

# Cleanup function
cleanup_test_runner() {
    print_info "Cleaning up test runner resources..."
    
    # Kill any remaining background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    # Standard cleanup
    cleanup_test_utils
}

# Main execution function
main() {
    # Set up trap for cleanup
    trap cleanup_test_runner EXIT
    
    # Parse command line arguments
    parse_arguments "$@"
    
    # Initialize test runner
    init_test_runner
    
    # Execute tests based on configuration
    local execution_result
    if [[ "$RUN_PARALLEL" == "true" ]]; then
        run_parallel_tests
        execution_result=$?
    else
        run_sequential_tests
        execution_result=$?
    fi
    
    # Handle early exit conditions
    if [[ $execution_result -eq 3 ]]; then
        print_warning "Test execution stopped early"
        exit 3
    fi
    
    # Generate and show final report
    if generate_final_report; then
        exit 0
    else
        exit 1
    fi
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi