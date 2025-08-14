use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::ffi::OsString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device_id: String,
    pub user_idx: String,
    pub verbose: bool,
    pub organize_output: bool,
    pub backup_originals: bool,
    pub output_directory: Option<String>,
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
}

impl BookInfo {
    pub fn new(book_dir: PathBuf) -> miette::Result<Self> {
        let id = book_dir.file_name()
            .ok_or_else(|| miette::miette!("Invalid book directory"))?
            .to_string_lossy()
            .to_string();
        
        let format = Self::detect_format(&book_dir)?;
        
        Ok(Self {
            id,
            format,
            path: book_dir,
            title: None,
        })
    }
    
    fn detect_format(book_dir: &PathBuf) -> miette::Result<BookFormat> {
        for entry in std::fs::read_dir(book_dir).map_err(|e| miette::miette!("Cannot read book directory: {}", e))? {
            let entry = entry.map_err(|e| miette::miette!("Directory entry error: {}", e))?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    match ext.to_string_lossy().to_lowercase().as_str() {
                        "epub" => return Ok(BookFormat::Epub),
                        "pdf" => return Ok(BookFormat::Pdf),
                        _ => continue,
                    }
                }
            }
        }
        
        // Default to EPUB if no specific format found
        Ok(BookFormat::Epub)
    }
    
    pub fn get_data_file_path(&self) -> PathBuf {
        let mut path = self.path.join(&self.id);
        path.set_extension("dat");
        path
    }
    
    pub fn get_book_file_path(&self) -> PathBuf {
        let mut path = self.path.join(&self.id);
        path.set_extension(self.format.as_str());
        path
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
    
    pub fn is_already_decrypted(&self) -> bool {
        // Check if output file already exists in current directory
        let output_path = std::env::current_dir()
            .map(|dir| dir.join(self.get_output_filename()))
            .unwrap_or_else(|_| PathBuf::from(self.get_output_filename()));
        
        output_path.exists()
    }
    
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
    Unknown,
}

impl BookFormat {
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
    pub source: LibrarySource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibrarySource {
    Registry,
    CommonPath,
    UserSpecified,
    Environment,
}

// Error handling
#[derive(Debug)]
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