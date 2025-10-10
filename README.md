# Ridiculous Enhanced

**Buy from RIDI, Read Anywhere!** - An enhanced command-line tool that extracts your purchased ebooks from [RIDI](https://ridi.com/) and converts them into DRM-free files with batch processing, progress tracking, and advanced features.

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
- **📚 Batch Processing**: Process multiple books simultaneously with configurable parallelism
- **📊 Progress Tracking**: Real-time progress bars and detailed status reporting  
- **🔄 Resume Support**: Continue from where you left off if processing is interrupted
- **🔍 Smart Library Detection**: Automatically finds RIDI installations across all platforms
- **⚙️ Configuration Management**: Save credentials and preferences in config files
- **🛠️ Advanced Diagnostics**: Built-in troubleshooting and system validation
- **🔐 Credential Validation**: Verify your RIDI credentials before processing
- **⚡ Async Architecture**: Fast, efficient processing with proper error handling

### 📖 Format Support
- **EPUB** - Complete support with metadata preservation
- **PDF** - Full extraction with original formatting

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
# Install directly from source
cargo install --git https://github.com/Some-0ne/ridiculous.git
```

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
#Use this generally to get your books
cargo run
```


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
output_directory = "/path/to/your/books"
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

# Simple build
cargo build --release

# Enhanced build with optimizations
./scripts/build.sh --install

# Run tests
cargo test

# Build with GUI support (future feature)
cargo build --release --features gui
```

**If build script won't run:**
```bash
# Run with bash instead
bash ./scripts/build.sh --install

# Or fix permissions first
chmod +x ./scripts/build.sh && ./scripts/build.sh --install
```

### Development
```bash
# Run with debug output
cargo run -- --device-id "your_id" --user-idx "your_idx" --verbose

# Run tests
cargo test

# Check code format
cargo fmt --check

# Run linter
cargo clippy
```

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