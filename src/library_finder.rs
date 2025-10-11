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
        // Try to find library path using user_idx
        let library_paths = self.get_library_paths(&config.user_idx)?;
        
        let mut books = Vec::new();
        let mut checked_paths = Vec::new();
        
        // Try each potential library path
        for library_path in library_paths {
            checked_paths.push(library_path.display().to_string());
            
            if !library_path.exists() {
                if config.verbose {
                    eprintln!("⚠️  Path doesn't exist: {}", library_path.display());
                }
                continue;
            }
            
            if config.verbose {
                println!("🔍 Scanning: {}", library_path.display());
            }
            
            // Scan the library directory for book folders
            match fs::read_dir(&library_path) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(_) => continue,
                        };
                        let path = entry.path();
                        
                        if path.is_dir() {
                            // Check if this directory contains book files
                            if self.is_book_directory(&path) {
                                if config.verbose {
                                    println!("📖 Found book directory: {}", path.display());
                                }
                                match BookInfo::new(path) {
                                    Ok(book) => books.push(book),
                                    Err(e) => {
                                        if config.verbose {
                                            eprintln!("⚠️  Failed to process book directory: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // If we found books in this path, no need to check others
                    if !books.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    if config.verbose {
                        eprintln!("⚠️  Cannot read directory {}: {}", library_path.display(), e);
                    }
                }
            }
        }
        
        if books.is_empty() {
            return Err(miette!(
                "No books found in any library location.\n\
                 Checked paths:\n{}\n\n\
                 Make sure:\n\
                 1. RIDI app is installed\n\
                 2. You've downloaded books in the RIDI app\n\
                 3. Books are in one of the above locations",
                checked_paths.join("\n")
            ));
        }
        
        Ok(books)
    }
    
    fn get_library_paths(&self, user_idx: &str) -> miette::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        
        #[cfg(target_os = "macos")]
        {
            let base = PathBuf::from(std::env::var("HOME").into_diagnostic()?)
                .join("Library")
                .join("Application Support")
                .join("Ridibooks")
                .join("library");
            
            // Try with _{user_idx} subdirectory first
            paths.push(base.join(format!("_{}", user_idx)));
            // Then try the base library directory
            paths.push(base.clone());
            // Also try scanning for any user directories
            if base.exists() {
                if let Ok(entries) = fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if name.starts_with('_') {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            let base = PathBuf::from(std::env::var("APPDATA").into_diagnostic()?)
                .join("Ridibooks")
                .join("library");
            
            paths.push(base.join(format!("_{}", user_idx)));
            paths.push(base.clone());
            
            if base.exists() {
                if let Ok(entries) = fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if name.starts_with('_') {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let home = std::env::var("HOME").into_diagnostic()?;
            let base = PathBuf::from(home).join(".local/share/Ridibooks/library");
            
            paths.push(base.join(format!("_{}", user_idx)));
            paths.push(base.clone());
            
            if base.exists() {
                if let Ok(entries) = fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if name.starts_with('_') {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(paths)
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
                    let name = entry.file_name().to_string_lossy().to_string();
                    
                    if entry_path.is_dir() && name.starts_with('_') {
                        user_dirs += 1;
                        
                        // Count books in user directory
                        if let Ok(user_entries) = fs::read_dir(&entry_path) {
                            book_count += user_entries
                                .flatten()
                                .filter(|e| e.path().is_dir() && self.is_book_directory(&e.path()))
                                .count();
                        }
                    } else if entry_path.is_dir() && self.is_book_directory(&entry_path) {
                        // Direct book directories (no user subdirectory)
                        book_count += 1;
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