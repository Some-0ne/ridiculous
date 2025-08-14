#!/bin/bash

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

# Permission handling function - runs at the very start
handle_permissions() {
    local script_path="${BASH_SOURCE[0]}"
    local script_name="$(basename "$script_path")"
    
    # Check if we're running directly (not via bash/sh)
    if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
        # We're running the script directly, check if it's executable
        if [[ ! -x "$script_path" ]]; then
            echo -e "${RED}❌ Permission denied: Script is not executable${NC}"
            echo
            echo "🔧 Attempting to fix permissions automatically..."
            
            # Try to make it executable
            if chmod +x "$script_path" 2>/dev/null; then
                echo -e "${GREEN}✅ Fixed! Script is now executable${NC}"
                echo "Re-running the script with proper permissions..."
                echo
                # Re-execute the script with the same arguments
                exec "$script_path" "$@"
            else
                echo -e "${YELLOW}⚠️  Could not fix permissions automatically${NC}"
                echo
                echo "Please run one of these commands instead:"
                echo
                echo -e "${CYAN}Option 1 (Fix permissions):${NC}"
                echo "  chmod +x \"$script_path\""
                echo "  \"$script_path\" $*"
                echo
                echo -e "${CYAN}Option 2 (Run with bash):${NC}"
                echo "  bash \"$script_path\" $*"
                echo
                echo -e "${CYAN}Option 3 (One-liner):${NC}"
                echo "  chmod +x \"$script_path\" && \"$script_path\" $*"
                echo
                exit 1
            fi
        fi
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
    echo "If you see an error or empty result, make sure you're still logged in."
    echo
    read -p "Press Enter when the API page has loaded..."
    echo
}

# Parse JSON with jq if available
parse_with_jq() {
    echo "Great! Let's parse the JSON to find your credentials."
    echo
    echo "Please copy the ENTIRE JSON response from the browser and paste it here."
    echo "Tips:"
    echo "  • Select all with Cmd+A (Mac) or Ctrl+A (Windows/Linux)"
    echo "  • If you see 'null' or '[]', make sure you're logged in"
    echo "  • The JSON should contain device information"
    echo
    echo "Paste the JSON here, then press Ctrl+D (Unix) or Ctrl+Z (Windows) to finish:"
    
    local json_content=""
    while IFS= read -r line; do
        json_content+="$line"$'\n'
    done
    
    if [[ -z "$json_content" ]]; then
        print_error "No JSON content provided"
        return 1
    fi
    
    # Check if JSON is valid and not empty
    if ! echo "$json_content" | jq . >/dev/null 2>&1; then
        print_error "Invalid JSON format"
        return 1
    fi
    
    # Check for common error patterns
    if echo "$json_content" | grep -q '"result":null\|"result":\[\]'; then
        print_error "API returned empty result - make sure you're logged in"
        return 1
    fi
    
    # Try to parse with jq
    local devices
    if devices=$(echo "$json_content" | jq -r '.result[]? | select(.device_id and .user_idx) | "Device: \(.device_name // .device_type // "Unknown") [\(.os // "Unknown OS")] - ID: \(.device_id) - User Index: \(.user_idx)"' 2>/dev/null); then
        if [[ -n "$devices" ]]; then
            echo -e "${GREEN}📱 Found devices:${NC}"
            echo "$devices"
            echo
            
            # Extract and display values separately
            local device_ids user_indices
            
            device_ids=$(echo "$json_content" | jq -r '.result[]? | select(.device_id) | .device_id' 2>/dev/null)
            user_indices=$(echo "$json_content" | jq -r '.result[]? | select(.user_idx) | .user_idx' 2>/dev/null)
            
            if [[ -n "$device_ids" ]]; then
                echo -e "${YELLOW}📋 Device IDs found:${NC}"
                echo "$device_ids" | nl -w2 -s'. '
                echo
            fi
            
            if [[ -n "$user_indices" ]]; then
                echo -e "${YELLOW}📋 User Indices found:${NC}"
                echo "$user_indices" | nl -w2 -s'. '
                echo
            fi
            
            return 0
        fi
    fi
    
    print_error "Could not parse JSON automatically"
    echo "The JSON structure might be different than expected."
    return 1
}

# Manual parsing fallback
parse_manually() {
    echo -e "${YELLOW}📝 Let's find your credentials manually:${NC}"
    echo
    echo "In the JSON you see in your browser, look for patterns like:"
    echo
    echo -e "${CYAN}  \"device_id\": \"abc123def456\",${NC}"
    echo -e "${CYAN}  \"user_idx\": \"789012345\",${NC}"
    echo
    echo "There might be multiple devices listed. Choose the one that corresponds"
    echo "to the device/app you use most often for reading RIDI books."
    echo
    
    # Get device ID
    while true; do
        echo
        read -p "Please enter your device_id: " input_device_id
        
        if [[ -z "$input_device_id" ]]; then
            print_error "Device ID cannot be empty"
            continue
        fi
        
        # Clean up the input (remove quotes, spaces, commas)
        device_id=$(echo "$input_device_id" | sed 's/[",]//g' | tr -d ' ')
        
        if [[ ${#device_id} -lt 10 ]]; then
            print_error "Device ID seems too short (${#device_id} characters). Please double-check."
            echo "  A typical device ID is much longer."
            continue
        fi
        
        break
    done
    
    # Get user index
    while true; do
        echo
        read -p "Please enter your user_idx: " input_user_idx
        
        if [[ -z "$input_user_idx" ]]; then
            print_error "User index cannot be empty"
            continue
        fi
        
        # Clean up the input
        user_idx=$(echo "$input_user_idx" | sed 's/[",]//g' | tr -d ' ')
        
        if [[ ! "$user_idx" =~ ^[0-9]+$ ]]; then
            print_warning "User index should typically be a number, but we'll accept: $user_idx"
        fi
        
        break
    done
    
    echo
    print_success "Credentials collected!"
    echo -e "  Device ID: ${GREEN}$device_id${NC}"
    echo -e "  User Index: ${GREEN}$user_idx${NC}"
    echo
}

# Validate input format
validate_credentials() {
    local errors=0
    
    echo -e "${BLUE}🔍 Validating credential format...${NC}"
    
    # Check device_id format
    if [[ ${#device_id} -lt 20 ]]; then
        print_warning "Device ID seems unusually short (${#device_id} chars)"
        ((errors++))
    fi
    
    # Check user_idx format
    if [[ ! "$user_idx" =~ ^[0-9]+$ ]]; then
        print_warning "User index is not numeric: $user_idx"
        ((errors++))
    fi
    
    if [[ $errors -eq 0 ]]; then
        print_success "Credential format looks good"
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
    echo -e "  Device ID: ${CYAN}$device_id${NC}"
    echo -e "  User Index: ${CYAN}$user_idx${NC}"
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
    has_jq=false
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
                echo
                parse_success=true
            fi
        fi
        
        # Always offer manual parsing as backup
        if ! $parse_success; then
            parse_manually
        else
            echo "Do you want to manually enter different credentials instead?"
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

# First thing: handle permissions before doing anything else
handle_permissions "$@"

# Parse arguments
parse_args "$@"

# Run main script
main