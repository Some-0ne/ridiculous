use miette::{IntoDiagnostic, miette};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::*;

pub struct LibraryFinder {
    common_paths: Vec<PathBuf>,
}

impl LibraryFinder {
    pub fn new() -> Self {
        let mut common_paths = Vec::new();
        
        if cfg!(target_os = "windows") {
            if let Ok(appdata) = std::env::var("APPDATA") {
                common_paths.push(PathBuf::from(appdata).join("Ridibooks").join("library"));
            }
            if let Some(appdata) = dirs::data_local_dir() {
                common_paths.push(appdata.join("Ridibooks").join("library"));
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                common_paths.push(PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Ridibooks")
                    .join("library"));
            }
        } else {
            // Linux and other Unix-like systems
            if let Some(home) = dirs::home_dir() {
                common_paths.push(home.join(".local/share/Ridibooks/library"));
                common_paths.push(home.join(".ridibooks/library"));
            }
        }
        
        Self { common_paths }
    }
    
    pub fn find_library_locations(&self) -> Vec<LibraryLocation> {
        let mut locations = Vec::new();
        
        // Check common paths
        for path in &self.common_paths {
            if path.exists() && path.is_dir() {
                let confidence = self.calculate_confidence(path);
                if confidence > 0.0 {
                    locations.push(LibraryLocation {
                        path: path.clone(),
                        confidence,
                        source: LibrarySource::CommonPath,
                    });
                }
            }
        }
        
        // Sort by confidence
        locations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        locations
    }
    
    pub fn find_books(&self, config: &Config) -> miette::Result<Vec<BookInfo>> {
        // Find library path using user_idx
        let library_path = self.get_library_path(&config.user_idx)?;
        
        if !library_path.exists() {
            return Err(miette!(
                "RIDI library not found at: {}\nMake sure RIDI is installed and books are downloaded", 
                library_path.display()
            ));
        }
        
        let mut books = Vec::new();
        
        // Scan the library directory for book folders
        for entry in fs::read_dir(&library_path).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();
            
            if path.is_dir() {
                // Check if this directory contains book files
                if self.is_book_directory(&path) {
                    match BookInfo::new(path) {
                        Ok(book) => books.push(book),
                        Err(e) => {
                            if config.verbose {
                                eprintln!("Warning: Failed to process book directory: {}", e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(books)
    }
    
    fn get_library_path(&self, user_idx: &str) -> miette::Result<PathBuf> {
        // Use the original library path logic
        #[cfg(target_os = "macos")]
        {
            Ok(PathBuf::from(std::env::var("HOME").into_diagnostic()?)
                .join("Library")
                .join("Application Support")
                .join("Ridibooks")
                .join("library")
                .join(format!("_{}", user_idx)))
        }
        
        #[cfg(target_os = "windows")]
        {
            Ok(PathBuf::from(std::env::var("APPDATA").into_diagnostic()?)
                .join("Ridibooks")
                .join("library")
                .join(format!("_{}", user_idx)))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            // Linux fallback
            let home = std::env::var("HOME").into_diagnostic()?;
            Ok(PathBuf::from(home)
                .join(".local/share/Ridibooks/library")
                .join(format!("_{}", user_idx)))
        }
    }
    
    fn calculate_confidence(&self, path: &Path) -> f32 {
        let mut confidence: f32 = 0.1; // Base confidence
        
        // Check for RIDI-specific structure
        if path.join("metadata").exists() {
            confidence += 0.3;
        }
        
        // Check for user directories (_{user_idx} pattern)
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut user_dirs = 0;
                let mut book_count = 0;
                
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let name = entry.file_name().to_string_lossy();
                    
                    if entry_path.is_dir() && name.starts_with('_') {
                        user_dirs += 1;
                        
                        // Count books in user directory
                        if let Ok(user_entries) = fs::read_dir(&entry_path) {
                            book_count += user_entries
                                .flatten()
                                .filter(|e| e.path().is_dir() && self.is_book_directory(&e.path()))
                                .count();
                        }
                    }
                }
                
                if user_dirs > 0 {
                    confidence += 0.4;
                }
                if book_count > 0 {
                    confidence += 0.3;
                }
            }
            Err(_) => return 0.0,
        }
        
        confidence.min(1.0f32)
    }
    
    fn is_book_directory(&self, path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }
        
        // Check if directory contains .dat and book files
        let mut has_dat = false;
        let mut has_book = false;
        
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                
                if entry_path.is_file() {
                    if let Some(ext) = entry_path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        match ext_str.as_str() {
                            "dat" => has_dat = true,
                            "epub" | "pdf" => has_book = true,
                            _ => {}
                        }
                    }
                }
            }
        }
        
        has_dat && has_book
    }
}