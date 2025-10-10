#!/bin/bash

# RIDI Credential Helper - Optimized & Efficient
# Automatically extracts RIDI credentials with maximum automation

set -e

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

# Constants
readonly API_URL="https://account.ridibooks.com/api/user-devices/app"
readonly LOGIN_URL="https://ridibooks.com/account/login"
readonly CONFIG_FILE="$HOME/.ridiculous.toml"
readonly TEMP_JSON="/tmp/ridi_devices_$$.json"

# Auto-fix permissions on first run
[[ ! -x "$0" ]] && chmod +x "$0" 2>/dev/null && exec "$0" "$@"

# Utility functions
print_header() {
    clear
    echo -e "${CYAN}${BOLD}╔═══════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}${BOLD}║   RIDI Credential Helper - Smart Extraction       ║${NC}"
    echo -e "${CYAN}${BOLD}╚═══════════════════════════════════════════════════╝${NC}"
    echo
}

print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_step() { echo -e "${CYAN}${BOLD}➤ $1${NC}"; }

# Cleanup on exit
cleanup() {
    rm -f "$TEMP_JSON" "/tmp/ridi_cookies_$$"* 2>/dev/null
}
trap cleanup EXIT

# Validation functions
validate_device_id() {
    [[ "$1" =~ ^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}$ ]] || \
    [[ "$1" =~ ^[a-fA-F0-9]{32}$ ]] || \
    [[ "$1" =~ ^[a-zA-Z0-9-]{20,50}$ ]]
}

validate_user_idx() {
    [[ "$1" =~ ^[0-9]{6,15}$ ]]
}

validate_json() {
    local file="$1"
    [[ -f "$file" ]] && [[ -s "$file" ]] && grep -q "device_id" "$file" 2>/dev/null
}

# Browser detection
detect_os() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        Linux*) echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

# Smart URL opener
open_url() {
    local url="$1"
    local os=$(detect_os)
    
    case "$os" in
        macos) open "$url" 2>/dev/null ;;
        linux) xdg-open "$url" 2>/dev/null || sensible-browser "$url" 2>/dev/null ;;
        windows) start "$url" 2>/dev/null ;;
    esac
}

# Check if curl has valid session
check_existing_session() {
    print_step "Checking for existing RIDI session..."
    
    if ! command -v curl &> /dev/null; then
        print_warning "curl not found, skipping session check"
        return 1
    fi
    
    local response=$(curl -sL -w "\n%{http_code}" "$API_URL" 2>/dev/null | tail -1)
    
    if [[ "$response" == "200" ]]; then
        curl -sL "$API_URL" > "$TEMP_JSON" 2>/dev/null
        if validate_json "$TEMP_JSON"; then
            print_success "Found existing session!"
            return 0
        fi
    fi
    
    return 1
}

# Try to fetch with browser cookies (cross-platform)
try_browser_cookies() {
    print_step "Attempting to use browser cookies..."
    
    local os=$(detect_os)
    local found=false
    
    case "$os" in
        macos)
            # Try Chrome
            if try_chrome_cookies_macos; then
                found=true
            # Try Safari
            elif try_safari_cookies_macos; then
                found=true
            fi
            ;;
        linux)
            if try_chrome_cookies_linux; then
                found=true
            elif try_firefox_cookies_linux; then
                found=true
            fi
            ;;
    esac
    
    [[ "$found" == true ]]
}

try_chrome_cookies_macos() {
    local chrome_cookies="$HOME/Library/Application Support/Google/Chrome/Default/Cookies"
    [[ ! -f "$chrome_cookies" ]] && return 1
    
    command -v sqlite3 &> /dev/null || return 1
    
    local cookie_file="/tmp/ridi_cookies_$$"
    
    # Extract RIDI-related cookies
    sqlite3 "$chrome_cookies" <<EOF 2>/dev/null | grep -v "^$" > "$cookie_file" || return 1
.mode tabs
.headers off
SELECT name || '=' || value
FROM cookies 
WHERE (host_key LIKE '%ridibooks%' OR host_key LIKE '%ridi%')
AND expires_utc > $(date +%s)000000;
EOF
    
    [[ -s "$cookie_file" ]] || return 1
    
    # Try to fetch with cookies
    if curl -sL -b "$cookie_file" "$API_URL" -o "$TEMP_JSON" 2>/dev/null; then
        if validate_json "$TEMP_JSON"; then
            print_success "Retrieved data using Chrome cookies"
            return 0
        fi
    fi
    
    return 1
}

try_chrome_cookies_linux() {
    local chrome_paths=(
        "$HOME/.config/google-chrome/Default/Cookies"
        "$HOME/.config/chromium/Default/Cookies"
        "$HOME/snap/chromium/common/chromium/Default/Cookies"
    )
    
    for chrome_cookies in "${chrome_paths[@]}"; do
        [[ -f "$chrome_cookies" ]] || continue
        command -v sqlite3 &> /dev/null || continue
        
        local cookie_file="/tmp/ridi_cookies_$$"
        
        sqlite3 "$chrome_cookies" <<EOF 2>/dev/null | grep -v "^$" > "$cookie_file" || continue
.mode tabs
.headers off
SELECT name || '=' || value
FROM cookies 
WHERE (host_key LIKE '%ridibooks%' OR host_key LIKE '%ridi%')
AND expires_utc > $(date +%s)000000;
EOF
        
        [[ -s "$cookie_file" ]] || continue
        
        if curl -sL -b "$cookie_file" "$API_URL" -o "$TEMP_JSON" 2>/dev/null; then
            if validate_json "$TEMP_JSON"; then
                print_success "Retrieved data using Chrome cookies"
                return 0
            fi
        fi
    done
    
    return 1
}

try_firefox_cookies_linux() {
    local firefox_dir="$HOME/.mozilla/firefox"
    [[ ! -d "$firefox_dir" ]] && return 1
    
    local profile=$(find "$firefox_dir" -maxdepth 1 -name "*.default*" -type d | head -1)
    [[ -z "$profile" ]] && return 1
    
    local cookies_db="$profile/cookies.sqlite"
    [[ ! -f "$cookies_db" ]] && return 1
    
    command -v sqlite3 &> /dev/null || return 1
    
    local cookie_file="/tmp/ridi_cookies_$$"
    
    sqlite3 "$cookies_db" <<EOF 2>/dev/null | grep -v "^$" > "$cookie_file" || return 1
.mode tabs
.headers off
SELECT name || '=' || value
FROM moz_cookies 
WHERE (host LIKE '%ridibooks%' OR host LIKE '%ridi%')
AND expiry > $(date +%s);
EOF
    
    [[ -s "$cookie_file" ]] || return 1
    
    if curl -sL -b "$cookie_file" "$API_URL" -o "$TEMP_JSON" 2>/dev/null; then
        if validate_json "$TEMP_JSON"; then
            print_success "Retrieved data using Firefox cookies"
            return 0
        fi
    fi
    
    return 1
}

try_safari_cookies_macos() {
    # Safari cookies are in binary format and harder to extract
    # For now, return false
    return 1
}

# Parse JSON and extract credentials
parse_and_save() {
    local json_file="$1"
    
    print_step "Parsing device information..."
    
    # Try jq first (best method)
    if command -v jq &> /dev/null; then
        parse_with_jq "$json_file"
        return $?
    fi
    
    # Fallback to python
    if command -v python3 &> /dev/null; then
        parse_with_python "$json_file"
        return $?
    fi
    
    # Last resort: regex
    parse_with_regex "$json_file"
}

parse_with_jq() {
    local json_file="$1"
    local device_count=0
    local best_device_id=""
    local best_user_idx=""
    local best_name=""
    
    echo
    echo -e "${BLUE}${BOLD}Available Devices:${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    while IFS=$'\t' read -r device_id user_idx device_name os; do
        if [[ -n "$device_id" && -n "$user_idx" ]]; then
            ((device_count++))
            
            echo -e "${CYAN}Device $device_count:${NC} ${BOLD}${device_name:-Unknown}${NC} [$os]"
            echo "  Device ID : $device_id"
            echo "  User Index: $user_idx"
            
            local valid=true
            if validate_device_id "$device_id"; then
                echo -e "  ${GREEN}✓ Device ID: Valid${NC}"
            else
                echo -e "  ${YELLOW}⚠ Device ID: Unusual format${NC}"
                valid=false
            fi
            
            if validate_user_idx "$user_idx"; then
                echo -e "  ${GREEN}✓ User Index: Valid${NC}"
            else
                echo -e "  ${YELLOW}⚠ User Index: Unusual format${NC}"
                valid=false
            fi
            
            # Use first valid device
            if [[ -z "$best_device_id" && "$valid" == true ]]; then
                best_device_id="$device_id"
                best_user_idx="$user_idx"
                best_name="$device_name"
                echo -e "  ${GREEN}${BOLD}⭐ SELECTED${NC}"
            fi
            
            echo
        fi
    done < <(jq -r '.result[]? | select(.device_id and .user_idx) | [.device_id, .user_idx, (.device_name // "Unknown"), (.os // "Unknown")] | @tsv' "$json_file" 2>/dev/null)
    
    if [[ $device_count -eq 0 ]]; then
        print_error "No valid devices found in response"
        return 1
    fi
    
    if [[ -n "$best_device_id" ]]; then
        save_config "$best_device_id" "$best_user_idx" "$best_name"
        return 0
    fi
    
    return 1
}

parse_with_python() {
    local json_file="$1"
    
    local result=$(python3 <<EOF 2>/dev/null
import json
import sys

try:
    with open('$json_file', 'r') as f:
        data = json.load(f)
    
    if 'result' in data and len(data['result']) > 0:
        device = data['result'][0]
        if 'device_id' in device and 'user_idx' in device:
            print(f"{device['device_id']}\t{device['user_idx']}\t{device.get('device_name', 'Unknown')}")
            sys.exit(0)
except:
    pass

sys.exit(1)
EOF
)
    
    if [[ -n "$result" ]]; then
        IFS=$'\t' read -r device_id user_idx device_name <<< "$result"
        print_success "Found device: $device_name"
        save_config "$device_id" "$user_idx" "$device_name"
        return 0
    fi
    
    return 1
}

parse_with_regex() {
    local json_file="$1"
    local content=$(cat "$json_file")
    
    # Extract first device_id and user_idx
    local device_id=$(echo "$content" | grep -oE '"device_id"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | grep -oE '[a-fA-F0-9-]{20,50}')
    local user_idx=$(echo "$content" | grep -oE '"user_idx"[[:space:]]*:[[:space:]]*"?[0-9]+"?' | head -1 | grep -oE '[0-9]{6,15}')
    
    if [[ -n "$device_id" && -n "$user_idx" ]]; then
        print_success "Extracted credentials using regex"
        save_config "$device_id" "$user_idx" "Auto-detected"
        return 0
    fi
    
    return 1
}

# Save configuration
save_config() {
    local device_id="$1"
    local user_idx="$2"
    local device_name="$3"
    
    print_step "Saving configuration..."
    
    # Backup existing config
    if [[ -f "$CONFIG_FILE" ]]; then
        local backup="$CONFIG_FILE.backup.$(date +%s)"
        cp "$CONFIG_FILE" "$backup" 2>/dev/null
        print_info "Backed up existing config to: $backup"
    fi
    
    # Create new config
    cat > "$CONFIG_FILE" << EOF
# RIDI Credentials - Auto-generated
# Device: $device_name
# Generated: $(date '+%Y-%m-%d %H:%M:%S')

device_id = "$device_id"
user_idx = "$user_idx"
verbose = false
organize_output = false
EOF

    if [[ -f "$CONFIG_FILE" ]]; then
        print_success "Configuration saved to: $CONFIG_FILE"
        echo
        echo -e "${GREEN}${BOLD}✨ Setup Complete!${NC}"
        echo
        echo "You can now run your decryption tool:"
        echo -e "  ${CYAN}cargo run${NC}"
        echo -e "  ${CYAN}cargo run -- --verbose${NC} (for detailed output)"
        echo -e "  ${CYAN}cargo run -- --batch-mode${NC} (for batch processing)"
        return 0
    else
        print_error "Failed to save configuration"
        return 1
    fi
}

# Manual entry fallback
manual_entry() {
    echo
    print_step "Manual Entry Mode"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo
    print_info "Opening device API page in your browser..."
    open_url "$API_URL"
    echo
    echo "In your browser, you'll see JSON data containing:"
    echo
    echo -e "${YELLOW}Example format:${NC}"
    echo '  "device_id": "12345678-abcd-1234-5678-123456789abc"'
    echo '  "user_idx": "12345678"'
    echo
    
    # Wait a moment for browser to open
    sleep 2
    
    # Get device_id
    local device_id=""
    while true; do
        read -p "Enter device_id: " device_id
        device_id=$(echo "$device_id" | tr -d '"' | tr -d ' ' | tr -d ',' | tr -d ':' | sed 's/device_id//g')
        
        if [[ -n "$device_id" ]]; then
            if validate_device_id "$device_id"; then
                print_success "Device ID validated"
            else
                print_warning "Unusual format, but continuing..."
            fi
            break
        fi
        print_error "Please enter a valid device ID"
    done
    
    # Get user_idx
    local user_idx=""
    while true; do
        read -p "Enter user_idx: " user_idx
        user_idx=$(echo "$user_idx" | tr -d '"' | tr -d ' ' | tr -d ',' | tr -d ':' | sed 's/user_idx//g')
        
        if [[ -n "$user_idx" ]]; then
            if validate_user_idx "$user_idx"; then
                print_success "User index validated"
            else
                print_warning "Unusual format, but continuing..."
            fi
            break
        fi
        print_error "Please enter a valid user index"
    done
    
    save_config "$device_id" "$user_idx" "Manual Entry"
}

# Main execution
main() {
    print_header
    
    # Check if already configured
    if [[ -f "$CONFIG_FILE" ]] && grep -q "device_id" "$CONFIG_FILE" 2>/dev/null; then
        print_info "Configuration file already exists: $CONFIG_FILE"
        read -p "Do you want to reconfigure? (y/N): " reconfigure
        if [[ ! "$reconfigure" =~ ^[Yy] ]]; then
            print_success "Using existing configuration"
            exit 0
        fi
        echo
    fi
    
    # Strategy 1: Check for existing session
    if check_existing_session && parse_and_save "$TEMP_JSON"; then
        return 0
    fi
    
    # Strategy 2: Try browser cookies
    if try_browser_cookies && parse_and_save "$TEMP_JSON"; then
        return 0
    fi
    
    # Strategy 3: Guide user to log in
    echo
    print_info "Automatic methods didn't work. Let's log you in..."
    echo
    print_info "Opening RIDI login page..."
    open_url "$LOGIN_URL"
    echo
    echo "Please log in to your RIDI account in the browser."
    read -p "Press Enter after you've logged in..."
    echo
    
    # Try session check again after login
    sleep 2
    if check_existing_session && parse_and_save "$TEMP_JSON"; then
        return 0
    fi
    
    # Strategy 4: Try cookies again after login
    if try_browser_cookies && parse_and_save "$TEMP_JSON"; then
        return 0
    fi
    
    # Final fallback: Manual entry
    print_warning "Automatic extraction failed. Falling back to manual entry."
    manual_entry
}

# Run
main "$@"