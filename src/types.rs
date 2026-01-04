use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::ffi::OsString;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]  // ← Added this for automatic defaults on missing fields
pub struct Config {
    pub device_id: String,
    pub user_idx: String,
    pub verbose: bool,
    pub organize_output: bool,
    pub backup_originals: bool,
    pub output_directory: Option<String>,
    pub library_path: Option<String>,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            user_idx: String::new(),
            verbose: false,
            organize_output: false,
            backup_originals: true,
            output_directory: None,
            library_path: None,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BookInfo {
    pub id: String,
    pub format: BookFormat,
    pub path: PathBuf, // Directory containing the book files
    pub title: Option<String>,
    pub book_filename: String, // Actual filename (may include version like .v11.epub)
}

impl BookInfo {
    pub fn new(book_dir: PathBuf) -> miette::Result<Self> {
        let id = book_dir.file_name()
            .ok_or_else(|| miette::miette!("Invalid book directory"))?
            .to_string_lossy()
            .to_string();

        let (format, book_filename) = Self::detect_format_and_filename(&book_dir, &id)?;

        Ok(Self {
            id,
            format,
            path: book_dir,
            title: None,
            book_filename,
        })
    }
    
    fn detect_format_and_filename(book_dir: &PathBuf, book_id: &str) -> miette::Result<(BookFormat, String)> {
        // Try to find the actual book file in the directory
        // Files can be named {id}.epub or {id}.v*.epub (versioned)
        for entry in std::fs::read_dir(book_dir).map_err(|e| miette::miette!("Cannot read book directory: {}", e))? {
            let entry = entry.map_err(|e| miette::miette!("Directory entry error: {}", e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();

                    // Check if it's a book file (starts with book_id and ends with .epub or .pdf)
                    if filename_str.starts_with(book_id) {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            match ext_str.as_str() {
                                "epub" => return Ok((BookFormat::Epub, filename_str.to_string())),
                                "pdf" => return Ok((BookFormat::Pdf, filename_str.to_string())),
                                _ => continue,
                            }
                        }
                    }
                }
            }
        }

        // If no book file found, return default (will fail later with proper error)
        Ok((BookFormat::Epub, format!("{}.epub", book_id)))
    }
    
    pub fn get_data_file_path(&self) -> PathBuf {
        let mut path = self.path.join(&self.id);
        path.set_extension("dat");
        path
    }
    
    pub fn get_book_file_path(&self) -> PathBuf {
        // Use the actual filename discovered during initialization
        self.path.join(&self.book_filename)
    }
    
    pub fn get_output_filename(&self) -> OsString {
        let mut filename = OsString::from(&self.id);
        filename.push(".");
        filename.push(self.format.as_str());
        filename
    }
    
    pub fn get_display_name(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.id.clone())
    }
    
    pub fn is_already_decrypted(&self, config: &Config) -> bool {
        // Check if output file already exists
        let output_path = if let Some(output_dir) = &config.output_directory {
            // Check custom output directory if specified
            PathBuf::from(output_dir).join(self.get_output_filename())
        } else if let Some(library_path) = &config.library_path {
            // Check in the library path (parent of book directories)
            PathBuf::from(library_path).join(self.get_output_filename())
        } else {
            // Fallback: check in parent of book directory (library folder)
            self.path.parent()
                .map(|p| p.join(self.get_output_filename()))
                .unwrap_or_else(|| self.path.join(self.get_output_filename()))
        };

        output_path.exists()
    }
    
    #[allow(dead_code)]  // ← Silences the warning
    pub fn format_file_size(&self) -> String {
        match std::fs::metadata(&self.get_book_file_path()) {
            Ok(metadata) => {
                let size = metadata.len();
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                }
            }
            Err(_) => "Unknown size".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BookFormat {
    Epub,
    Pdf,
    #[allow(dead_code)]  // ← Silences the warning
    Unknown,
}

impl BookFormat {
    #[allow(dead_code)]  // ← Silences the warning
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "epub" => BookFormat::Epub,
            "pdf" => BookFormat::Pdf,
            _ => BookFormat::Unknown,
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            BookFormat::Epub => "epub",
            BookFormat::Pdf => "pdf",
            BookFormat::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LibraryLocation {
    pub path: PathBuf,
    pub confidence: f32,
    #[allow(dead_code)]  // ← Silences the warning
    pub source: LibrarySource,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]  // ← Silences all warnings for this enum
pub enum LibrarySource {
    Registry,
    CommonPath,
    UserSpecified,
    Environment,
}

// Error handling
#[derive(Debug)]
#[allow(dead_code)]  // ← Silences all warnings for this enum
pub enum ProcessingError {
    IoError(std::io::Error),
    DecryptionError(String),
    InvalidPath(String),
    FileNotFound(String),
    ConfigError(String),
}

impl std::fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingError::IoError(e) => write!(f, "IO Error: {}", e),
            ProcessingError::DecryptionError(e) => write!(f, "Decryption Error: {}", e),
            ProcessingError::InvalidPath(e) => write!(f, "Invalid Path: {}", e),
            ProcessingError::FileNotFound(e) => write!(f, "File Not Found: {}", e),
            ProcessingError::ConfigError(e) => write!(f, "Configuration Error: {}", e),
        }
    }
}

impl std::error::Error for ProcessingError {}

impl From<std::io::Error> for ProcessingError {
    fn from(err: std::io::Error) -> Self {
        ProcessingError::IoError(err)
    }
}