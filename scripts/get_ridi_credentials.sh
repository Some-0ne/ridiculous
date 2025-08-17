#!/bin/bash

# Self-permission check - make script executable if it isn't
if [[ ! -x "$0" ]]; then
    echo "🔧 Making script executable..."
    chmod +x "$0"
    exec "$0" "$@"
fi

# Device ID Helper Script for Ridiculous Enhanced
# Helps users get their RIDI device ID, user index, and user ID
# Auto-handles permissions and provides multiple execution methods

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Global variables for credentials
device_id=""
user_idx=""

# Initialize options
NO_BROWSER=false
MANUAL_ONLY=false

# ============================================================================
# SYSTEM DETECTION FUNCTIONS
# ============================================================================

# Detect current system information
detect_current_system() {
    local system_info=()
    
    # Detect OS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        local mac_model=""
        if command -v system_profiler &> /dev/null; then
            mac_model=$(system_profiler SPHardwareDataType 2>/dev/null | grep "Model Name" | cut -d: -f2 | xargs)
        fi
        if [[ -z "$mac_model" ]]; then
            mac_model=$(sysctl -n hw.model 2>/dev/null || echo "Mac")
        fi
        system_info+=("macOS" "$mac_model" "Mac" "macOS" "OSX")
        
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux (could be Android via Termux or regular Linux)
        if [[ -f "/system/build.prop" ]] || command -v getprop &> /dev/null; then
            # Likely Android via Termux
            local android_model=$(getprop ro.product.model 2>/dev/null || echo "Android Device")
            system_info+=("Android" "$android_model" "android" "mobile")
        else
            # Regular Linux
            local distro=$(lsb_release -d 2>/dev/null | cut -f2 || echo "Linux")
            local arch=$(uname -m)
            system_info+=("Linux" "$distro" "linux" "$arch" "desktop")
        fi
        
    elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
        # Windows (Git Bash, Cygwin, WSL)
        local win_model=""
        if command -v wmic &> /dev/null; then
            win_model=$(wmic computersystem get model /format:value 2>/dev/null | grep "Model=" | cut -d= -f2 | tr -d '\r\n' || echo "Windows PC")
        elif command -v powershell &> /dev/null; then
            win_model=$(powershell -Command "Get-WmiObject -Class Win32_ComputerSystem | Select-Object -ExpandProperty Model" 2>/dev/null || echo "Windows PC")
        else
            win_model="Windows PC"
        fi
        system_info+=("Windows" "$win_model" "windows" "desktop" "pc")
        
    else
        # Unknown/Other
        system_info+=("Unknown" "Unknown System" "unknown")
    fi
    
    printf '%s\n' "${system_info[@]}"
}

# Match detected system against RIDI device list
match_system_to_devices() {
    local system_info_str="$1"
    local devices_json="$2"
    
    if [[ -z "$devices_json" ]]; then
        return 1
    fi
    
    # Convert system info string back to array
    local -a system_info_array
    IFS=$'\n' read -d '' -r -a system_info_array <<< "$system_info_str" || true
    
    local current_os="${system_info_array[0]}"
    local current_model="${system_info_array[1]}"
    local -a search_terms=("${system_info_array[@]:2}")
    
    echo -e "${BLUE}🔍 System Detection Results:${NC}"
    echo -e "  Current OS: ${CYAN}$current_os${NC}"
    echo -e "  Device Model: ${CYAN}$current_model${NC}"
    echo
    
    # Try to find matching devices in the JSON
    local matches_found=false
    local device_count=1
    
    if command -v jq &> /dev/null; then
        # Use jq to extract device information with matching
        while IFS=$'\t' read -r device_id_val user_idx_val device_name os device_type; do
            if [[ -n "$device_id_val" ]]; then
                local match_score=0
                local match_reasons=()
                
                # Score matching based on various factors
                for term in "${search_terms[@]}"; do
                    if [[ "$os" =~ $term ]] || [[ "$device_name" =~ $term ]] || [[ "$device_type" =~ $term ]]; then
                        ((match_score++))
                        match_reasons+=("$term")
                    fi
                done
                
                # Check for partial matches in device names
                if [[ "$device_name" =~ $current_model ]] || [[ "$current_model" =~ $device_name ]]; then
                    ((match_score += 2))
                    match_reasons+=("model name")
                fi
                
                # Display device with match information
                local match_indicator=""
                local match_color="$NC"
                
                if [[ $match_score -gt 0 ]]; then
                    matches_found=true
                    match_indicator="🎯 LIKELY MATCH"
                    match_color="$GREEN"
                elif [[ "$os" =~ ^($current_os|${current_os,,}|${current_os^^}) ]]; then
                    match_indicator="📱 Same OS"
                    match_color="$YELLOW"
                fi
                
                echo -e "${match_color}${device_count}. $match_indicator${NC}"
                echo -e "   Device: ${CYAN}${device_name:-Unknown}${NC} [${os:-Unknown OS}]"
                echo -e "   Type: ${device_type:-Unknown}"
                echo -e "   ID: $device_id_val"
                echo -e "   User Index: $user_idx_val"
                
                if [[ ${#match_reasons[@]} -gt 0 ]]; then
                    echo -e "   ${GREEN}Match reasons: ${match_reasons[*]}${NC}"
                fi
                
                if [[ $match_score -ge 2 ]]; then
                    echo -e "   ${GREEN}${BOLD}⭐ RECOMMENDED CHOICE${NC}"
                fi
                
                echo
                ((device_count++))
            fi
        done < <(echo "$devices_json" | jq -r '.result[]? | select(.device_id and .user_idx) | [.device_id, .user_idx, (.device_name // ""), (.os // ""), (.device_type // "")] | @tsv' 2>/dev/null)
    fi
    
    if [[ "$matches_found" == "true" ]]; then
        echo -e "${GREEN}✅ Found devices that might match your current system!${NC}"
        echo -e "${YELLOW}💡 Look for entries marked as 'LIKELY MATCH' or 'RECOMMENDED CHOICE'${NC}"
    else
        echo -e "${YELLOW}ℹ️  No obvious matches found, but any device should work${NC}"
        echo -e "   Choose the device you use most often for reading RIDI books."
    fi
    
    return 0
}

# ============================================================================
# ENHANCED VALIDATION FUNCTIONS
# ============================================================================

# Enhanced validation for device ID format - now more precise for RIDI UUIDs
validate_device_id() {
    local device_id="$1"
    
    # Primary check: Standard UUID v4 format (8-4-4-4-12 hexadecimal with dashes)
    if [[ "$device_id" =~ ^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}$ ]]; then
        return 0
    fi
    
    # Secondary check: UUID-like but with mixed case or slight variations
    if [[ "$device_id" =~ ^[a-zA-Z0-9]{8}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{12}$ ]]; then
        return 0
    fi
    
    # Tertiary check: 32-character hex string without dashes
    if [[ "$device_id" =~ ^[a-fA-F0-9]{32}$ ]]; then
        return 0
    fi
    
    # Fallback: Any reasonable device ID pattern
    if [[ "$device_id" =~ ^[a-zA-Z0-9-]{20,50}$ ]] && [[ "$device_id" == *"-"* ]]; then
        return 0
    fi
    
    return 1
}

# Improved validation for user index format  
validate_user_idx() {
    local user_idx="$1"
    
    # User index should be numbers only, typically 8+ digits for RIDI
    if [[ "$user_idx" =~ ^[0-9]{6,15}$ ]]; then
        return 0
    fi
    
    return 1
}

# New function to classify device ID type
classify_device_id() {
    local device_id="$1"
    
    if [[ "$device_id" =~ ^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}$ ]]; then
        echo "UUID v4 (Standard)"
    elif [[ "$device_id" =~ ^[a-zA-Z0-9]{8}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{12}$ ]]; then
        echo "UUID-like"
    elif [[ "$device_id" =~ ^[a-fA-F0-9]{32}$ ]]; then
        echo "32-char Hex"
    else
        echo "Alternative Format"
    fi
}

# Print colored header
print_header() {
    echo -e "${CYAN}${BOLD}"
    echo "🔑 ═══════════════════════════════════════════════════════════"
    echo "   RIDI Device ID & User Index Helper"
    echo "   ═══════════════════════════════════════════════════════════"
    echo -e "${NC}"
    echo
}

# Print step with number
print_step() {
    echo -e "${BLUE}${BOLD}📋 Step $1:${NC} $2"
    echo
}

# Print warning
print_warning() {
    echo -e "${YELLOW}⚠️  Warning:${NC} $1"
    echo
}

# Print success
print_success() {
    echo -e "${GREEN}✅ Success:${NC} $1"
    echo
}

# Print error
print_error() {
    echo -e "${RED}❌ Error:${NC} $1"
    echo
}

# Check if jq is available
check_dependencies() {
    if ! command -v jq &> /dev/null; then
        print_warning "jq is not installed but recommended for JSON parsing"
        echo "  On macOS: brew install jq"
        echo "  On Ubuntu/Debian: sudo apt install jq"
        echo "  On Arch Linux: sudo pacman -S jq"
        echo "  On CentOS/RHEL: sudo yum install jq"
        echo
        echo "  Don't worry - we'll help you parse the JSON manually if needed!"
        echo
        return 1
    fi
    return 0
}

# Open RIDI login page
open_login_page() {
    print_step "1" "Opening RIDI login page..."
    
    local login_url="https://ridibooks.com/account/login"
    
    if [[ "$NO_BROWSER" == "true" ]]; then
        echo "Browser auto-open disabled. Please manually visit:"
        echo -e "🌐 Login URL: ${CYAN}$login_url${NC}"
    else
        # Try to open the URL
        local opened=false
        if command -v open &> /dev/null; then
            # macOS
            if open "$login_url" 2>/dev/null; then
                echo "✅ Opened in default browser (macOS)"
                opened=true
            fi
        elif command -v xdg-open &> /dev/null; then
            # Linux
            if xdg-open "$login_url" 2>/dev/null; then
                echo "✅ Opened in default browser (Linux)"
                opened=true
            fi
        elif command -v start &> /dev/null; then
            # Windows (Git Bash, WSL)
            if start "$login_url" 2>/dev/null; then
                echo "✅ Opened in default browser (Windows)"
                opened=true
            fi
        fi
        
        if [[ "$opened" == "false" ]]; then
            print_warning "Could not automatically open browser"
            echo "  Please manually open the URL below"
        fi
        
        echo -e "🌐 Login URL: ${CYAN}$login_url${NC}"
    fi
    
    echo
    echo "Please log in to your RIDI account in the browser."
    echo
    read -p "Press Enter when you have successfully logged in..."
    echo
}

# Get user profile info (optional step to get additional user information)
get_user_profile() {
    print_step "2a" "Checking user profile for additional information..."
    
    local profile_url="https://account.ridibooks.com/api/user/profile"
    
    echo -e "🔗 Profile API URL: ${CYAN}$profile_url${NC}"
    echo
    echo "This can help find additional user information if needed."
    
    if [[ "$NO_BROWSER" == "true" ]]; then
        echo "Browser auto-open disabled. Please manually visit the URL above."
    else
        echo "Attempting to open this URL in a new tab..."
        echo -e "${BOLD}$profile_url${NC}"
        
        # Try to open profile URL
        local opened=false
        if command -v open &> /dev/null; then
            open "$profile_url" 2>/dev/null && opened=true
        elif command -v xdg-open &> /dev/null; then
            xdg-open "$profile_url" 2>/dev/null && opened=true
        elif command -v start &> /dev/null; then
            start "$profile_url" 2>/dev/null && opened=true
        fi
        
        if [[ "$opened" == "false" ]]; then
            echo "Could not auto-open browser. Please visit the URL above manually."
        fi
    fi
    
    echo
    read -p "Press Enter to continue to device information..."
    echo
}

# Open API endpoint and get JSON
get_device_info() {
    print_step "2" "Opening RIDI API endpoint to get device information..."
    
    local api_url="https://account.ridibooks.com/api/user-devices/app"
    
    if [[ "$NO_BROWSER" == "true" ]]; then
        echo "Browser auto-open disabled. Please manually visit:"
        echo -e "🔗 API URL: ${CYAN}$api_url${NC}"
    else
        # Try to open the API URL
        local opened=false
        if command -v open &> /dev/null; then
            if open "$api_url" 2>/dev/null; then
                echo "✅ Opened API endpoint in browser"
                opened=true
            fi
        elif command -v xdg-open &> /dev/null; then
            if xdg-open "$api_url" 2>/dev/null; then
                echo "✅ Opened API endpoint in browser"
                opened=true
            fi
        elif command -v start &> /dev/null; then
            if start "$api_url" 2>/dev/null; then
                echo "✅ Opened API endpoint in browser"
                opened=true
            fi
        fi
        
        if [[ "$opened" == "false" ]]; then
            print_warning "Could not automatically open browser"
            echo "  Please manually open the URL below"
        fi
        
        echo -e "🔗 API URL: ${CYAN}$api_url${NC}"
    fi
    
    echo
    echo "This page will show JSON data with your device information."
    echo "Look for entries that contain 'device_id' and 'user_idx' fields."
    echo
    echo -e "${YELLOW}💡 Expected device_id format:${NC} 8-4-4-4-12 characters with dashes"
    echo -e "   ${CYAN}Example pattern: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx${NC}"
    echo
    echo "If you see an error or empty result, make sure you're still logged in."
    echo
    read -p "Press Enter when the API page has loaded..."
    echo
}

# Enhanced JSON parsing with better input handling
parse_with_jq() {
    echo "Great! Let's parse the JSON to find your credentials."
    echo
    echo "Please copy the ENTIRE JSON response from the browser."
    echo "Tips:"
    echo "  • Select all with Cmd+A (Mac) or Ctrl+A (Windows/Linux)"
    echo "  • If you see 'null' or '[]', make sure you're logged in"
    echo "  • The JSON should contain device information"
    echo
    
    # Offer multiple input methods
    echo "Choose how to provide the JSON:"
    echo "1. Paste directly and type 'END' when finished"
    echo "2. Save JSON to a file and provide the file path"
    echo "3. Use HERE document method (advanced)"
    echo
    read -p "Choose method (1, 2, or 3): " input_method
    
    local json_content=""
    
    case $input_method in
        1)
            echo
            echo "Paste your JSON below, then type 'END' on a new line when finished:"
            echo "----------------------------------------"
            
            while IFS= read -r line; do
                # Check if user typed END to finish
                if [[ "$line" == "END" ]] || [[ "$line" == "end" ]]; then
                    break
                fi
                json_content+="$line"$'\n'
            done
            ;;
            
        2)
            echo
            echo "Save your JSON to a temporary file first, then provide the path."
            echo "Example: /tmp/ridi_devices.json or ~/Desktop/devices.json"
            echo
            read -p "Enter the full path to your JSON file: " json_file_path
            
            # Expand tilde and handle spaces
            json_file_path="${json_file_path/#\~/$HOME}"
            
            if [[ -f "$json_file_path" ]]; then
                json_content=$(cat "$json_file_path")
                echo "✅ Successfully read JSON from file"
            else
                print_error "File not found: $json_file_path"
                echo "Make sure the file exists and the path is correct."
                return 1
            fi
            ;;
            
        3)
            echo
            echo "Advanced method using HERE document:"
            echo "1. Type: cat > /tmp/ridi_json.tmp << 'JSONEND'"
            echo "2. Paste your JSON"
            echo "3. Type: JSONEND"
            echo "4. Press Enter to continue here"
            echo
            read -p "Press Enter when you've completed the above steps..."
            
            if [[ -f "/tmp/ridi_json.tmp" ]]; then
                json_content=$(cat "/tmp/ridi_json.tmp")
                echo "✅ Successfully read JSON from temporary file"
                # Clean up
                rm -f "/tmp/ridi_json.tmp"
            else
                print_error "Temporary file not found"
                return 1
            fi
            ;;
            
        *)
            print_error "Invalid choice"
            return 1
            ;;
    esac
    
    echo "----------------------------------------"
    
    if [[ -z "$json_content" ]]; then
        print_error "No JSON content provided"
        return 1
    fi
    
    # Show first and last few characters to confirm we got something reasonable
    local content_length=${#json_content}
    local preview_start=$(echo "$json_content" | head -c 100)
    local preview_end=$(echo "$json_content" | tail -c 100)
    
    echo "JSON received: $content_length characters"
    echo "Starts with: ${preview_start}..."
    echo "Ends with: ...${preview_end}"
    echo
    
    # Check if JSON is valid
    echo "🔍 Validating JSON format..."
    if ! echo "$json_content" | jq empty >/dev/null 2>&1; then
        print_error "Invalid JSON format detected"
        echo
        echo "Common issues and solutions:"
        echo "• Extra text before/after JSON - make sure you copied only the JSON"
        echo "• Incomplete JSON - ensure you copied everything from { to }"
        echo "• Special characters - some terminals modify pasted content"
        echo
        echo "First 500 characters of what we received:"
        echo "----------------------------------------"
        echo "$json_content" | head -c 500
        echo
        echo "----------------------------------------"
        echo "Last 200 characters:"
        echo "----------------------------------------"
        echo "$json_content" | tail -c 200
        echo "----------------------------------------"
        
        return 1
    fi
    
    echo "✅ JSON format is valid!"
    echo
    
    # Check for common error patterns
    if echo "$json_content" | jq -e '.result == null or (.result | length) == 0' >/dev/null 2>&1; then
        print_error "API returned empty result - make sure you're logged in"
        echo "The API response indicates no devices found."
        echo "Please ensure you're logged into RIDI in your browser and try again."
        return 1
    fi
    
    echo -e "${GREEN}✅ JSON parsed successfully!${NC}"
    echo
    
    # Detect current system before showing devices
    local system_info
    system_info=$(detect_current_system)
    
    # Show system-matched devices with smart recommendations
    if match_system_to_devices "$system_info" "$json_content"; then
        echo
    fi
    
    # Show traditional device list as backup
    local devices
    if devices=$(echo "$json_content" | jq -r '.result[]? | select(.device_id and .user_idx) | "Device: \(.device_name // .device_type // "Unknown") [\(.os // "Unknown OS")] - ID: \(.device_id) - User Index: \(.user_idx)"' 2>/dev/null); then
        if [[ -n "$devices" ]]; then
            echo -e "${BLUE}📋 Complete Device List:${NC}"
            local device_count=1
            
            # Enhanced device display with validation
            while IFS=$'\t' read -r device_id_val user_idx_val device_name os device_type; do
                if [[ -n "$device_id_val" ]]; then
                    echo -e "${CYAN}${device_count}.${NC} Device: ${BOLD}${device_name:-Unknown}${NC} [${os:-Unknown OS}]"
                    
                    # Validate and classify device ID
                    if validate_device_id "$device_id_val"; then
                        local id_type=$(classify_device_id "$device_id_val")
                        echo -e "     Device ID: ${GREEN}$device_id_val${NC} ✅ ($id_type)"
                    else
                        echo -e "     Device ID: ${YELLOW}$device_id_val${NC} ⚠️  (length: ${#device_id_val})"
                    fi
                    
                    # Validate user index
                    if validate_user_idx "$user_idx_val"; then
                        echo -e "     User Index: ${GREEN}$user_idx_val${NC} ✅ (${#user_idx_val} digits)"
                    else
                        echo -e "     User Index: ${YELLOW}$user_idx_val${NC} ⚠️  (${#user_idx_val} digits)"
                    fi
                    
                    echo
                    ((device_count++))
                fi
            done < <(echo "$json_content" | jq -r '.result[]? | select(.device_id and .user_idx) | [.device_id, .user_idx, (.device_name // ""), (.os // ""), (.device_type // "")] | @tsv' 2>/dev/null)
            
            echo -e "${CYAN}💡 Selection Tips:${NC}"
            echo "• Look for devices marked as 'LIKELY MATCH' or 'RECOMMENDED CHOICE'"
            echo "• UUID v4 format device IDs typically work best"
            echo "• Choose the device you use most often for reading"
            echo
            
            return 0
        fi
    fi
    
    print_error "Could not parse JSON automatically"
    echo "The JSON structure might be different than expected."
    
    # Enhanced fallback: try to extract with improved regex patterns
    echo
    echo "Attempting fallback regex extraction..."
    
    # Look for UUID-pattern device_ids (prioritize standard UUID format)
    local regex_device_ids
    regex_device_ids=$(echo "$json_content" | grep -oE '"device_id":\s*"[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}"' | grep -oE '[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{4}-[a-fA-F0-9]{12}' | head -5)
    
    # If no perfect UUIDs found, look for any device_id pattern
    if [[ -z "$regex_device_ids" ]]; then
        regex_device_ids=$(echo "$json_content" | grep -oE '"device_id":\s*"[^"]{20,50}"' | grep -oE '[a-fA-F0-9-]{20,50}' | head -5)
    fi
    
    if [[ -n "$regex_device_ids" ]]; then
        echo -e "${CYAN}Found potential device IDs with regex:${NC}"
        local count=1
        while IFS= read -r did; do
            if [[ -n "$did" ]]; then
                local id_type=$(classify_device_id "$did")
                if validate_device_id "$did"; then
                    echo -e "  ${count}. ${GREEN}$did${NC} ✅ ($id_type)"
                else
                    echo -e "  ${count}. ${YELLOW}$did${NC} ⚠️ ($id_type)"
                fi
                ((count++))
            fi
        done <<< "$regex_device_ids"
        echo
    fi
    
    # Look for user_idx patterns (numeric)
    local regex_user_indices
    regex_user_indices=$(echo "$json_content" | grep -oE '"user_idx":\s*"?[0-9]{6,15}"?' | grep -oE '[0-9]{6,15}' | head -5)
    
    if [[ -n "$regex_user_indices" ]]; then
        echo -e "${CYAN}Found potential user indices with regex:${NC}"
        local count=1
        while IFS= read -r uidx; do
            if [[ -n "$uidx" ]]; then
                if validate_user_idx "$uidx"; then
                    echo -e "  ${count}. ${GREEN}$uidx${NC} ✅ (${#uidx} digits)"
                else
                    echo -e "  ${count}. ${YELLOW}$uidx${NC} ⚠️ (${#uidx} digits)"
                fi
                ((count++))
            fi
        done <<< "$regex_user_indices"
        echo
    fi
    
    return 1
}

# Enhanced manual parsing with system detection and smart suggestions
parse_manually() {
    echo -e "${YELLOW}📝 Let's find your credentials manually:${NC}"
    echo
    
    # Show system detection first
    local system_info
    system_info=$(detect_current_system)
    local current_os
    current_os=$(echo "$system_info" | head -n 1)
    
    echo -e "${BLUE}🔍 Detected System: ${CYAN}$current_os${NC}"
    echo
    
    echo "In the JSON you see in your browser, look for patterns like:"
    echo
    echo -e "${CYAN}  \"device_id\": \"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\",${NC}"
    echo -e "${CYAN}  \"user_idx\": \"12345678\",${NC}"
    echo -e "${CYAN}  \"device_name\": \"Your Device Name\",${NC}"
    echo -e "${CYAN}  \"os\": \"$current_os\",${NC}"
    echo
    echo -e "${GREEN}Device ID format examples:${NC}"
    echo "  • Standard UUID: 12345678-abcd-1234-5678-123456789abc (recommended)"
    echo "  • 32-char hex: 1234567890abcdef1234567890abcdef"
    echo "  • Other formats with dashes and alphanumeric characters"
    echo
    echo -e "${GREEN}User Index format:${NC}"
    echo "  • Numbers only: 12345678 (typically 6-15 digits)"
    echo
    echo -e "${YELLOW}💡 Smart Selection Tips:${NC}"
    echo "• Look for devices with OS matching '$current_os'"
    echo "• Choose the device you're currently using or use most often"
    echo "• If unsure, pick any device - they usually all work"
    echo "• There might be multiple devices listed"
    echo
    
    # Get device ID with enhanced validation and system-aware feedback
    while true; do
        echo
        echo -e "${BLUE}Available devices in your JSON might include:${NC}"
        echo "• Desktop/laptop devices (Windows, macOS, Linux)"
        echo "• Mobile devices (iOS, Android)"
        echo "• Tablet devices"
        echo "• Web browser sessions"
        echo
        read -p "Please enter your device_id: " input_device_id
        
        if [[ -z "$input_device_id" ]]; then
            print_error "Device ID cannot be empty"
            continue
        fi
        
        # Clean up the input (remove quotes, spaces, commas)
        device_id=$(echo "$input_device_id" | sed 's/[",]//g' | tr -d ' ')
        
        if validate_device_id "$device_id"; then
            local id_type=$(classify_device_id "$device_id")
            print_success "Device ID format looks correct! ($id_type)"
            if [[ "$id_type" == "UUID v4 (Standard)" ]]; then
                echo -e "  ${GREEN}Perfect!${NC} This is the standard UUID format that works best."
            fi
            break
        else
            print_warning "Device ID format seems unusual"
            echo "  Expected: UUID format like 12345678-abcd-1234-5678-123456789abc"
            echo "  Got: $device_id (length: ${#device_id})"
            
            # Provide specific guidance based on what they entered
            if [[ ${#device_id} -lt 20 ]]; then
                echo "  Issue: Too short (expected at least 20 characters)"
            elif [[ ${#device_id} -gt 50 ]]; then
                echo "  Issue: Too long (expected max 50 characters)"
            elif [[ ! "$device_id" =~ [-] ]]; then
                echo "  Issue: Missing dashes (UUID should have dashes)"
            fi
            
            echo
            read -p "Use this device ID anyway? (y/n): " use_anyway
            if [[ "$use_anyway" =~ ^[Yy] ]]; then
                break
            fi
        fi
    done
    
    # Get user index with enhanced validation
    while true; do
        echo
        echo -e "${BLUE}User Index Info:${NC}"
        echo "• This should be the same for all your devices"
        echo "• Look for 'user_idx' in the JSON"
        echo "• It's typically a 6-15 digit number"
        echo
        read -p "Please enter your user_idx: " input_user_idx
        
        if [[ -z "$input_user_idx" ]]; then
            print_error "User index cannot be empty"
            continue
        fi
        
        # Clean up the input
        user_idx=$(echo "$input_user_idx" | sed 's/[",]//g' | tr -d ' ')
        
        if validate_user_idx "$user_idx"; then
            print_success "User index format looks correct! (${#user_idx} digits)"
            break
        else
            print_warning "User index format seems unusual"
            echo "  Expected: Numbers only, like 12345678 (6-15 digits)"
            echo "  Got: $user_idx (length: ${#user_idx})"
            
            # Provide specific guidance
            if [[ ! "$user_idx" =~ ^[0-9]+$ ]]; then
                echo "  Issue: Contains non-numeric characters"
            elif [[ ${#user_idx} -lt 6 ]]; then
                echo "  Issue: Too short (expected at least 6 digits)"
            elif [[ ${#user_idx} -gt 15 ]]; then
                echo "  Issue: Too long (expected max 15 digits)"
            fi
            
            echo
            read -p "Use this user index anyway? (y/n): " use_anyway
            if [[ "$use_anyway" =~ ^[Yy] ]]; then
                break
            fi
        fi
    done
    
    echo
    print_success "Credentials collected!"
    echo -e "  Device ID: ${GREEN}$device_id${NC} ($(classify_device_id "$device_id"))"
    echo -e "  User Index: ${GREEN}$user_idx${NC} (${#user_idx} digits)"
    echo -e "  Detected System: ${CYAN}$current_os${NC}"
    echo
}

# Enhanced validation using new validation functions
validate_credentials() {
    local errors=0
    
    echo -e "${BLUE}🔍 Validating credential format...${NC}"
    
    # Check device_id format using enhanced validation
    if ! validate_device_id "$device_id"; then
        print_warning "Device ID format seems unusual: $device_id"
        echo "  Expected UUID format: 12345678-abcd-1234-5678-123456789abc"
        ((errors++))
    else
        local id_type=$(classify_device_id "$device_id")
        echo -e "  ${GREEN}Device ID:${NC} Valid ($id_type)"
    fi
    
    # Check user_idx format using enhanced validation
    if ! validate_user_idx "$user_idx"; then
        print_warning "User index format seems unusual: $user_idx"
        echo "  Expected: Numbers only, 6-15 digits"
        ((errors++))
    else
        echo -e "  ${GREEN}User Index:${NC} Valid (${#user_idx} digits)"
    fi
    
    if [[ $errors -eq 0 ]]; then
        print_success "All credential formats look good!"
    else
        echo -e "${YELLOW}Found $errors potential format issues, but continuing...${NC}"
        echo
    fi
}

# Test credentials
test_credentials() {
    echo -e "${BLUE}🧪 Testing credentials...${NC}"
    echo
    
    # Try to run ridiculous in validation mode
    if command -v ridiculous &> /dev/null; then
        echo "Testing with ridiculous --validate-only..."
        local test_args="--device-id \"$device_id\" --user-idx \"$user_idx\""
        
        if eval "ridiculous $test_args --validate-only" 2>/dev/null; then
            print_success "Credentials appear to be valid!"
        else
            print_warning "Could not validate credentials automatically"
            echo "  This might be normal if you haven't downloaded any books yet"
            echo "  or if the ridiculous tool needs additional setup"
        fi
    else
        print_warning "Ridiculous binary not found in PATH"
        echo "  You can test these credentials after building the project"
    fi
    echo
}

# Save to config
save_config() {
    echo -e "${BLUE}💾 Save credentials to config file?${NC}"
    echo "  This will create ~/.ridiculous.toml with your credentials"
    echo "  so you won't need to enter them each time."
    echo
    
    while true; do
        read -p "Save to config? (y/n): " save_choice
        case $save_choice in
            [Yy]* ) 
                # Create config file
                local config_file="$HOME/.ridiculous.toml"
                
                # Backup existing config if it exists
                if [[ -f "$config_file" ]]; then
                    cp "$config_file" "$config_file.backup.$(date +%s)"
                    echo "  Backed up existing config to $config_file.backup.*"
                fi
                
                cat > "$config_file" << EOF
# Ridiculous Enhanced Configuration
# Generated on $(date)

device_id = "$device_id"
user_idx = "$user_idx"
verbose = false
organize_output = false
backup_originals = false
EOF
                print_success "Configuration saved to $config_file"
                break
                ;;
            [Nn]* )
                echo "Configuration not saved. You can run this script again later."
                break
                ;;
            * )
                echo "Please answer yes (y) or no (n)."
                ;;
        esac
    done
    echo
}

# Print final instructions
print_final_instructions() {
    echo -e "${GREEN}${BOLD}🎉 Setup Complete!${NC}"
    echo
    echo "Your RIDI credentials:"
    echo -e "  Device ID: ${CYAN}$device_id${NC} ($(classify_device_id "$device_id"))"
    echo -e "  User Index: ${CYAN}$user_idx${NC} (${#user_idx} digits)"
    echo
    echo -e "${YELLOW}📋 Next Steps:${NC}"
    echo "1. Make sure RIDI app is installed and you're logged in"
    echo "2. Download some books from your purchases in the RIDI app"
    echo "3. Run ridiculous to start decrypting your books:"
    echo
    echo -e "   ${CYAN}# Simple usage (uses saved config):${NC}"
    echo -e "   ${BOLD}ridiculous${NC}"
    echo
    echo -e "   ${CYAN}# Or with explicit credentials:${NC}"
    echo -e "   ${BOLD}ridiculous --device-id \"$device_id\" --user-idx \"$user_idx\"${NC}"
    echo
    echo -e "   ${CYAN}# Verbose mode with progress:${NC}"
    echo -e "   ${BOLD}ridiculous --verbose${NC}"
    echo
    echo -e "   ${CYAN}# Run diagnostics if you have issues:${NC}"
    echo -e "   ${BOLD}ridiculous --diagnose${NC}"
    echo
    echo -e "${GREEN}✨ Enjoy your DRM-free books!${NC}"
    echo
    echo -e "${YELLOW}💡 Troubleshooting tips:${NC}"
    echo "• If you get authentication errors, try logging out and back into RIDI"
    echo "• Make sure books are fully downloaded in the RIDI app before decrypting"
    echo "• Different devices might have different capabilities"
    echo "• UUID v4 format device IDs typically work most reliably"
    echo "• You can run this script again to get credentials for other devices"
    echo
}

# Show alternative methods if main method fails
show_alternatives() {
    echo -e "${BLUE}🔄 Alternative methods:${NC}"
    echo
    echo "If the automatic method didn't work, you can also try:"
    echo
    echo "1. Browser Developer Tools:"
    echo "   • Open browser dev tools (F12)"
    echo "   • Go to Network tab"
    echo "   • Visit the API URL again"
    echo "   • Look for the request and copy the response"
    echo
    echo "2. Manual API call:"
    echo "   • Use curl or similar tool with your browser cookies"
    echo "   • Export cookies from browser and use with curl"
    echo
    echo "3. Check different endpoints:"
    echo "   • Sometimes user info is in profile endpoint"
    echo "   • Try: https://account.ridibooks.com/api/user/profile"
    echo
    echo "4. Look for UUID patterns:"
    echo "   • Device IDs often follow the format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
    echo "   • These are typically the most compatible with decryption tools"
    echo
}

# Help function
show_help() {
    echo "RIDI Credentials Helper Script"
    echo
    echo "Usage:"
    echo "  $0 [options]"
    echo
    echo "Options:"
    echo "  -h, --help     Show this help message"
    echo "  --no-browser   Don't try to open browser automatically"
    echo "  --manual-only  Skip automatic JSON parsing, go straight to manual entry"
    echo
    echo "This script automatically handles permission issues."
    echo "If you get permission denied, it will try to fix itself."
    echo
    echo "The script looks for RIDI device IDs in UUID format:"
    echo "  Standard format: 12345678-abcd-1234-5678-123456789abc"
    echo "  This format typically works best with RIDI decryption tools."
    echo
    echo "Alternative ways to run:"
    echo "  bash $0        # Always works, bypasses permission issues"
    echo "  chmod +x $0 && $0  # Fix permissions and run"
    echo
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            --no-browser)
                NO_BROWSER=true
                shift
                ;;
            --manual-only)
                MANUAL_ONLY=true
                shift
                ;;
            *)
                echo "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Main script execution
main() {
    clear
    print_header
    
    echo "This script will help you get your RIDI device ID and user index"
    echo "needed for the ridiculous book decryption tool."
    echo
    echo -e "${YELLOW}⚠️  Important:${NC}"
    echo "• You must have a RIDI account with purchased books"
    echo "• You need to be logged into RIDI in your browser"
    echo "• This process is safe and only retrieves your own device info"
    echo "• Keep your credentials secure and don't share them"
    echo "• Look for UUID format device IDs (they work best)"
    echo
    
    # Show execution method
    if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
        echo -e "${GREEN}✅ Running with proper permissions${NC}"
    else
        echo -e "${BLUE}ℹ️  Running via bash interpreter${NC}"
    fi
    echo
    
    read -p "Ready to start? Press Enter to continue..."
    echo
    
    # Check if we have helpful tools
    local has_jq=false
    if check_dependencies; then
        has_jq=true
    fi
    
    # Step 1: Login
    open_login_page
    
    # Optional: Check profile for additional info
    echo -e "${YELLOW}Would you like to check the user profile endpoint first?${NC}"
    echo "This might provide additional user information if needed."
    read -p "Check profile? (y/n): " check_profile
    if [[ "$check_profile" =~ ^[Yy] ]]; then
        get_user_profile
    fi
    
    # Step 2: Get device info
    get_device_info
    
    # Step 3: Parse JSON
    local parse_success=false
    
    if [[ "$MANUAL_ONLY" == "true" ]]; then
        echo -e "${YELLOW}Manual-only mode selected. Skipping automatic parsing.${NC}"
        echo
        parse_manually
    else
        if $has_jq; then
            if parse_with_jq; then
                echo -e "${GREEN}✅ Successfully parsed device information!${NC}"
                echo
                echo "You can use any of the device IDs and user indices shown above."
                echo "Typically, choose the device you use most often for reading."
                echo -e "${CYAN}💡 Tip:${NC} UUID v4 format device IDs usually work best."
                echo
                parse_success=true
            fi
        fi
        
        # Always offer manual parsing as backup
        if ! $parse_success; then
            echo -e "${BLUE}Let's try manual entry to ensure accuracy...${NC}"
            echo
            parse_manually
        else
            echo "Do you want to manually enter different credentials instead?"
            echo "(This lets you choose specific device IDs from the list above)"
            read -p "Manual entry? (y/n): " manual_choice
            if [[ "$manual_choice" =~ ^[Yy] ]]; then
                parse_manually
            fi
        fi
    fi
    
    # Step 4: Validate and test credentials
    if [[ -n "$device_id" && -n "$user_idx" ]]; then
        validate_credentials
        test_credentials
        save_config
        print_final_instructions
    else
        print_error "Could not obtain valid credentials"
        show_alternatives
        exit 1
    fi
}

# Handle interrupts gracefully
trap 'echo; print_error "Script interrupted by user"; exit 1' INT

# ===============================================
# ENTRY POINT - Handle permissions automatically
# ===============================================

# Parse arguments
parse_args "$@"

# Run main script
main
        echo "