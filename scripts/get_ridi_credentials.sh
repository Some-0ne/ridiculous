#!/bin/bash

# Simplified RIDI Device ID Helper
# Automates credential extraction with minimal user interaction

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Auto-fix permissions
if [[ ! -x "$0" ]]; then
    chmod +x "$0" 2>/dev/null && exec "$0" "$@" || echo "Warning: Could not make script executable"
fi

print_header() {
    clear
    echo -e "${CYAN}${BOLD}RIDI Credential Helper (Simplified)${NC}"
    echo "======================================"
    echo
}

print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }

# Validate formats
validate_device_id() {
    [[ "$1" =~ ^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}$ ]] || \
    [[ "$1" =~ ^[a-fA-F0-9]{32}$ ]] || \
    [[ "$1" =~ ^[a-zA-Z0-9-]{20,50}$ ]]
}

validate_user_idx() {
    [[ "$1" =~ ^[0-9]{6,15}$ ]]
}

# Try to use curl to get JSON automatically
try_auto_fetch() {
    local api_url="https://account.ridibooks.com/api/user-devices/app"
    
    print_info "Attempting to fetch device info automatically..."
    
    # Try different approaches to get cookies
    local cookie_file="/tmp/ridi_cookies.txt"
    
    # Method 1: Try to extract cookies from browsers
    local browsers=("Chrome" "Safari" "Firefox")
    for browser in "${browsers[@]}"; do
        if extract_browser_cookies "$browser" "$cookie_file"; then
            print_info "Found cookies from $browser"
            if fetch_with_cookies "$api_url" "$cookie_file"; then
                return 0
            fi
        fi
    done
    
    # Method 2: Guide user through simple cookie export
    echo
    echo "Let's try a simple approach:"
    echo "1. Open https://ridibooks.com and log in"
    echo "2. Press F12 to open developer tools"
    echo "3. Go to Application/Storage tab"
    echo "4. Click 'Cookies' > 'https://ridibooks.com'"
    echo "5. Look for a cookie named 'ridi_session' or similar"
    echo
    read -p "Do you see any RIDI-related cookies? (y/n): " has_cookies
    
    if [[ "$has_cookies" =~ ^[Yy] ]]; then
        echo "Copy the cookie value (the long string after the = sign)"
        read -p "Paste cookie value here: " cookie_value
        
        if [[ -n "$cookie_value" ]]; then
            echo "ridi_session=$cookie_value" > "$cookie_file"
            fetch_with_cookies "$api_url" "$cookie_file"
            return $?
        fi
    fi
    
    return 1
}

# Extract cookies from browser (macOS specific)
extract_browser_cookies() {
    local browser="$1"
    local output_file="$2"
    
    case "$browser" in
        "Chrome")
            local chrome_cookies="$HOME/Library/Application Support/Google/Chrome/Default/Cookies"
            if [[ -f "$chrome_cookies" ]] && command -v sqlite3 &> /dev/null; then
                sqlite3 "$chrome_cookies" \
                    "SELECT name, value FROM cookies WHERE host_key LIKE '%ridibooks%' OR host_key LIKE '%account.ridibooks%';" \
                    2>/dev/null | while IFS='|' read -r name value; do
                    echo "$name=$value" >> "$output_file"
                done
                [[ -s "$output_file" ]]
            fi
            ;;
        "Safari")
            # Safari cookies are more complex to extract
            return 1
            ;;
        "Firefox")
            # Firefox uses different cookie format
            return 1
            ;;
    esac
}

# Fetch JSON using cookies
fetch_with_cookies() {
    local url="$1"
    local cookie_file="$2"
    local output_file="/tmp/ridi_devices.json"
    
    if command -v curl &> /dev/null; then
        if curl -s -b "$cookie_file" "$url" -o "$output_file" 2>/dev/null; then
            if [[ -s "$output_file" ]] && grep -q "device_id" "$output_file" 2>/dev/null; then
                print_success "Successfully fetched device information!"
                parse_json_file "$output_file"
                return 0
            fi
        fi
    fi
    return 1
}

# Parse JSON file and extract credentials
parse_json_file() {
    local json_file="$1"
    
    if command -v jq &> /dev/null; then
        parse_with_jq_file "$json_file"
    else
        parse_with_regex "$json_file"
    fi
}

parse_with_jq_file() {
    local json_file="$1"
    local found_devices=false
    
    echo
    echo -e "${BLUE}Available Devices:${NC}"
    echo "===================="
    
    local device_count=1
    while IFS=$'\t' read -r device_id user_idx device_name os; do
        if [[ -n "$device_id" && -n "$user_idx" ]]; then
            found_devices=true
            echo -e "${CYAN}$device_count.${NC} ${BOLD}${device_name:-Unknown Device}${NC} [$os]"
            echo "   Device ID: $device_id"
            echo "   User Index: $user_idx"
            
            # Auto-validate
            local valid_device=false
            local valid_user=false
            
            if validate_device_id "$device_id"; then
                echo -e "   ${GREEN}✓ Device ID format: Valid${NC}"
                valid_device=true
            else
                echo -e "   ${YELLOW}⚠ Device ID format: Unusual${NC}"
            fi
            
            if validate_user_idx "$user_idx"; then
                echo -e "   ${GREEN}✓ User Index format: Valid${NC}"
                valid_user=true
            else
                echo -e "   ${YELLOW}⚠ User Index format: Unusual${NC}"
            fi
            
            if [[ "$valid_device" == true && "$valid_user" == true ]]; then
                echo -e "   ${GREEN}${BOLD}⭐ RECOMMENDED${NC}"
                
                # Auto-save first valid set
                if [[ $device_count -eq 1 ]]; then
                    save_credentials "$device_id" "$user_idx" "$device_name"
                fi
            fi
            
            echo
            ((device_count++))
        fi
    done < <(jq -r '.result[]? | select(.device_id and .user_idx) | [.device_id, .user_idx, (.device_name // "Unknown"), (.os // "Unknown")] | @tsv' "$json_file" 2>/dev/null)
    
    if [[ "$found_devices" == false ]]; then
        print_error "No devices found in JSON"
        return 1
    fi
    
    return 0
}

# Regex-based parsing fallback
parse_with_regex() {
    local json_file="$1"
    local content=$(cat "$json_file")
    
    echo "Parsing without jq..."
    
    # Extract device IDs
    local device_ids=($(echo "$content" | grep -oE '"device_id":\s*"[^"]*"' | grep -oE '[a-fA-F0-9-]{20,50}' | head -5))
    local user_indices=($(echo "$content" | grep -oE '"user_idx":\s*"?[0-9]+"?' | grep -oE '[0-9]{6,15}' | head -5))
    
    if [[ ${#device_ids[@]} -gt 0 && ${#user_indices[@]} -gt 0 ]]; then
        echo
        echo "Found credentials:"
        echo "Device ID: ${device_ids[0]}"
        echo "User Index: ${user_indices[0]}"
        
        save_credentials "${device_ids[0]}" "${user_indices[0]}" "Auto-detected"
        return 0
    fi
    
    return 1
}

# Save credentials to config
save_credentials() {
    local device_id="$1"
    local user_idx="$2" 
    local device_name="$3"
    
    echo
    print_info "Saving credentials..."
    
    local config_file="$HOME/.ridiculous.toml"
    
    # Backup existing
    if [[ -f "$config_file" ]]; then
        cp "$config_file" "$config_file.backup.$(date +%s)" 2>/dev/null
    fi
    
    cat > "$config_file" << EOF
# RIDI Credentials - Auto-generated
# Device: $device_name
# Generated: $(date)

device_id = "$device_id"
user_idx = "$user_idx"
verbose = false
EOF

    if [[ -f "$config_file" ]]; then
        print_success "Saved to $config_file"
        print_success "Setup complete! You can now run: ridiculous"
    else
        print_error "Could not save config file"
    fi
}

# Fallback: Simple manual entry
simple_manual_entry() {
    echo
    echo -e "${YELLOW}Manual Entry Mode${NC}"
    echo "=================="
    echo
    echo "Please open this URL and log in:"
    echo -e "${CYAN}https://account.ridibooks.com/api/user-devices/app${NC}"
    echo
    echo "Look for patterns like:"
    echo '  "device_id": "12345678-abcd-1234-5678-123456789abc"'
    echo '  "user_idx": "12345678"'
    echo
    
    while true; do
        read -p "Enter your device_id: " device_id
        device_id=$(echo "$device_id" | tr -d '"' | tr -d ' ' | tr -d ',')
        
        if [[ -n "$device_id" ]] && validate_device_id "$device_id"; then
            print_success "Device ID looks good"
            break
        elif [[ -n "$device_id" ]]; then
            echo "Device ID format seems unusual, but continuing..."
            break
        else
            echo "Please enter a device ID"
        fi
    done
    
    while true; do
        read -p "Enter your user_idx: " user_idx
        user_idx=$(echo "$user_idx" | tr -d '"' | tr -d ' ' | tr -d ',')
        
        if [[ -n "$user_idx" ]] && validate_user_idx "$user_idx"; then
            print_success "User index looks good"
            break
        elif [[ -n "$user_idx" ]]; then
            echo "User index format seems unusual, but continuing..."
            break
        else
            echo "Please enter a user index"
        fi
    done
    
    save_credentials "$device_id" "$user_idx" "Manual Entry"
}

# Open URL helper
open_url() {
    local url="$1"
    if command -v open &> /dev/null; then
        open "$url" 2>/dev/null
    elif command -v xdg-open &> /dev/null; then
        xdg-open "$url" 2>/dev/null
    elif command -v start &> /dev/null; then
        start "$url" 2>/dev/null
    fi
}

# Check if user is already logged in by trying to access API
check_login_status() {
    local api_url="https://account.ridibooks.com/api/user-devices/app"
    print_info "Checking if you're logged in..."
    
    if command -v curl &> /dev/null; then
        local response=$(curl -s "$api_url" 2>/dev/null || echo "")
        if echo "$response" | grep -q "device_id" 2>/dev/null; then
            print_success "You're already logged in!"
            echo "$response" > "/tmp/ridi_devices.json"
            if parse_json_file "/tmp/ridi_devices.json"; then
                return 0
            fi
        fi
    fi
    return 1
}

# Main function
main() {
    print_header
    
    echo "This script will automatically find your RIDI credentials."
    echo
    
    # First try: Check if already logged in and can get data directly
    if check_login_status; then
        return 0
    fi
    
    echo "You need to be logged into RIDI first."
    echo "Opening RIDI login page..."
    echo
    
    # Open login page
    open_url "https://ridibooks.com/account/login"
    
    echo "Please log in to your RIDI account in the browser that just opened."
    echo
    read -p "Press Enter after you've logged in..."
    
    echo
    
    # Try automatic methods
    if try_auto_fetch; then
        print_success "Automatic credential extraction successful!"
        return 0
    fi
    
    # Fallback to manual
    echo
    print_info "Automatic method didn't work, trying manual entry..."
    
    # Open the API endpoint for them
    print_info "Opening device API page..."
    open_url "https://account.ridibooks.com/api/user-devices/app"
    
    simple_manual_entry
}

# Run the script
main