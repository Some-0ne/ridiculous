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
                common_paths.push(appdata_path(&appdata));
            }
            if let Some(local) = dirs::data_local_dir() {
                common_paths.push(local.join("Ridibooks").join("library"));
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                common_paths.push(home_path(&home));
            }
        } else {
            // Linux / Unix
            if let Some(home) = dirs::home_dir() {
                common_paths.push(home.join(".local/share/Ridibooks/library"));
                common_paths.push(home.join(".ridibooks/library"));
            }
        }

        Self { common_paths }
    }

    pub fn find_library_locations(&self) -> Vec<LibraryLocation> {
        let mut locations: Vec<LibraryLocation> = self
            .common_paths
            .iter()
            .filter(|path| path.exists() && path.is_dir())
            .filter_map(|path| {
                let confidence = self.calculate_confidence(path);
                if confidence > 0.0 {
                    Some(LibraryLocation {
                        path: path.clone(),
                        confidence,
                        source: LibrarySource::CommonPath,
                    })
                } else {
                    None
                }
            })
            .collect();

        locations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        locations
    }

    pub fn find_books(&self, config: &Config) -> miette::Result<Vec<BookInfo>> {
        let library_path = self.get_library_path(&config.user_idx)?;

        if !library_path.exists() {
            return Err(miette!(
                "RIDI library not found at: {}\nMake sure RIDI is installed and books are downloaded",
                library_path.display()
            ));
        }

        let mut books = Vec::new();

        for entry in fs::read_dir(&library_path).into_diagnostic()? {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();

            if path.is_dir() && self.is_book_directory(&path) {
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

        Ok(books)
    }

    fn get_library_path(&self, user_idx: &str) -> miette::Result<PathBuf> {
        let path = if cfg!(target_os = "macos") {
            PathBuf::from(std::env::var("HOME").into_diagnostic()?)
                .join("Library")
                .join("Application Support")
                .join("Ridibooks")
                .join("library")
                .join(format!("_{}", user_idx))
        } else if cfg!(target_os = "windows") {
            PathBuf::from(std::env::var("APPDATA").into_diagnostic()?)
                .join("Ridibooks")
                .join("library")
                .join(format!("_{}", user_idx))
        } else {
            PathBuf::from(std::env::var("HOME").into_diagnostic()?)
                .join(".local/share/Ridibooks/library")
                .join(format!("_{}", user_idx))
        };

        Ok(path)
    }

    fn calculate_confidence(&self, path: &Path) -> f32 {
        if !path.exists() || !path.is_dir() {
            return 0.0;
        }

        let mut confidence = 0.1;

        if path.join("metadata").exists() {
            confidence += 0.3;
        }

        let mut user_dirs = 0;
        let mut book_count = 0;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name: String = entry.file_name().into_string().unwrap_or_default();

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
        }

        if user_dirs > 0 {
            confidence += 0.4;
        }
        if book_count > 0 {
            confidence += 0.3;
        }

        confidence.min(1.0)
    }

    fn is_book_directory(&self, path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }

        let mut has_dat = false;
        let mut has_book = false;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if let Some(ext) = entry_path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    match ext.as_str() {
                        "dat" => has_dat = true,
                        "epub" | "pdf" => has_book = true,
                        _ => {}
                    }
                }
            }
        }

        has_dat && has_book
    }
}

/// Helper functions for platform-specific paths
fn appdata_path(appdata: &str) -> PathBuf {
    PathBuf::from(appdata).join("Ridibooks").join("library")
}

fn home_path(home: &str) -> PathBuf {
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Ridibooks")
        .join("library")
}