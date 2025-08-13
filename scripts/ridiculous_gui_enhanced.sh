#!/bin/bash

# GUI Wrapper for Ridiculous Enhanced
# Provides a user-friendly graphical interface for macOS and Linux

set -e

# Colors and formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Configuration
RIDICULOUS_BIN="ridiculous"
CONFIG_FILE="$HOME/.ridiculous.toml"

# Check if we're on macOS for native dialogs
is_macos() {
    [[ "$(uname -s)" == "Darwin" ]]
}

# Show native dialog on macOS
show_macos_dialog() {
    local dialog_type="$1"
    local title="$2"
    local message="$3"
    local buttons="${4:-OK}"
    
    osascript << EOF
tell application "System Events"
    display dialog "$message" with title "$title" buttons {"$buttons"} default button 1 with icon $dialog_type
    return button returned of result
end tell
EOF
}

# Show notification
show_notification() {
    local title="$1"
    local message="$2"
    
    if is_macos; then
        osascript -e "display notification \"$message\" with title \"$title\""
    elif command -v notify-send &> /dev/null; then
        notify-send "$title" "$message"
    else
        echo "🔔 $title: $message"
    fi
}

# Show error dialog
show_error() {
    local title="$1"
    local message="$2"
    
    if is_macos; then
        show_macos_dialog "stop" "$title" "$message"
    else
        echo -e "${RED}❌ $title${NC}"
        echo -e "$message"
        read -p "Press Enter to continue..."
    fi
}

# Show info dialog
show_info() {
    local title="$1"
    local message="$2"
    
    if is_macos; then
        show_macos_dialog "note" "$title" "$message"
    else
        echo -e "${BLUE}ℹ️  $title${NC}"
        echo -e "$message"
        read -p "Press Enter to continue..."
    fi
}

# Show success dialog
show_success() {
    local title="$1"
    local message="$2"
    
    if is_macos; then
        show_macos_dialog "note" "$title" "$message"
    else
        echo -e "${GREEN}✅ $title${NC}"
        echo -e "$message"
        read -p "Press Enter to continue..."
    fi
}

# Ask yes/no question
ask_yes_no() {
    local title="$1"
    local message="$2"
    
    if is_macos; then
        local result=$(show_macos_dialog "note" "$title" "$message" "No,Yes")
        [[ "$result" == "Yes" ]]
    else
        echo -e "${YELLOW}❓ $title${NC}"
        echo -e "$message"
        while true; do
            read -p "Continue? (y/n): " yn
            case $yn in
                [Yy]* ) return 0;;
                [Nn]* ) return 1;;
                * ) echo "Please answer yes or no.";;
            esac
        done
    fi
}

# Get text input
get_text_input() {
    local title="$1"
    local prompt="$2"
    local default="$3"
    
    if is_macos; then
        osascript << EOF
tell application "System Events"
    display dialog "$prompt" with title "$title" default answer "$default"
    return text returned of result
end tell
EOF
    else
        echo -e "${BLUE}📝 $title${NC}"
        read -p "$prompt: " -i "$default" -e input
        echo "$input"
    fi
}

# Check if ridiculous binary exists
check_ridiculous() {
    if ! command -v "$RIDICULOUS_BIN" &> /dev/null; then
        show_error "Ridiculous Not Found" "The ridiculous binary was not found in your PATH.\n\nPlease make sure it's installed and accessible.\n\nYou can install it by running: ./build.sh"
        exit 1
    fi
}

# Check configuration
check_config() {
    if [[ -f "$CONFIG_FILE" ]]; then
        return 0
    else
        return 1
    fi
}

# First-time setup wizard
setup_wizard() {
    show_info "Welcome to Ridiculous Enhanced!" "This appears to be your first time using Ridiculous Enhanced.\n\nWe'll guide you through the initial setup process."
    
    if ask_yes_no "Setup Device Credentials" "Do you want to set up your RIDI device credentials now?\n\nThis is required to decrypt your books."; then
        if command -v get_device_id &> /dev/null; then
            # Run device ID helper in a terminal
            if is_macos; then
                osascript -e "tell application \"Terminal\" to do script \"get_device_id; exit\""
            else
                x-terminal-emulator -e "get_device_id" || gnome-terminal -- get_device_id || xterm -e get_device_id
            fi
            
            show_info "Device Setup" "The device ID setup should have opened in a terminal window.\n\nPlease follow the instructions there, then return here when complete."
        else
            show_error "Setup Helper Not Found" "The device ID setup helper was not found.\n\nPlease run 'get_device_id' manually in a terminal."
        fi
    fi
}

# Main menu
show_main_menu() {
    clear
    echo -e "${CYAN}${BOLD}"
    echo "🚀 ═══════════════════════════════════════════════════════════"
    echo "   Ridiculous Enhanced - GUI Interface"
    echo "   ═══════════════════════════════════════════════════════════"
    echo -e "${NC}"
    echo
    echo -e "${YELLOW}📚 Main Menu:${NC}"
    echo
    echo "1. 📖 Process Books (Decrypt DRM)"
    echo "2. 🔍 Diagnose System"
    echo "3. ⚙️  Configuration"
    echo "4. 🔑 Setup Device Credentials"
    echo "5. ❓ Help & About"
    echo "6. 🚪 Exit"
    echo
    
    read -p "Choose an option (1-6): " choice
    
    case $choice in
        1) process_books_menu ;;
        2) run_diagnostics_gui ;;
        3) configuration_menu ;;
        4) setup_credentials ;;
        5) show_help ;;
        6) exit 0 ;;
        *) 
            echo -e "${RED}Invalid option. Please choose 1-6.${NC}"
            sleep 1
            show_main_menu
            ;;
    esac
}

# Process books menu
process_books_menu() {
    clear
    echo -e "${GREEN}${BOLD}📖 Process Books${NC}"
    echo
    echo "Choose processing options:"
    echo
    echo "1. 🔄 Smart Process (skip already decrypted)"
    echo "2. 🔁 Force Re-process All Books"
    echo "3. ✅ Validate Files Only (no decryption)"
    echo "4. 🔧 Advanced Options"
    echo "5. ↩️  Back to Main Menu"
    echo
    
    read -p "Choose an option (1-5): " choice
    
    case $choice in
        1) run_ridiculous "" ;;
        2) run_ridiculous "--force" ;;
        3) run_ridiculous "--validate-only" ;;
        4) advanced_processing_menu ;;
        5) show_main_menu ;;
        *)
            echo -e "${RED}Invalid option. Please choose 1-5.${NC}"
            sleep 1
            process_books_menu
            ;;
    esac
}

# Advanced processing options
advanced_processing_menu() {
    clear
    echo -e "${BLUE}${BOLD}🔧 Advanced Processing Options${NC}"
    echo
    
    local options=""
    local verbose=false
    local organize=false
    local custom_output=""
    
    echo "Configure processing options:"
    echo
    
    # Verbose output
    if ask_yes_no "Verbose Output" "Enable detailed progress information?"; then
        verbose=true
        options="$options --verbose"
    fi
    
    # Organize output
    if ask_yes_no "Organize Output" "Create separate epub/ and pdf/ folders?"; then
        organize=true
        options="$options --organize"
    fi
    
    # Custom output path
    if ask_yes_no "Custom Output Path" "Specify a custom output directory?"; then
        custom_output=$(get_text_input "Output Directory" "Enter output path" "$HOME/Documents/RidiBooks")
        if [[ -n "$custom_output" ]]; then
            options="$options --output \"$custom_output\""
        fi
    fi
    
    # Force re-process
    if ask_yes_no "Force Re-process" "Re-decrypt all books (ignore already decrypted)?"; then
        options="$options --force"
    fi
    
    echo
    echo -e "${YELLOW}Selected options:${NC} $options"
    echo
    
    if ask_yes_no "Confirm Processing" "Start processing with these options?"; then
        run_ridiculous "$options"
    else
        process_books_menu
    fi
}

# Run ridiculous with options
run_ridiculous() {
    local options="$1"
    
    show_notification "Ridiculous Enhanced" "Starting book processing..."
    
    clear
    echo -e "${CYAN}${BOLD}🚀 Running Ridiculous Enhanced${NC}"
    echo
    echo -e "${YELLOW}Command:${NC} ridiculous $options"
    echo
    echo "Press Ctrl+C to cancel"
    echo
    echo "═══════════════════════════════════════════════════════════"
    echo
    
    # Run the command
    if eval "$RIDICULOUS_BIN $options"; then
        show_notification "Ridiculous Enhanced" "Processing completed successfully!"
        show_success "Processing Complete" "Your books have been processed successfully.\n\nCheck your library directory for the decrypted files."
    else
        show_notification "Ridiculous Enhanced" "Processing failed!"
        show_error "Processing Failed" "An error occurred during processing.\n\nCheck the terminal output for details."
    fi
    
    echo
    read -p "Press Enter to return to main menu..."
    show_main_menu
}

# Run diagnostics
run_diagnostics_gui() {
    clear
    echo -e "${BLUE}${BOLD}🔍 System Diagnostics${NC}"
    echo
    echo "Running diagnostic checks..."
    echo
    echo "═══════════════════════════════════════════════════════════"
    echo
    
    $RIDICULOUS_BIN --diagnose
    
    echo
    echo "═══════════════════════════════════════════════════════════"
    echo
    read -p "Press Enter to return to main menu..."
    show_main_menu
}

# Configuration menu
configuration_menu() {
    clear
    echo -e "${YELLOW}${BOLD}⚙️  Configuration${NC}"
    echo
    
    if check_config; then
        echo -e "${GREEN}✅ Configuration file found:${NC} $CONFIG_FILE"
        echo
        
        # Show current config
        echo -e "${CYAN}Current configuration:${NC}"
        echo "─────────────────────────"
        cat "$CONFIG_FILE" | grep -v '^#' | grep -v '^$' || echo "No configuration found"
        echo "─────────────────────────"
        echo
        
        echo "Configuration options:"
        echo "1. 👀 View Full Configuration"
        echo "2. 🔑 Update Device Credentials"
        echo "3. 🗑️  Delete Configuration (reset)"
        echo "4. 💾 Save Current CLI Settings"
        echo "5. ↩️  Back to Main Menu"
        echo
        
        read -p "Choose an option (1-5): " choice
        
        case $choice in
            1) view_config ;;
            2) setup_credentials ;;
            3) delete_config ;;
            4) save_cli_config ;;
            5) show_main_menu ;;
            *)
                echo -e "${RED}Invalid option.${NC}"
                sleep 1
                configuration_menu
                ;;
        esac
    else
        echo -e "${YELLOW}⚠️  No configuration file found${NC}"
        echo
        echo "Would you like to:"
        echo "1. 🔑 Setup Device Credentials"
        echo "2. 📝 Create Basic Configuration"
        echo "3. ↩️  Back to Main Menu"
        echo
        
        read -p "Choose an option (1-3): " choice
        
        case $choice in
            1) setup_credentials ;;
            2) create_basic_config ;;
            3) show_main_menu ;;
            *)
                echo -e "${RED}Invalid option.${NC}"
                sleep 1
                configuration_menu
                ;;
        esac
    fi
}

# View full configuration
view_config() {
    clear
    echo -e "${CYAN}${BOLD}📄 Full Configuration${NC}"
    echo
    echo -e "${YELLOW}File:${NC} $CONFIG_FILE"
    echo
    echo "═══════════════════════════════════════════════════════════"
    cat "$CONFIG_FILE" 2>/dev/null || echo "Configuration file not found"
    echo "═══════════════════════════════════════════════════════════"
    echo
    read -p "Press Enter to return..."
    configuration_menu
}

# Delete configuration
delete_config() {
    if ask_yes_no "Delete Configuration" "Are you sure you want to delete your configuration?\n\nThis will remove your saved device credentials and settings."; then
        if [[ -f "$CONFIG_FILE" ]]; then
            rm "$CONFIG_FILE"
            show_success "Configuration Deleted" "Your configuration has been deleted.\n\nYou'll need to set up your credentials again."
        else
            show_info "No Configuration" "No configuration file found to delete."
        fi
    fi
    configuration_menu
}

# Create basic configuration
create_basic_config() {
    cat > "$CONFIG_FILE" << EOF
# Ridiculous Enhanced Configuration
# Created on $(date)

# Device credentials (required)
# Get these from: https://account.ridibooks.com/api/user-devices/app
device_id = ""
user_idx = ""

# Processing options
verbose = false
organize_output = false
backup_originals = false

# Custom output path (optional)
# custom_output_path = "/path/to/output"
EOF

    show_success "Configuration Created" "A basic configuration file has been created at:\n$CONFIG_FILE\n\nYou can now edit it to add your credentials."
    configuration_menu
}

# Save CLI configuration
save_cli_config() {
    if ask_yes_no "Save CLI Settings" "This will save your current command-line preferences to the configuration file.\n\nContinue?"; then
        $RIDICULOUS_BIN --save-config
        show_success "Settings Saved" "Your current CLI settings have been saved to the configuration file."
    fi
    configuration_menu
}

# Setup credentials
setup_credentials() {
    clear
    echo -e "${GREEN}${BOLD}🔑 Setup Device Credentials${NC}"
    echo
    
    if ask_yes_no "Device Credential Setup" "Do you want to use the interactive device ID helper?\n\nThis will guide you through getting your credentials from RIDI."; then
        if command -v get_device_id &> /dev/null; then
            # Run in terminal
            if is_macos; then
                osascript -e "tell application \"Terminal\" to activate"
                osascript -e "tell application \"Terminal\" to do script \"get_device_id\""
            else
                # Try various terminal emulators
                if command -v gnome-terminal &> /dev/null; then
                    gnome-terminal -- get_device_id
                elif command -v xterm &> /dev/null; then
                    xterm -e get_device_id
                elif command -v konsole &> /dev/null; then
                    konsole -e get_device_id
                else
                    echo "Please run 'get_device_id' in your terminal"
                fi
            fi
            
            show_info "Credential Setup" "The device credential setup has been started in a terminal.\n\nPlease complete the setup there, then return here."
        else
            show_error "Helper Not Found" "Device ID helper script not found.\n\nPlease run 'get_device_id' manually in a terminal."
        fi
    else
        # Manual entry
        manual_credential_entry
    fi
    
    show_main_menu
}

# Manual credential entry
manual_credential_entry() {
    echo -e "${YELLOW}📝 Manual Credential Entry${NC}"
    echo
    echo "You'll need to get your credentials from:"
    echo "https://account.ridibooks.com/api/user-devices/app"
    echo
    
    local device_id=$(get_text_input "Device ID" "Enter your device_id" "")
    local user_idx=$(get_text_input "User Index" "Enter your user_idx" "")
    
    if [[ -n "$device_id" && -n "$user_idx" ]]; then
        # Create or update config
        cat > "$CONFIG_FILE" << EOF
# Ridiculous Enhanced Configuration
# Updated on $(date)

device_id = "$device_id"
user_idx = "$user_idx"
verbose = false
organize_output = false
backup_originals = false
EOF
        
        show_success "Credentials Saved" "Your device credentials have been saved to the configuration file."
    else
        show_error "Invalid Input" "Both device ID and user index are required."
    fi
}

# Show help
show_help() {
    clear
    echo -e "${CYAN}${BOLD}❓ Help & About${NC}"
    echo
    echo -e "${GREEN}Ridiculous Enhanced v2.0${NC}"
    echo "Enhanced RIDI Books DRM removal tool"
    echo
    echo -e "${YELLOW}📚 What it does:${NC}"
    echo "• Removes DRM from your purchased RIDI books"
    echo "• Converts encrypted .dat files to readable EPUB/PDF"
    echo "• Smart detection of library locations"
    echo "• Skips already processed books"
    echo "• User-friendly progress tracking"
    echo
    echo -e "${YELLOW}🔧 Requirements:${NC}"
    echo "• RIDI app installed and logged in"
    echo "• Books downloaded in RIDI app"
    echo "• Valid device credentials from RIDI API"
    echo
    echo -e "${YELLOW}⚖️  Legal Notice:${NC}"
    echo "• Only for personal use of books you own"
    echo "• Do not share or distribute processed books"
    echo "• Respect copyright laws in your jurisdiction"
    echo
    echo -e "${YELLOW}🆘 Getting Help:${NC}"
    echo "• Run 'Diagnose System' for troubleshooting"
    echo "• Check that RIDI is properly installed"
    echo "• Ensure you have downloaded books in RIDI app"
    echo "• Verify your device credentials are correct"
    echo
    echo -e "${YELLOW}🌐 Online Resources:${NC}"
    echo "• Device credentials: https://account.ridibooks.com/api/user-devices/app"
    echo "• Original project: https://github.com/hsj1/ridiculous"
    echo
    
    read -p "Press Enter to return to main menu..."
    show_main_menu
}

# Check for updates (placeholder)
check_updates() {
    echo -e "${BLUE}🔄 Checking for updates...${NC}"
    echo
    echo "Update checking is not yet implemented."
    echo "Please check the project repository for updates."
    echo
    read -p "Press Enter to continue..."
}

# Error handler
handle_error() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        show_error "Unexpected Error" "An unexpected error occurred (exit code: $exit_code).\n\nPlease try running the command manually for more details."
    fi
}

# Main execution
main() {
    # Set up error handling
    trap handle_error EXIT
    
    # Check requirements
    check_ridiculous
    
    # Check if this is first run
    if ! check_config; then
        setup_wizard
    fi
    
    # Show main menu
    show_main_menu
}

# Handle interrupts
trap 'echo; echo "GUI interrupted by user"; exit 0' INT

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi