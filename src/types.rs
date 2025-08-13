use std::path::PathBuf;
use std::ffi::OsString;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Enhanced book information with validation and output path tracking
#[derive(Debug, Clone)]
pub struct BookInfo {
    pub format: BookFormat,
    pub id: OsString,
    pub path: PathBuf,
    pub library_root: PathBuf,  // NEW: for calculating output paths
    pub file_size: Option<u64>, // NEW: for progress display
    pub is_decrypted: bool,     // NEW: track decryption status
}

impl BookInfo {
    /// Create new BookInfo with smart detection
    pub fn new(path: PathBuf, library_root: PathBuf) -> Result<Self, ProcessingError> {
        let format = BookFormat::from_path(&path)?;
        let id = path.file_stem()
            .ok_or(ProcessingError::InvalidPath("No file stem found".into()))?
            .to_os_string();
        
        let file_size = std::fs::metadata(&path).ok().map(|m| m.len());
        let is_decrypted = Self::check_if_already_decrypted(&path, &library_root);

        Ok(BookInfo {
            format,
            id,
            path,
            library_root,
            file_size,
            is_decrypted,
        })
    }

    /// Check if this book has already been decrypted
    fn check_if_already_decrypted(&self, library_root: &PathBuf) -> bool {
        let output_path = self.get_output_path();
        output_path.exists() && output_path.is_file()
    }

    /// Calculate the output path for this book
    pub fn get_output_path(&self) -> PathBuf {
        let mut output = self.library_root.clone();
        
        // Add format subdirectory if organizing
        match self.format {
            BookFormat::Epub => output.push("epub"),
            BookFormat::Pdf => output.push("pdf"),
        }

        // Add the filename with proper extension
        let mut filename = self.id.clone();
        filename.push(self.format.extension());
        output.push(filename);

        output
    }

    /// Get human-readable file size
    pub fn format_file_size(&self) -> String {
        match self.file_size {
            Some(size) => format_bytes(size),
            None => "Unknown size".to_string(),
        }
    }
}

/// Book format detection and handling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BookFormat {
    Epub,
    Pdf,
}

impl BookFormat {
    /// Detect format from file path
    pub fn from_path(path: &PathBuf) -> Result<Self, ProcessingError> {
        let ext = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match ext.as_deref() {
            Some("dat") => {
                // For .dat files, we need to guess based on content or naming
                // This is where the original logic would go
                Ok(BookFormat::Epub) // Default assumption
            }
            Some("epub") => Ok(BookFormat::Epub),
            Some("pdf") => Ok(BookFormat::Pdf),
            _ => Err(ProcessingError::UnsupportedFormat(format!("Unknown format: {:?}", ext)))
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            BookFormat::Epub => ".epub",
            BookFormat::Pdf => ".pdf",
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            BookFormat::Epub => "EPUB",
            BookFormat::Pdf => "PDF",
        }
    }
}

/// Enhanced library location with confidence scoring
#[derive(Debug, Clone)]
pub struct LibraryLocation {
    pub path: PathBuf,
    pub confidence: u8,    // 0-100, higher = more reliable
    pub source: String,    // How we found this location
    pub book_count: usize, // Number of books found here
}

impl LibraryLocation {
    pub fn new(path: PathBuf, confidence: u8, source: String) -> Self {
        let book_count = Self::count_books(&path);
        LibraryLocation {
            path,
            confidence,
            source,
            book_count,
        }
    }

    /// Count potential book files in this location
    fn count_books(path: &PathBuf) -> usize {
        if !path.exists() {
            return 0;
        }

        walkdir::WalkDir::new(path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| matches!(ext, "dat" | "epub" | "pdf"))
                    .unwrap_or(false)
            })
            .count()
    }

    /// Check if this location looks valid
    pub fn is_valid(&self) -> bool {
        self.path.exists() && self.book_count > 0
    }
}

/// Configuration management
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub device_id: Option<String>,
    pub user_idx: Option<String>,
    pub verbose: Option<bool>,
    pub organize_output: Option<bool>,
    pub custom_output_path: Option<PathBuf>,
    pub backup_originals: Option<bool>,
}

impl Config {
    /// Load config from ~/.ridiculous.toml
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::ReadError(e.to_string()))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Save config to ~/.ridiculous.toml
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::WriteError(e.to_string()))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        std::fs::write(&config_path, content)
            .map_err(|e| ConfigError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf, ConfigError> {
        dirs::home_dir()
            .map(|home| home.join(".ridiculous.toml"))
            .ok_or(ConfigError::HomeDirectoryNotFound)
    }

    /// Merge with command line arguments
    pub fn merge_with_cli(&mut self, cli_verbose: bool, cli_organize: bool, cli_output: Option<PathBuf>) {
        if cli_verbose {
            self.verbose = Some(true);
        }
        if cli_organize {
            self.organize_output = Some(true);
        }
        if cli_output.is_some() {
            self.custom_output_path = cli_output;
        }
    }
}

/// Enhanced error handling
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Invalid file path: {0}")]
    InvalidPath(String),
    
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionError(String),
    
    #[error("Key extraction failed: {0}")]
    KeyExtractionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not find home directory")]
    HomeDirectoryNotFound,
    
    #[error("Failed to read config: {0}")]
    ReadError(String),
    
    #[error("Failed to parse config: {0}")]
    ParseError(String),
    
    #[error("Failed to write config: {0}")]
    WriteError(String),
    
    #[error("Failed to serialize config: {0}")]
    SerializeError(String),
}

/// Utility function to format byte sizes
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_book_format_detection() {
        let epub_path = PathBuf::from("test.epub");
        assert_eq!(BookFormat::from_path(&epub_path).unwrap(), BookFormat::Epub);
        
        let pdf_path = PathBuf::from("test.pdf");
        assert_eq!(BookFormat::from_path(&pdf_path).unwrap(), BookFormat::Pdf);
    }
}