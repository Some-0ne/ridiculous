use miette::IntoDiagnostic;
use std::fs;
use std::path::PathBuf;

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
            if let Some(data_local) = dirs::data_local_dir() {
                common_paths.push(data_local.join("Ridibooks").join("library"));
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                common_paths.push(
                    PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                        .join("Ridibooks")
                        .join("library"),
                );
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

        // Sort by confidence descending
        locations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        locations
    }

    pub fn find_books(&self, config: &Config) -> miette::Result<Vec<BookInfo>> {
        let mut books = Vec::new();

        for library_path in &self.common_paths {
            if !library_path.exists() {
                continue;
            }

            let entries = match fs::read_dir(library_path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if path.is_dir() && name.starts_with('_') {
                    // Scan books in this user directory
                    if let Ok(user_entries) = fs::read_dir(&path) {
                        for book_entry in user_entries.flatten() {
                            let book_path = book_entry.path();
                            if book_path.is_dir() && self.is_book_directory(&book_path) {
                                match BookInfo::new(book_path) {
                                    Ok(book) => books.push(book),
                                    Err(e) if config.verbose => {
                                        eprintln!("Warning: Failed to process book directory: {}", e);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(books)
    }

    fn calculate_confidence(&self, path: &PathBuf) -> f32 {
        let mut confidence: f32 = 0.1;

        if path.join("metadata").exists() {
            confidence += 0.3;
        }

        if let Ok(entries) = fs::read_dir(path) {
            let mut user_dirs = 0;
            let mut book_count = 0;

            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy();

                if entry_path.is_dir() && name.starts_with('_') {
                    user_dirs += 1;

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

        confidence.min(1.0)
    }

    fn is_book_directory(&self, path: &PathBuf) -> bool {
        if !path.is_dir() {
            return false;
        }

        let mut has_dat = false;
        let mut has_book_file = false;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(ext) = entry_path.extension() {
                        match ext.to_string_lossy().to_lowercase().as_str() {
                            "dat" => has_dat = true,
                            "epub" | "pdf" => has_book_file = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        has_dat && has_book_file
    }
}