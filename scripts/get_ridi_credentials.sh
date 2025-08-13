#!/bin/bash

# Device ID Helper Script for Ridiculous Enhanced
# Helps users get their RIDI device ID and user index

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Print colored header
print_header() {
    echo -e "${CYAN}

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
    
    # Step 2: Get device info
    get_device_info
    
    # Step 3: Parse JSON
    if $has_jq; then
        if parse_with_jq; then
            echo -e "${GREEN}✅ Successfully parsed device information!${NC}"
            echo
            echo "You can use any of the device IDs and user indices shown above."
            echo "Typically, choose the device you use most often for reading."
            echo
            parse_manually
        else
            parse_manually
        fi
    else
        parse_manually
    fi
    
    # Step 4: Test credentials
    if [[ -n "$device_id" && -n "$user_idx" ]]; then
        test_credentials
        save_config
        print_final_instructions
    else
        print_error "Could not obtain valid credentials"
        exit 1
    fi
}

# Handle interrupts gracefully
trap 'echo; print_error "Script interrupted by user"; exit 1' INT

# Check if script is being run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi${BOLD}"
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
    
    # Try to open the URL
    if command -v open &> /dev/null; then
        # macOS
        open "$login_url"
    elif command -v xdg-open &> /dev/null; then
        # Linux
        xdg-open "$login_url"
    elif command -v start &> /dev/null; then
        # Windows (Git Bash, WSL)
        start "$login_url"
    else
        print_warning "Could not automatically open browser"
        echo "  Please manually open: $login_url"
    fi
    
    echo -e "🌐 Login URL: ${CYAN}$login_url${NC}"
    echo
    echo "Please log in to your RIDI account in the browser."
    echo
    read -p "Press Enter when you have successfully logged in..."
    echo
}

# Open API endpoint and get JSON
get_device_info() {
    print_step "2" "Opening RIDI API endpoint to get device information..."
    
    local api_url="https://account.ridibooks.com/api/user-devices/app"
    
    # Try to open the API URL
    if command -v open &> /dev/null; then
        open "$api_url"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "$api_url"
    elif command -v start &> /dev/null; then
        start "$api_url"
    else
        print_warning "Could not automatically open browser"
        echo "  Please manually open: $api_url"
    fi
    
    echo -e "🔗 API URL: ${CYAN}$api_url${NC}"
    echo
    echo "This page will show JSON data with your device information."
    echo "Look for entries that contain 'device_id' and 'user_idx' fields."
    echo
    read -p "Press Enter when the API page has loaded..."
    echo
}

# Parse JSON with jq if available
parse_with_jq() {
    echo "Great! Let's parse the JSON to find your credentials."
    echo
    echo "Please copy the ENTIRE JSON response from the browser and paste it here."
    echo "Tip: You can usually select all with Cmd+A (Mac) or Ctrl+A (Windows/Linux)"
    echo
    echo "Paste the JSON here (press Enter when done, Ctrl+D to finish):"
    
    local json_content=""
    while IFS= read -r line; do
        json_content+="$line"
    done
    
    if [[ -z "$json_content" ]]; then
        print_error "No JSON content provided"
        return 1
    fi
    
    # Try to parse with jq
    local devices
    if devices=$(echo "$json_content" | jq -r '.result[]? | select(.device_id and .user_idx) | "Device: \(.device_name // "Unknown") - ID: \(.device_id) - User Index: \(.user_idx)"' 2>/dev/null); then
        if [[ -n "$devices" ]]; then
            echo -e "${GREEN}📱 Found devices:${NC}"
            echo "$devices"
            echo
            
            # Extract just the values for easy copying
            echo -e "${YELLOW}📋 Device IDs found:${NC}"
            echo "$json_content" | jq -r '.result[]? | select(.device_id) | .device_id' 2>/dev/null
            echo
            
            echo -e "${YELLOW}📋 User Indices found:${NC}"
            echo "$json_content" | jq -r '.result[]? | select(.user_idx) | .user_idx' 2>/dev/null
            echo
            
            return 0
        fi
    fi
    
    print_error "Could not parse JSON automatically"
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
        read -p "Please enter your device_id (the long string after \"device_id\":): " device_id
        
        if [[ -z "$device_id" ]]; then
            print_error "Device ID cannot be empty"
            continue
        fi
        
        # Clean up the input (remove quotes, spaces)
        device_id=$(echo "$device_id" | sed 's/[",]//g' | tr -d ' ')
        
        if [[ ${#device_id} -lt 10 ]]; then
            print_error "Device ID seems too short. Please double-check."
            continue
        fi
        
        break
    done
    
    # Get user index
    while true; do
        echo
        read -p "Please enter your user_idx (usually a number): " user_idx
        
        if [[ -z "$user_idx" ]]; then
            print_error "User index cannot be empty"
            continue
        fi
        
        # Clean up the input
        user_idx=$(echo "$user_idx" | sed 's/[",]//g' | tr -d ' ')
        
        if [[ ! "$user_idx" =~ ^[0-9]+$ ]]; then
            print_warning "User index should typically be a number, but we'll accept what you provided"
        fi
        
        break
    done
    
    echo
    print_success "Credentials collected!"
    echo -e "  Device ID: ${GREEN}$device_id${NC}"
    echo -e "  User Index: ${GREEN}$user_idx${NC}"
    echo
}

# Test credentials
test_credentials() {
    echo -e "${BLUE}🧪 Testing credentials...${NC}"
    echo
    
    # Try to run ridiculous in validation mode
    if command -v ridiculous &> /dev/null; then
        echo "Testing with ridiculous --validate-only..."
        if ridiculous --device-id "$device_id" --user-idx "$user_idx" --validate-only 2>/dev/null; then
            print_success "Credentials appear to be valid!"
        else
            print_warning "Could not validate credentials automatically"
            echo "  This might be normal if you haven't downloaded any books yet"
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