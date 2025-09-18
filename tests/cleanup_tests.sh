#!/bin/bash
# cleanup_tests.sh - Test cleanup utility for msaada test suite

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR="$SCRIPT_DIR/.tmp"

# Color constants
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
VERBOSE=false
PRESERVE_LOGS=false
FORCE_CLEANUP=false
DRY_RUN=false
INTERACTIVE=true

# Usage information
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Test cleanup utility for msaada test suite. Removes temporary files, logs, and
test artifacts from previous test runs to ensure a clean testing environment.

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output showing all operations
    -f, --force             Force cleanup without confirmation prompts
    -p, --preserve-logs     Preserve recent log files (last 24 hours)
    -n, --dry-run          Show what would be cleaned without removing files
    -q, --quiet            Suppress all output except errors
    --no-interactive       Don't prompt for confirmations (same as --force)

CLEANUP TARGETS:
    • Test temporary directory (.tmp/)
    • Server log files (server_*.log)
    • Test artifacts (temp files, certificates, etc.)
    • Lingering test processes
    • Lock files and PIDs

EXAMPLES:
    $0                                    # Interactive cleanup
    $0 --force --verbose                  # Force cleanup with details
    $0 --preserve-logs --dry-run         # Preview cleanup keeping logs
    $0 --quiet --force                   # Silent forced cleanup

EXIT CODES:
    0    Cleanup completed successfully
    1    Cleanup failed or was cancelled
    2    Invalid arguments or configuration error

EOF
}

# Print functions
print_header() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "\n${BLUE}${BOLD}=== $1 ===${NC}"
    fi
}

print_info() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${CYAN}ℹ INFO:${NC} $1"
    fi
}

print_success() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${GREEN}✓ SUCCESS:${NC} $1"
    fi
}

print_warning() {
    echo -e "${YELLOW}⚠ WARNING:${NC} $1" >&2
}

print_error() {
    echo -e "${RED}❌ ERROR:${NC} $1" >&2
}

print_dry_run() {
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN]${NC} Would: $1"
    fi
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
                VERBOSE=true
                shift
                ;;
            -f|--force)
                FORCE_CLEANUP=true
                INTERACTIVE=false
                shift
                ;;
            -p|--preserve-logs)
                PRESERVE_LOGS=true
                shift
                ;;
            -n|--dry-run)
                DRY_RUN=true
                VERBOSE=true  # Enable verbose for dry run
                shift
                ;;
            -q|--quiet)
                VERBOSE=false
                shift
                ;;
            --no-interactive)
                INTERACTIVE=false
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                echo "Use --help for usage information."
                exit 2
                ;;
        esac
    done
}

# Check if directory exists and get size
get_directory_info() {
    local dir="$1"

    if [[ ! -d "$dir" ]]; then
        echo "0 0"
        return
    fi

    # Count files and get size
    local file_count=$(find "$dir" -type f 2>/dev/null | wc -l | tr -d ' ')
    local size_kb=$(du -sk "$dir" 2>/dev/null | cut -f1)

    echo "$file_count $size_kb"
}

# Format file size for display
format_size() {
    local size_kb=$1

    if [[ $size_kb -lt 1024 ]]; then
        echo "${size_kb} KB"
    elif [[ $size_kb -lt 1048576 ]]; then
        echo "$((size_kb / 1024)) MB"
    else
        echo "$((size_kb / 1048576)) GB"
    fi
}

# Stop any lingering test processes
cleanup_processes() {
    print_header "Cleaning up test processes"

    # Look for msaada processes that might be left running
    local pids=$(pgrep -f "msaada.*--port" 2>/dev/null || true)

    if [[ -n "$pids" ]]; then
        print_warning "Found running msaada test processes: $pids"

        if [[ "$DRY_RUN" == "true" ]]; then
            print_dry_run "Kill processes: $pids"
        else
            if [[ "$INTERACTIVE" == "true" ]] && [[ "$FORCE_CLEANUP" == "false" ]]; then
                echo -n "Kill these processes? (y/N): "
                read -r response
                if [[ ! "$response" =~ ^[Yy]$ ]]; then
                    print_info "Skipping process cleanup"
                    return 0
                fi
            fi

            echo "$pids" | xargs kill 2>/dev/null || true
            sleep 1

            # Force kill if still running
            local remaining_pids=$(pgrep -f "msaada.*--port" 2>/dev/null || true)
            if [[ -n "$remaining_pids" ]]; then
                print_warning "Force killing remaining processes: $remaining_pids"
                echo "$remaining_pids" | xargs kill -9 2>/dev/null || true
            fi

            print_success "Cleaned up test processes"
        fi
    else
        print_info "No test processes found"
    fi
}

# Clean up temporary files
cleanup_temp_files() {
    print_header "Cleaning up temporary files"

    if [[ ! -d "$TEMP_DIR" ]]; then
        print_info "No temporary directory found"
        return 0
    fi

    # Get directory information
    local info=($(get_directory_info "$TEMP_DIR"))
    local file_count=${info[0]}
    local size_kb=${info[1]}

    if [[ $file_count -eq 0 ]]; then
        print_info "Temporary directory is already empty"
        return 0
    fi

    local size_display=$(format_size $size_kb)
    print_info "Found $file_count files ($size_display) in temporary directory"

    if [[ "$DRY_RUN" == "true" ]]; then
        print_dry_run "Remove temporary directory: $TEMP_DIR"
        if [[ "$VERBOSE" == "true" ]]; then
            find "$TEMP_DIR" -type f | head -10
            if [[ $file_count -gt 10 ]]; then
                echo "... and $((file_count - 10)) more files"
            fi
        fi
        return 0
    fi

    # Handle log preservation
    if [[ "$PRESERVE_LOGS" == "true" ]]; then
        print_info "Preserving recent log files (last 24 hours)"

        # Create backup directory
        local backup_dir="$TEMP_DIR.backup"
        mkdir -p "$backup_dir"

        # Move recent log files to backup
        find "$TEMP_DIR" -name "*.log" -type f -mtime -1 -exec mv {} "$backup_dir/" \; 2>/dev/null || true

        # Remove temp directory
        rm -rf "$TEMP_DIR" 2>/dev/null || {
            print_error "Failed to remove temporary directory"
            return 1
        }

        # Recreate temp directory and restore logs
        mkdir -p "$TEMP_DIR"
        if [[ -d "$backup_dir" ]] && [[ -n "$(ls -A "$backup_dir" 2>/dev/null)" ]]; then
            mv "$backup_dir"/* "$TEMP_DIR/" 2>/dev/null || true
            local preserved_count=$(ls -1 "$TEMP_DIR" 2>/dev/null | wc -l | tr -d ' ')
            print_success "Preserved $preserved_count recent log files"
        fi

        # Clean up backup directory
        rm -rf "$backup_dir" 2>/dev/null || true
    else
        # Full cleanup
        if [[ "$INTERACTIVE" == "true" ]] && [[ "$FORCE_CLEANUP" == "false" ]]; then
            echo -n "Remove $file_count files ($size_display) from temporary directory? (y/N): "
            read -r response
            if [[ ! "$response" =~ ^[Yy]$ ]]; then
                print_info "Skipping temporary file cleanup"
                return 0
            fi
        fi

        rm -rf "$TEMP_DIR" 2>/dev/null || {
            print_error "Failed to remove temporary directory"
            return 1
        }

        # Recreate empty temp directory
        mkdir -p "$TEMP_DIR"
    fi

    print_success "Cleaned up temporary files"
}

# Clean up lock files and PIDs
cleanup_locks() {
    print_header "Cleaning up lock files"

    local locks_found=0

    # Look for common lock file patterns
    for pattern in "*.pid" "*.lock" ".lock*"; do
        while IFS= read -r -d '' lock_file; do
            locks_found=$((locks_found + 1))
            print_info "Found lock file: $lock_file"

            if [[ "$DRY_RUN" == "true" ]]; then
                print_dry_run "Remove lock file: $lock_file"
            else
                rm -f "$lock_file" 2>/dev/null || {
                    print_warning "Could not remove lock file: $lock_file"
                }
            fi
        done < <(find "$SCRIPT_DIR" -maxdepth 2 -name "$pattern" -type f -print0 2>/dev/null || true)
    done

    if [[ $locks_found -eq 0 ]]; then
        print_info "No lock files found"
    else
        if [[ "$DRY_RUN" == "false" ]]; then
            print_success "Cleaned up $locks_found lock files"
        fi
    fi
}

# Verify cleanup results
verify_cleanup() {
    if [[ "$DRY_RUN" == "true" ]]; then
        return 0
    fi

    print_header "Verifying cleanup"

    local verification_passed=true

    # Check temp directory
    if [[ -d "$TEMP_DIR" ]]; then
        local info=($(get_directory_info "$TEMP_DIR"))
        local remaining_files=${info[0]}

        if [[ "$PRESERVE_LOGS" == "true" ]]; then
            # Count only non-log files
            remaining_files=$(find "$TEMP_DIR" -type f ! -name "*.log" 2>/dev/null | wc -l | tr -d ' ')
        fi

        if [[ $remaining_files -gt 0 ]]; then
            print_warning "$remaining_files files remain in temporary directory"
            verification_passed=false
        else
            print_success "Temporary directory is clean"
        fi
    else
        print_error "Temporary directory was not recreated"
        verification_passed=false
    fi

    # Check for lingering processes
    local remaining_pids=$(pgrep -f "msaada.*--port" 2>/dev/null || true)
    if [[ -n "$remaining_pids" ]]; then
        print_warning "Test processes still running: $remaining_pids"
        verification_passed=false
    else
        print_success "No test processes running"
    fi

    if [[ "$verification_passed" == "true" ]]; then
        print_success "Cleanup verification passed"
        return 0
    else
        print_error "Cleanup verification failed"
        return 1
    fi
}

# Main cleanup function
main() {
    # Parse arguments
    parse_arguments "$@"

    # Show header
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}${BOLD}=== MSAADA TEST CLEANUP (DRY RUN) ===${NC}\n"
    else
        echo -e "${BLUE}${BOLD}=== MSAADA TEST CLEANUP ===${NC}\n"
    fi

    # Show what will be cleaned
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "Cleanup directory: ${CYAN}$SCRIPT_DIR${NC}"
        echo -e "Temporary directory: ${CYAN}$TEMP_DIR${NC}"
        echo -e "Preserve logs: ${CYAN}$PRESERVE_LOGS${NC}"
        echo -e "Dry run mode: ${CYAN}$DRY_RUN${NC}"
        echo
    fi

    # Perform cleanup steps
    local cleanup_failed=false

    cleanup_processes || cleanup_failed=true
    cleanup_temp_files || cleanup_failed=true
    cleanup_locks || cleanup_failed=true

    # Verify results (skip for dry run)
    if [[ "$DRY_RUN" == "false" ]]; then
        verify_cleanup || cleanup_failed=true
    fi

    # Final status
    echo
    if [[ "$cleanup_failed" == "true" ]]; then
        print_error "Cleanup completed with errors"
        return 1
    elif [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}${BOLD}✓ DRY RUN COMPLETED${NC}"
        echo "Run without --dry-run to perform actual cleanup"
        return 0
    else
        echo -e "${GREEN}${BOLD}✅ CLEANUP COMPLETED SUCCESSFULLY${NC}"
        return 0
    fi
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi