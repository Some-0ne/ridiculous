use std::path::PathBuf;
use std::collections::HashMap;
use crate::types::{LibraryLocation, ProcessingError};
use console::{Style, Term};
use std::io::Write;

/// Smart multi-platform RIDI library detection
pub struct LibraryFinder {
    verbose: bool,
}

impl LibraryFinder {
    pub fn new(verbose: bool) -> Self {
        LibraryFinder { verbose }
    }

    /// Find RIDI library locations with confidence scoring
    pub fn find_libraries(&self) -> Result<Vec<LibraryLocation>, ProcessingError> {
        if self.verbose {
            println!("🔍 Searching for RIDI library locations...");
        }

        let mut locations = Vec::new();

        // macOS specific locations
        locations.extend(self.check_macos_locations());
        
        // Windows specific locations (for completeness)
        locations.extend(self.check_windows_locations());
        
        // Generic locations
        locations.extend(self.check_generic_locations());

        // Sort by confidence (highest first)
        locations.sort_by(|a, b| b.confidence.cmp(&a.confidence));

        // Filter out invalid locations
        let valid_locations: Vec<LibraryLocation> = locations
            .into_iter()
            .filter(|loc| loc.is_valid())
            .collect();

        if self.verbose {
            self.print_search_results(&valid_locations);
        }

        Ok(valid_locations)
    }

    /// Check macOS-specific RIDI library locations
    fn check_macos_locations(&self) -> Vec<LibraryLocation> {
        let mut locations = Vec::new();

        // Get home directory
        if let Some(home) = dirs::home_dir() {
            
            // Standard Application Support location
            let app_support = home.join("Library/Application Support/RIDI/library");
            if app_support.exists() {
                locations.push(LibraryLocation::new(
                    app_support,
                    95,
                    "macOS Application Support (standard)".to_string()
                ));
            }

            // Container/Sandboxed app locations
            let containers_base = home.join("Library/Containers");
            
            // Check for various RIDI app containers
            let ridi_containers = [
                "com.ridibooks.mac",
                "com.ridi.RIDIBOOKS",
                "RIDIBOOKS",
                "ridi",
            ];

            for container in &ridi_containers {
                let container_path = containers_base
                    .join(container)
                    .join("Data/Library/Application Support/RIDI/library");
                
                if container_path.exists() {
                    locations.push(LibraryLocation::new(
                        container_path,
                        90,
                        format!("macOS Container ({})", container)
                    ));
                }
            }

            // Group Containers (for shared app data)
            let group_containers = home.join("Library/Group Containers");
            if group_containers.exists() {
                // Look for RIDI-related group containers
                if let Ok(entries) = std::fs::read_dir(&group_containers) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy().to_lowercase();
                        
                        if name_str.contains("ridi") {
                            let lib_path = entry.path().join("Library/Application Support/RIDI/library");
                            if lib_path.exists() {
                                locations.push(LibraryLocation::new(
                                    lib_path,
                                    85,
                                    format!("macOS Group Container ({})", name.to_string_lossy())
                                ));
                            }
                        }
                    }
                }
            }

            // Alternative locations within user directory
            let alt_locations = [
                home.join("Documents/RIDI"),
                home.join("Downloads/RIDI"),
                home.join(".ridi/library"),
                home.join("Library/Caches/RIDI/library"),
            ];

            for (i, alt_path) in alt_locations.iter().enumerate() {
                if alt_path.exists() {
                    locations.push(LibraryLocation::new(
                        alt_path.clone(),
                        60 - (i * 5) as u8, // Decreasing confidence
                        "macOS Alternative location".to_string()
                    ));
                }
            }
        }

        locations
    }

    /// Check Windows-specific locations (for cross-platform support)
    fn check_windows_locations(&self) -> Vec<LibraryLocation> {
        let mut locations = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use std::env;

            // Windows-specific paths
            if let Ok(appdata) = env::var("APPDATA") {
                let appdata_path = PathBuf::from(appdata);
                
                let win_locations = [
                    appdata_path.join("RIDI/library"),
                    appdata_path.join("Roaming/RIDI/library"),
                ];

                for (i, path) in win_locations.iter().enumerate() {
                    if path.exists() {
                        locations.push(LibraryLocation::new(
                            path.clone(),
                            90 - (i * 5) as u8,
                            "Windows AppData".to_string()
                        ));
                    }
                }
            }

            if let Ok(localappdata) = env::var("LOCALAPPDATA") {
                let local_path = PathBuf::from(localappdata).join("RIDI/library");
                if local_path.exists() {
                    locations.push(LibraryLocation::new(
                        local_path,
                        85,
                        "Windows LocalAppData".to_string()
                    ));
                }
            }

            // Program Files locations
            let program_files = [
                PathBuf::from("C:\\Program Files\\RIDI\\library"),
                PathBuf::from("C:\\Program Files (x86)\\RIDI\\library"),
            ];

            for path in &program_files {
                if path.exists() {
                    locations.push(LibraryLocation::new(
                        path.clone(),
                        75,
                        "Windows Program Files".to_string()
                    ));
                }
            }
        }

        locations
    }

    /// Check generic cross-platform locations
    fn check_generic_locations(&self) -> Vec<LibraryLocation> {
        let mut locations = Vec::new();

        // Current directory and common relative paths
        let current_dir = std::env::current_dir().unwrap_or_default();
        let generic_paths = [
            current_dir.join("library"),
            current_dir.join("ridi/library"),
            current_dir.join("RIDI/library"),
            PathBuf::from("./library"),
            PathBuf::from("../library"),
        ];

        for (i, path) in generic_paths.iter().enumerate() {
            if path.exists() {
                locations.push(LibraryLocation::new(
                    path.clone(),
                    30 - (i * 5) as u8, // Low confidence for generic paths
                    "Generic/Current directory".to_string()
                ));
            }
        }

        locations
    }

    /// Interactive library selection when multiple found
    pub fn select_library(&self, locations: &[LibraryLocation]) -> Result<PathBuf, ProcessingError> {
        if locations.is_empty() {
            return Err(ProcessingError::FileNotFound(
                "No RIDI library locations found. Please ensure RIDI is installed and books are downloaded.".to_string()
            ));
        }

        if locations.len() == 1 {
            if self.verbose {
                println!("✅ Found single library location: {}", locations[0].path.display());
            }
            return Ok(locations[0].path.clone());
        }

        // Multiple locations found - interactive selection
        self.interactive_selection(locations)
    }

    /// Interactive selection interface
    fn interactive_selection(&self, locations: &[LibraryLocation]) -> Result<PathBuf, ProcessingError> {
        let term = Term::stdout();
        let style_header = Style::new().bold().cyan();
        let style_option = Style::new().yellow();
        let style_detail = Style::new().dim();

        println!();
        println!("{}", style_header.apply_to("📚 Multiple RIDI library locations found:"));
        println!();

        for (i, location) in locations.iter().enumerate() {
            println!("{} {}. {}",
                style_option.apply_to(&format!("[{}]", i + 1)),
                location.path.display(),
                style_detail.apply_to(&format!("({} books, {} confidence, via {})", 
                    location.book_count,
                    location.confidence,
                    location.source
                ))
            );
        }

        println!();
        println!("{} {}. {}", 
            style_option.apply_to("[M]"), 
            "Manually specify path",
            style_detail.apply_to("(enter custom path)")
        );
        println!();

        loop {
            print!("Select library location (1-{}, M for manual): ", locations.len());
            std::io::stdout().flush().unwrap();

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            if input.to_lowercase() == "m" {
                return self.manual_path_input();
            }

            if let Ok(choice) = input.parse::<usize>() {
                if choice > 0 && choice <= locations.len() {
                    let selected = &locations[choice - 1];
                    println!("✅ Selected: {}", selected.path.display());
                    return Ok(selected.path.clone());
                }
            }

            println!("❌ Invalid selection. Please choose 1-{} or M.", locations.len());
        }
    }

    /// Manual path input
    fn manual_path_input(&self) -> Result<PathBuf, ProcessingError> {
        println!();
        println!("📁 Enter the full path to your RIDI library directory:");
        println!("   (This should contain .dat files or book files)");
        print!("Path: ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let path = PathBuf::from(input.trim());

        if !path.exists() {
            return Err(ProcessingError::FileNotFound(
                format!("Path does not exist: {}", path.display())
            ));
        }

        if !path.is_dir() {
            return Err(ProcessingError::InvalidPath(
                format!("Path is not a directory: {}", path.display())
            ));
        }

        // Validate that it looks like a library directory
        let test_location = LibraryLocation::new(path.clone(), 100, "Manual input".to_string());
        if test_location.book_count == 0 {
            println!("⚠️  Warning: No book files found in the specified directory.");
            print!("Continue anyway? (y/N): ");
            std::io::stdout().flush().unwrap();

            let mut confirm = String::new();
            std::io::stdin().read_line(&mut confirm).unwrap();
            
            if !confirm.trim().to_lowercase().starts_with('y') {
                return Err(ProcessingError::InvalidPath(
                    "Directory validation cancelled by user".to_string()
                ));
            }
        }

        println!("✅ Manual path accepted: {}", path.display());
        Ok(path)
    }

    /// Print search results for verbose mode
    fn print_search_results(&self, locations: &[LibraryLocation]) {
        println!();
        println!("🔍 Library search results:");
        
        if locations.is_empty() {
            println!("   ❌ No valid library locations found");
        } else {
            for location in locations {
                println!("   ✅ {} (confidence: {}, books: {}, source: {})",
                    location.path.display(),
                    location.confidence,
                    location.book_count,
                    location.source
                );
            }
        }
        println!();
    }

    /// Diagnose library detection for troubleshooting
    pub fn diagnose(&self) -> Result<(), ProcessingError> {
        println!("🔧 RIDI Library Detection Diagnostics");
        println!("=====================================");
        println!();

        // System info
        println!("System Information:");
        println!("  OS: {}", std::env::consts::OS);
        println!("  Architecture: {}", std::env::consts::ARCH);
        if let Some(home) = dirs::home_dir() {
            println!("  Home directory: {}", home.display());
        }
        println!();

        // Check all possible locations (including invalid ones)
        println!("Checking all possible RIDI locations:");
        println!();

        let mut all_locations = Vec::new();
        all_locations.extend(self.check_macos_locations());
        all_locations.extend(self.check_windows_locations());
        all_locations.extend(self.check_generic_locations());

        if all_locations.is_empty() {
            println!("❌ No potential locations found");
        } else {
            for location in &all_locations {
                let status = if location.path.exists() {
                    if location.is_valid() {
                        format!("✅ Valid ({} books)", location.book_count)
                    } else {
                        "⚠️  Exists but no books found".to_string()
                    }
                } else {
                    "❌ Does not exist".to_string()
                };

                println!("  {} - {} ({})", 
                    location.path.display(),
                    status,
                    location.source
                );
            }
        }

        println!();
        println!("Recommendations:");
        println!("  1. Make sure RIDI app is installed");
        println!("  2. Download some books from your purchases");
        println!("  3. Check that books appear in the RIDI app");
        println!("  4. Use the manual path option if needed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_library_finder_creation() {
        let finder = LibraryFinder::new(false);
        assert!(!finder.verbose);

        let finder_verbose = LibraryFinder::new(true);
        assert!(finder_verbose.verbose);
    }

    #[test]
    fn test_library_location_validation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Empty directory should have 0 books
        let location = LibraryLocation::new(temp_path.clone(), 100, "Test".to_string());
        assert_eq!(location.book_count, 0);
        assert!(!location.is_valid());

        // Create a test book file
        let test_book = temp_path.join("test.dat");
        std::fs::write(&test_book, "test content").unwrap();

        let location_with_book = LibraryLocation::new(temp_path, 100, "Test".to_string());
        assert_eq!(location_with_book.book_count, 1);
        assert!(location_with_book.is_valid());
    }
}