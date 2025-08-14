#!/bin/bash

# Self-permission check - make script executable if it isn't
if [[ ! -x "$0" ]]; then
    echo "🔧 Making script executable..."
    chmod +x "$0"
    exec "$0" "$@"
fi


# Build and Install Script for Ridiculous Enhanced
# One-click setup for macOS and Linux

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Configuration
PROJECT_NAME="ridiculous"
APP_NAME="Ridiculous"
INSTALL_DIR="$HOME/.local/bin"
GUI_SCRIPTS_DIR="$HOME/.local/share/ridiculous"

print_header() {
    echo -e "${CYAN}${BOLD}"
    echo "🔨 ═══════════════════════════════════════════════════════════"
    echo "   Ridiculous Enhanced - Build & Install Script"
    echo "   ═══════════════════════════════════════════════════════════"
    echo -e "${NC}"
    echo
}

print_step() {
    echo -e "${BLUE}${BOLD}🔧 Step $1:${NC} $2"
}

print_success() {
    echo -e "${GREEN}✅${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC} $1"
}

print_error() {
    echo -e "${RED}❌${NC} $1"
}

# Check system requirements
check_requirements() {
    print_step "1" "Checking system requirements..."
    
    # Check for Rust
    if ! command -v rustc &> /dev/null; then
        print_error "Rust is not installed!"
        echo
        echo "Please install Rust from: https://rustup.rs/"
        echo "Then run this script again."
        exit 1
    fi
    
    local rust_version=$(rustc --version)
    print_success "Rust found: $rust_version"
    
    # Check for Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed!"
        echo "Cargo should come with Rust. Please reinstall Rust."
        exit 1
    fi
    
    print_success "Cargo found: $(cargo --version)"
    
    # Check OS
    case "$(uname -s)" in
        Darwin)
            OS="macOS"
            print_success "Operating System: macOS"
            ;;
        Linux)
            OS="Linux"
            print_success "Operating System: Linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            OS="Windows"
            print_success "Operating System: Windows"
            print_warning "Windows support is experimental"
            ;;
        *)
            OS="Unknown"
            print_warning "Unknown operating system: $(uname -s)"
            echo "Proceeding anyway, but some features may not work."
            ;;
    esac
    
    echo
}

# Clean previous builds
clean_build() {
    print_step "2" "Cleaning previous builds..."
    
    if [[ -d "target" ]]; then
        print_success "Removing target directory..."
        rm -rf target
    fi
    
    if [[ -f "Cargo.lock" ]]; then
        print_success "Removing Cargo.lock..."
        rm -f Cargo.lock
    fi
    
    echo
}

# Build the project
build_project() {
    print_step "3" "Building project in release mode..."
    
    echo "This may take a few minutes on first build..."
    echo
    
    if cargo build --release; then
        print_success "Build completed successfully!"
        
        # Check if binary exists
        if [[ -f "target/release/$PROJECT_NAME" ]]; then
            local binary_size=$(du -h "target/release/$PROJECT_NAME" | cut -f1)
            print_success "Binary created: target/release/$PROJECT_NAME ($binary_size)"
        else
            print_error "Binary not found after build!"
            exit 1
        fi
    else
        print_error "Build failed!"
        echo
        echo "Common solutions:"
        echo "1. Make sure all dependencies are available"
        echo "2. Check that you have internet connection for downloading crates"
        echo "3. Try: cargo clean && cargo build --release"
        exit 1
    fi
    
    echo
}

# Install binary and scripts
install_files() {
    print_step "4" "Installing files..."
    
    # Create install directories
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$GUI_SCRIPTS_DIR"
    
    # Install main binary
    if cp "target/release/$PROJECT_NAME" "$INSTALL_DIR/"; then
        print_success "Installed binary to: $INSTALL_DIR/$PROJECT_NAME"
        chmod +x "$INSTALL_DIR/$PROJECT_NAME"
    else
        print_error "Failed to install binary"
        exit 1
    fi
    
    # Install helper scripts
    if [[ -f "get_device_id.sh" ]]; then
        cp "get_device_id.sh" "$GUI_SCRIPTS_DIR/"
        chmod +x "$GUI_SCRIPTS_DIR/get_device_id.sh"
        print_success "Installed device ID helper"
    fi
    
    # Create convenience symlink in install directory
    if [[ -f "$GUI_SCRIPTS_DIR/get_device_id.sh" ]]; then
        ln -sf "$GUI_SCRIPTS_DIR/get_device_id.sh" "$INSTALL_DIR/get_device_id"
        print_success "Created get_device_id command"
    fi
    
    echo
}

# Create macOS app bundle
create_macos_app() {
    if [[ "$OS" != "macOS" ]]; then
        return
    fi
    
    print_step "5" "Creating macOS App Bundle..."
    
    local app_dir="$APP_NAME.app"
    local contents_dir="$app_dir/Contents"
    local macos_dir="$contents_dir/MacOS"
    local resources_dir="$contents_dir/Resources"
    
    # Create app structure
    mkdir -p "$macos_dir"
    mkdir -p "$resources_dir"
    
    # Copy binary
    cp "$INSTALL_DIR/$PROJECT_NAME" "$macos_dir/"
    
    # Create Info.plist
    cat > "$contents_dir/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$PROJECT_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.ridiculous.enhanced</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>Ridiculous Enhanced</string>
    <key>CFBundleVersion</key>
    <string>2.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>2.0</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.14</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
</dict>
</plist>
EOF
    
    # Create wrapper script for GUI
    cat > "$macos_dir/$PROJECT_NAME" << 'EOF'
#!/bin/bash

# macOS App Bundle wrapper for Ridiculous Enhanced

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUNDLE_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Check if we should launch GUI or terminal
if [[ "$1" == "--gui" ]] || [[ -z "$1" && -t 1 ]]; then
    # Terminal is available, run CLI interface
    exec "$SCRIPT_DIR/ridiculous-cli" "$@"
else
    # Try to launch in Terminal.app
    osascript -e "
        tell application \"Terminal\"
            activate
            do script \"cd '$BUNDLE_DIR' && ./Contents/MacOS/ridiculous-cli --gui\"
        end tell
    " 2>/dev/null || {
        # Fallback: run directly
        exec "$SCRIPT_DIR/ridiculous-cli" "$@"
    }
fi
EOF
    
    # Copy actual binary with different name
    cp "$INSTALL_DIR/$PROJECT_NAME" "$macos_dir/ridiculous-cli"
    chmod +x "$macos_dir/$PROJECT_NAME"
    chmod +x "$macos_dir/ridiculous-cli"
    
    print_success "Created macOS App Bundle: $app_dir"
    print_success "You can now drag this to /Applications/"
    
    echo
}

# Setup shell integration
setup_shell() {
    print_step "6" "Setting up shell integration..."
    
    # Check if install directory is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        print_warning "$INSTALL_DIR is not in your PATH"
        echo
        echo "To use 'ridiculous' command from anywhere, add this line to your shell config:"
        echo
        case "$SHELL" in
            */bash)
                echo -e "${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc${NC}"
                ;;
            */zsh)
                echo -e "${CYAN}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc${NC}"
                ;;
            */fish)
                echo -e "${CYAN}echo 'set -gx PATH \$HOME/.local/bin \$PATH' >> ~/.config/fish/config.fish${NC}"
                ;;
            *)
                echo -e "${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
                echo "(Add to your shell's configuration file)"
                ;;
        esac
        echo
        echo "Then restart your terminal or run: source ~/.bashrc (or appropriate file)"
    else
        print_success "Install directory is already in PATH"
    fi
    
    echo
}

# Run tests
run_tests() {
    print_step "7" "Running tests..."
    
    if cargo test --release; then
        print_success "All tests passed!"
    else
        print_warning "Some tests failed, but installation can continue"
    fi
    
    echo
}

# Print final instructions
print_final_instructions() {
    echo -e "${GREEN}${BOLD}🎉 Installation Complete!${NC}"
    echo
    echo -e "${YELLOW}📋 What was installed:${NC}"
    echo "  • Main binary: $INSTALL_DIR/$PROJECT_NAME"
    echo "  • Device ID helper: $INSTALL_DIR/get_device_id"
    echo "  • GUI scripts: $GUI_SCRIPTS_DIR/"
    
    if [[ "$OS" == "macOS" && -d "$APP_NAME.app" ]]; then
        echo "  • macOS App Bundle: ./$APP_NAME.app"
    fi
    
    echo
    echo -e "${YELLOW}🚀 Quick Start:${NC}"
    echo
    echo "1. Get your RIDI credentials:"
    echo -e "   ${CYAN}get_device_id${NC}"
    echo
    echo "2. Start processing books:"
    echo -e "   ${CYAN}ridiculous${NC}"
    echo
    echo "3. For help and options:"
    echo -e "   ${CYAN}ridiculous --help${NC}"
    echo
    echo "4. Run diagnostics if you have issues:"
    echo -e "   ${CYAN}ridiculous --diagnose${NC}"
    echo
    
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo -e "${YELLOW}⚠️  Note:${NC} Add $INSTALL_DIR to your PATH to use commands from anywhere"
        echo
    fi
    
    echo -e "${GREEN}✨ Happy reading with your DRM-free books!${NC}"
    echo
}

# Main execution
main() {
    clear
    print_header
    
    echo "This script will build and install Ridiculous Enhanced."
    echo "The process includes:"
    echo "  • Building the Rust binary in release mode"
    echo "  • Installing to ~/.local/bin/"
    echo "  • Setting up helper scripts"
    if [[ "$(uname -s)" == "Darwin" ]]; then
        echo "  • Creating macOS App Bundle"
    fi
    echo
    
    read -p "Continue with installation? (Y/N): " confirm
    case $confirm in
        [Yy]* ) ;;
        * ) echo "Installation cancelled."; exit 0;;
    esac
    
    echo
    
    # Run all steps
    check_requirements
    clean_build
    build_project
    run_tests
    install_files
    create_macos_app
    setup_shell
    print_final_instructions
}

# Handle interrupts
trap 'echo; print_error "Installation interrupted"; exit 1' INT

# Run main function
main "$@"