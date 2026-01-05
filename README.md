# Ridiculous Enhanced

**Buy from RIDI, Read Anywhere!** - An enhanced tool (CLI & GUI) that extracts your purchased ebooks from [RIDI](https://ridi.com/) and converts them into DRM-free files with batch processing, progress tracking, and advanced features.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/Some-0ne/ridiculous)

## 📋 Prerequisites

Before getting started, make sure you have:

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **Git** - For cloning the repository
- **RIDI Account** - With purchased books you want to decrypt
- **RIDI App** - Installed with books downloaded locally

### Platform-Specific Requirements
- **Windows**: No additional requirements
- **macOS**: Xcode Command Line Tools - `xcode-select --install`
- **Linux**: Build essentials - `sudo apt install build-essential` (Ubuntu/Debian)

## ✨ Enhanced Features

### 🔥 Capabilities
- **🖥️ GUI Mode**: User-friendly graphical interface for non-technical users
- **💻 CLI Mode**: Powerful command-line interface for advanced users
- **📚 Batch Processing**: Process multiple books simultaneously with configurable parallelism
- **📊 Progress Tracking**: Real-time progress bars and detailed status reporting
- **🔄 Resume Support**: Continue from where you left off if processing is interrupted
- **🔍 Smart Library Detection**: Automatically finds RIDI installations across all platforms
- **📁 Custom Library Path**: Specify custom book library locations
- **⚙️ Configuration Management**: Save credentials and preferences in config files
- **🛠️ Advanced Diagnostics**: Built-in troubleshooting and system validation
- **🔐 Credential Validation**: Verify your RIDI credentials before processing
- **⚡ Async Architecture**: Fast, efficient processing with proper error handling
- **🛡️ Graceful Shutdown**: Saves progress when interrupted (Ctrl+C)

### 📖 Format Support
- **EPUB** - Complete support with metadata preservation
  - **v1 DRM** - Original RIDI encryption format (full-file encryption)
  - **v11 DRM** - Newer RIDI encryption format (per-file ZIP encryption)
- **PDF** - Full extraction with original formatting

> **Automatic DRM Detection**: The tool automatically detects and handles both v1 and v11 DRM formats. No manual intervention needed!

### 🎯 Core Features
- **Cross-platform**: Windows, macOS, and Linux support
- **Original Decryption**: Uses the proven RIDI decryption algorithm
- **Retry Logic**: Automatic retry for transient failures
- **File Validation**: Ensures successful decryption before completion
- **State Persistence**: Saves progress between sessions

## 🚀 Quick Start

### 1. Installation

#### Option A: Pre-built Binaries
Download from [Releases](https://github.com/Some-0ne/ridiculous/releases) and extract.

#### Option B: Build from Source
```bash
# Clone the repository
git clone https://github.com/Some-0ne/ridiculous.git
cd ridiculous

# Make scripts executable
chmod +x ./scripts/build.sh
chmod +x ./scripts/get_ridi_credentials.sh

# Build with the enhanced build script
./scripts/build.sh --install
```

**If you get "Permission denied":**
```bash
# Alternative: Run scripts with bash (no permissions needed)
bash ./scripts/build.sh --install

# Or fix permissions for all scripts at once
find ./scripts -name "*.sh" -exec chmod +x {} \;
```

#### Option C: Using Cargo
```bash
# Install CLI version directly from source
cargo install --git https://github.com/Some-0ne/ridiculous.git

# Install with GUI support
cargo install --git https://github.com/Some-0ne/ridiculous.git --features gui
```

### 🖥️ GUI Mode (User-Friendly Interface)

For non-technical users, we provide a graphical interface with an easy-to-use workflow.

#### macOS App Bundle (Recommended for macOS users)

Build a native macOS application:

```bash
# Make the script executable (first time only)
chmod +x build_gui_app.sh

# Build the app bundle
./build_gui_app.sh

# Open the app
open Ridiculous.app

# Optional: Install to Applications folder
mv Ridiculous.app /Applications/
```

#### Cross-Platform GUI

For Windows and Linux, or if you prefer running from terminal:

```bash
# Build with GUI feature
cargo build --release --features gui

# Run the GUI
./target/release/ridiculous --gui

# Or using cargo run
cargo run --release --features gui -- --gui
```

#### Using the GUI

The GUI provides an intuitive workflow:

1. **Setup Screen**
   - Enter your Device ID (from RIDI credentials)
   - Enter your User Index (from RIDI credentials)
   - Optionally specify a custom Library Path (or leave empty for auto-detection)
   - Click "🔍 Find Books" to scan your library

2. **Book Selection**
   - View all discovered books with their DRM format (v1 or v11)
   - Select/deselect books using checkboxes
   - Use "Select All" or "Deselect All" for quick selection
   - Click "🔓 Decrypt" to start processing

3. **Progress Tracking**
   - Real-time progress bar showing overall completion
   - Current book being processed
   - Live status updates

4. **Results**
   - Summary of successful and failed decryptions
   - Option to decrypt more books
   - Decrypted files saved to library root folder

**GUI Features:**
- ✨ No command-line experience needed
- 📁 Visual file browser for library path selection
- 📊 Real-time progress tracking with percentage
- ✅ Success/failure indicators for each book
- 🔄 Support for both v1 and v11 DRM formats
- 🎯 Skip already-decrypted books automatically

### 2. Get Your RIDI Credentials

Run the interactive credential setup script:
```bash
./scripts/get_ridi_credentials.sh
```

**If you get "Permission denied":**
```bash
# Alternative: Run with bash (no permissions needed)
bash ./scripts/get_ridi_credentials.sh

# Or fix and run in one command
chmod +x ./scripts/get_ridi_credentials.sh && ./scripts/get_ridi_credentials.sh
```

This script will:
- Open RIDI login page in your browser
- Guide you to the device API endpoint  
- Help extract your `device_id` and `user_idx`
- Optionally save credentials to config file

**Manual Method:**
1. Go to [https://ridibooks.com/account/login](https://ridibooks.com/account/login) and log in
2. Visit [https://account.ridibooks.com/api/user-devices/app](https://account.ridibooks.com/api/user-devices/app)
3. Find the `device_id` and `user_idx` values in the JSON response

### 3. Process Your Books

```bash
# Use this generally to decrypt your books
cargo run

# Batch mode for faster processing (processes all books without prompts)
cargo run -- --batch-mode

# Custom output directory (default: books are placed in their source directories)
cargo run -- --batch-mode --output-dir "/path/to/output"
```

**Default Behavior:**
- Decrypted books are placed in the library root folder
- Example: Books in `/library/12345/` subdirectories are decrypted to `/library/12345.epub`
- All decrypted files in one easy-to-find location
- Original encrypted files remain in their subdirectories


### Troubleshooting
```bash
# Run full diagnostics
ridiculous --diagnose

# Validate credentials only
ridiculous --device-id "abc123..." --user-idx "12345" --validate-only

# Debug mode with maximum verbosity
RUST_LOG=debug ridiculous --device-id "abc123..." --user-idx "12345" --verbose
```

## ⚙️ Configuration

### Config File
Save your credentials and preferences in a config file to avoid typing them each time:

**Location:**
- **Linux/macOS**: `~/.ridiculous.toml`
- **Windows**: `%USERPROFILE%\.ridiculous.toml`

**Sample Configuration:**
```toml
# Ridiculous Enhanced Configuration
device_id = "your_device_id_here"
user_idx = "your_user_idx_here"
verbose = false
organize_output = true
backup_originals = true

# Optional: custom output directory (if not set, files go to library root folder)
# output_directory = "/path/to/your/books"

# Optional: custom library location (if books are not in standard RIDI location)
# library_path = "/custom/path/to/ridi/library"

max_retries = 3
timeout_seconds = 30
```

**Using Config File:**
```bash
# Will automatically use saved credentials
ridiculous --verbose
```

## 🔍 Troubleshooting

### Common Issues

**Script Permission Errors**
```bash
# If ./scripts/get_ridi_credentials.sh gives "Permission denied"
bash ./scripts/get_ridi_credentials.sh

# If ./scripts/build.sh gives "Permission denied"  
bash ./scripts/build.sh --install

# Or fix all script permissions at once
find ./scripts -name "*.sh" -exec chmod +x {} \;
```

**"No books found"**
```bash
# Check system setup
ridiculous --diagnose

# Make sure RIDI app is installed and books are downloaded
# Books should be in the RIDI app's library, not just purchased
```

**"Invalid credentials"**
```bash
# Validate credentials
ridiculous --device-id "your_id" --user-idx "your_idx" --validate-only

# Re-run credential setup
./scripts/get_ridi_credentials.sh
# Or: bash ./scripts/get_ridi_credentials.sh
```

**Processing failures**
```bash
# Use resume to retry failed books
ridiculous --device-id "your_id" --user-idx "your_idx" --resume

# Reduce parallel workers for stability
ridiculous --device-id "your_id" --user-idx "your_idx" --batch-mode --parallel 2
```

**Library not found**
```bash
# Check detected library locations
ridiculous --diagnose

# Make sure you're using the correct user_idx
# Each user has their own library folder (_{user_idx})

# Use custom library path if books are in non-standard location
ridiculous --library-path "/path/to/your/ridi/books" --device-id "your_id" --user-idx "your_idx"
```

### Debug Mode
For detailed debugging information:
```bash
export RUST_LOG=debug
ridiculous --device-id "your_id" --user-idx "your_idx" --verbose
```

## 🏗️ Building from Source

### Build Process
```bash
# Clone and setup
git clone https://github.com/Some-0ne/ridiculous.git
cd ridiculous

# Make build script executable
chmod +x ./scripts/build.sh

# Simple CLI build
cargo build --release

# Enhanced build with optimizations
./scripts/build.sh --install

# Build with GUI support
cargo build --release --features gui

# Build macOS app bundle (macOS only)
chmod +x build_gui_app.sh
./build_gui_app.sh

# Run tests
cargo test
```

**Build Options:**
- **CLI only**: `cargo build --release` - Smallest binary, command-line interface
- **CLI + GUI**: `cargo build --release --features gui` - Includes both modes
- **macOS App**: `./build_gui_app.sh` - Creates `Ridiculous.app` bundle


### Finding and understanding your IDs'



## ⚖️ Legal Disclaimer

**IMPORTANT**: This tool is for **personal use only** with books you have legally purchased.

✅ **Allowed:**
- Decrypt books you have legally purchased
- Create personal backups of your books
- Read your books on different devices/software

❌ **Prohibited:**
- Share, distribute, or sell decrypted books
- Use with books you don't own
- Any form of piracy or copyright infringement

**Use at your own risk.** The developers assume no responsibility for misuse.

## Acknowledgments

- Original [ridiculous](https://github.com/hsj1/ridiculous) project by hsj1
- Rust async ecosystem (tokio, etc.)

---