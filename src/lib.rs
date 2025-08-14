//! # Ridiculous Enhanced
//! 
//! Enhanced RIDI Books DRM removal tool with smart detection, user-friendly interface,
//! and comprehensive error handling.
//! 
//! ## Features
//! 
//! - Smart multi-platform library detection
//! - Interactive setup and configuration management  
//! - Progress bars and user-friendly feedback
//! - Skip already-decrypted books
//! - Comprehensive error handling with helpful messages
//! - Optional GUI wrapper
//! 
//! ## Usage
//! 
//! ```bash
//! # Interactive mode (recommended for first-time users)
//! ridiculous
//! 
//! # With specific credentials
//! ridiculous --device-id YOUR_ID --user-idx YOUR_INDEX
//! 
//! # Diagnostic mode
//! ridiculous --diagnose
//! 
//! # Force re-decrypt all books
//! ridiculous --force --verbose
//! ```

pub mod types;
pub mod library_finder;
pub mod credential_manager;

pub use types::*;
pub use library_finder::LibraryFinder;
pub use credential_manager::CredentialManager;