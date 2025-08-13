use clap::{Arg, Command, ArgMatches};
use std::path::PathBuf;
use std::process;
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};

mod types;
mod library_finder;

use types::{Config, ProcessingError, BookInfo};
use library_finder::LibraryFinder;

fn main() {
    // Set up panic hook for better error messages
    std::panic::set_hook(Box::new(|info| {
        eprintln!("💥 Critical error occurred:");
        eprintln!("{}", info);
        eprintln!("\n🔧 This might help:");
        eprintln!("   1. Try running with --verbose for more details");
        eprintln!("   2. Run with --diagnose to check your setup");
        eprintln!("   3. Check that RIDI is properly installed");
    }));

    // Parse command line arguments
    let matches = build_cli().get_matches();
    
    // Run the application
    if let Err(e) = run(matches) {
        eprintln!("❌ Error: {}", e);
        
        // Provide helpful suggestions based on error type
        match e {
            ProcessingError::FileNotFound(ref msg) if msg.contains("library") => {
                eprintln!("\n💡 Try these solutions:");
                eprintln!("   • Run with --diagnose to check library detection");
                eprintln!("   • Make sure RIDI app is installed and books downloaded");
                eprintln!("   • Use manual path selection if prompted");
            }
            ProcessingError::DecryptionError(_) => {
                eprintln!("\n💡 Decryption issues can be caused by:");
                eprintln!("   • Incorrect device ID or user index");
                eprintln!("   • Run get_device_id.sh to verify credentials");
                eprintln!("   • Check that you're logged into the same account");
            }
            _ => {
                eprintln!("\n💡 For more help:");
                eprintln!("   • Run with --verbose for detailed output");
                eprintln!("   • Run with --diagnose for system analysis");
            }
        }
        
        process::exit(1);
    }
}

/// Build the enhanced CLI interface
fn build_cli() -> Command {
    Command::new("ridiculous")
        .version("2.0.0")
        .author("Enhanced by Community")
        .about("Enhanced RIDI Books DRM removal tool with smart detection and GUI support")
        .long_about("
🚀 Ridiculous Enhanced - Smart RIDI Books DRM Removal

This enhanced version includes:
• Smart multi-platform library detection
• Progress bars and user-friendly interface  
• One-time configuration with saved settings
• Skip already-decrypted books automatically
• Comprehensive error handling and recovery
• Optional GUI wrapper scripts

For first-time setup, run without arguments and follow the interactive setup.
        ")
        .arg(Arg::new("device-id")
            .short('d')
            .long("device-id")
            .value_name("ID")
            .help("RIDI device ID (get from account.ridibooks.com/api/user-devices/app)")
            .action(clap::ArgAction::Set))
        .arg(Arg::new("user-idx")
            .short('u')
            .long("user-idx") 
            .value_name("INDEX")
            .help("User index from RIDI API")
            .action(clap::ArgAction::Set))
        .arg(Arg::new("library-path")
            .short('l')
            .long("library-path")
            .value_name("PATH")
            .help("Manual RIDI library path (overrides auto-detection)")
            .action(clap::ArgAction::Set))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("PATH")
            .help("Custom output directory (default: library directory)")
            .action(clap::ArgAction::Set))
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Enable verbose output with detailed progress")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("force")
            .short('f')
            .long("force")
            .help("Re-decrypt all books (ignore already decrypted)")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("organize")
            .long("organize")
            .help("Create epub/ and pdf/ subfolders in output")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("diagnose")
            .long("diagnose")
            .help("Run diagnostic checks and show system info")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("validate-only")
            .long("validate-only")
            .help("Check files and settings without decrypting")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("save-config")
            .long("save-config")
            .help("Save current settings to config file")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("gui")
            .long("gui")
            .help("Launch GUI interface (if available)")
            .action(clap::ArgAction::SetTrue))
}

/// Main application logic
fn run(matches: ArgMatches) -> Result<(), ProcessingError> {
    let verbose = matches.get_flag("verbose");
    
    // Print welcome message
    if verbose {
        print_welcome();
    }

    // Handle special modes first
    if matches.get_flag("diagnose") {
        return run_diagnostics();
    }

    if matches.get_flag("gui") {
        return launch_gui();
    }

    // Load or create configuration
    let mut config = Config::load().unwrap_or_default();
    
    // Merge CLI arguments with config
    config.merge_with_cli(
        matches.get_flag("verbose"),
        matches.get_flag("organize"),
        matches.get_one::<String>("output").map(PathBuf::from)
    );

    // Get device credentials
    let (device_id, user_idx) = get_credentials(&matches, &config)?;
    
    // Update config with credentials if provided via CLI
    if matches.contains_id("device-id") {
        config.device_id = Some(device_id.clone());
    }
    if matches.contains_id("user-idx") {
        config.user_idx = Some(user_idx.clone());
    }

    // Save config if requested
    if matches.get_flag("save-config") {
        config.save().map_err(|e| ProcessingError::InvalidPath(e.to_string()))?;
        println!("✅ Configuration saved to ~/.ridiculous.toml");
    }

    // Find library location
    let library_path = get_library_path(&matches, verbose)?;
    
    if verbose {
        println!("📚 Using library: {}", library_path.display());
    }

    // Scan for books
    let books = scan_for_books(&library_path, verbose)?;
    
    if books.is_empty() {
        println!("📚 No books found in library directory.");
        println!("💡 Make sure you have downloaded books in the RIDI app.");
        return Ok(());
    }

    // Filter books if not forcing re-decryption
    let books_to_process = if matches.get_flag("force") {
        books
    } else {
        let new_books: Vec<BookInfo> = books.into_iter()
            .filter(|book| !book.is_decrypted)
            .collect();
        
        if verbose && new_books.len() == 0 {
            println!("✅ All books already decrypted. Use --force to re-decrypt.");
            return Ok(());
        }
        
        new_books
    };

    if matches.get_flag("validate-only") {
        return validate_books(&books_to_process);
    }

    // Process books
    process_books(books_to_process, &device_id, &user_idx, &config, verbose)
}

/// Print welcome message
fn print_welcome() {
    let term = Term::stdout();
    if term.size().1 > 80 { // If terminal is wide enough
        println!("{}", style("
🚀 ═══════════════════════════════════════════════════════════════
   RIDICULOUS ENHANCED - Smart RIDI Books DRM Removal v2.0
   ═══════════════════════════════════════════════════════════════").cyan().bold());
    } else {
        println!("{}", style("🚀 RIDICULOUS ENHANCED v2.0").cyan().bold());
    }
    println!();
}

/// Get device credentials from CLI args, config, or prompt user
fn get_credentials(matches: &ArgMatches, config: &Config) -> Result<(String, String), ProcessingError> {
    let device_id = matches.get_one::<String>("device-id")
        .cloned()
        .or_else(|| config.device_id.clone())
        .ok_or_else(|| {
            ProcessingError::InvalidPath(
                "Device ID required. Get it from: https://account.ridibooks.com/api/user-devices/app".to_string()
            )
        })?;

    let user_idx = matches.get_one::<String>("user-idx")
        .cloned()
        .or_else(|| config.user_idx.clone())
        .ok_or_else(|| {
            ProcessingError::InvalidPath("User index required. Get it from the same API endpoint.".to_string())
        })?;

    Ok((device_id, user_idx))
}

/// Get library path from CLI args or auto-detection
fn get_library_path(matches: &ArgMatches, verbose: bool) -> Result<PathBuf, ProcessingError> {
    if let Some(manual_path) = matches.get_one::<String>("library-path") {
        let path = PathBuf::from(manual_path);
        if !path.exists() {
            return Err(ProcessingError::FileNotFound(
                format!("Specified library path does not exist: {}", path.display())
            ));
        }
        return Ok(path);
    }

    // Auto-detect library location
    let finder = LibraryFinder::new(verbose);
    let locations = finder.find_libraries()?;
    finder.select_library(&locations)
}

/// Scan directory for book files
fn scan_for_books(library_path: &PathBuf, verbose: bool) -> Result<Vec<BookInfo>, ProcessingError> {
    if verbose {
        println!("🔍 Scanning for books in: {}", library_path.display());
    }

    let mut books = Vec::new();
    
    // Create progress bar for scanning
    let pb = if verbose {
        Some(ProgressBar::new_spinner())
    } else {
        None
    };

    if let Some(pb) = &pb {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
        );
        pb.set_message("Scanning for books...");
    }

    // Walk through directory looking for book files
    for entry in walkdir::WalkDir::new(library_path)
        .max_depth(3) // Don't go too deep
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        
        // Check for book files (typically .dat files for encrypted books)
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if matches!(ext_str.as_str(), "dat" | "epub" | "pdf") {
                if let Ok(book_info) = BookInfo::new(path.to_path_buf(), library_path.clone()) {
                    books.push(book_info);
                    
                    if let Some(pb) = &pb {
                        pb.set_message(format!("Found {} books...", books.len()));
                    }
                }
            }
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message(format!("✅ Found {} books", books.len()));
    }

    Ok(books)
}

/// Validate books without decrypting
fn validate_books(books: &[BookInfo]) -> Result<(), ProcessingError> {
    println!("🔍 Validating {} books...", books.len());
    
    for (i, book) in books.iter().enumerate() {
        println!("{}. {} ({}, {})", 
            i + 1,
            book.path.display(),
            book.format.name(),
            book.format_file_size()
        );
        
        // Check if file exists and is readable
        match std::fs::metadata(&book.path) {
            Ok(metadata) => {
                if metadata.len() == 0 {
                    println!("   ⚠️  Warning: File is empty");
                } else {
                    println!("   ✅ File OK");
                }
            }
            Err(e) => {
                println!("   ❌ Error reading file: {}", e);
            }
        }
    }
    
    println!("\n✅ Validation complete");
    Ok(())
}

/// Process (decrypt) books
fn process_books(
    books: Vec<BookInfo>, 
    device_id: &str, 
    user_idx: &str, 
    config: &Config,
    verbose: bool
) -> Result<(), ProcessingError> {
    println!("🔄 Processing {} books...", books.len());
    
    // Create main progress bar
    let main_pb = ProgressBar::new(books.len() as u64);
    main_pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ")
    );

    let mut success_count = 0;
    let mut skip_count = 0;
    let mut error_count = 0;

    for (i, book) in books.iter().enumerate() {
        main_pb.set_position(i as u64);
        main_pb.set_message(format!("Processing: {}", 
            book.path.file_stem().unwrap_or_default().to_string_lossy()
        ));

        // Skip if already decrypted (unless force mode)
        if book.is_decrypted && !config.verbose.unwrap_or(false) {
            skip_count += 1;
            if verbose {
                println!("⏭️  Skipping already decrypted: {}", book.path.display());
            }
            continue;
        }

        // Here you would call your actual decryption function
        // For now, this is a placeholder that represents the original decryption logic
        match decrypt_book(book, device_id, user_idx, verbose) {
            Ok(_) => {
                success_count += 1;
                if verbose {
                    println!("✅ Successfully decrypted: {}", book.path.display());
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("❌ Failed to decrypt {}: {}", book.path.display(), e);
            }
        }
    }

    main_pb.finish_with_message("Processing complete");
    
    // Print summary
    println!("\n📊 Processing Summary:");
    println!("   ✅ Successful: {}", success_count);
    if skip_count > 0 {
        println!("   ⏭️  Skipped: {}", skip_count);
    }
    if error_count > 0 {
        println!("   ❌ Errors: {}", error_count);
    }
    
    Ok(())
}

/// Placeholder for the actual decryption function
/// You'll need to integrate your existing decryption logic here
fn decrypt_book(book: &BookInfo, device_id: &str, user_idx: &str, verbose: bool) -> Result<(), ProcessingError> {
    // TODO: Integrate your existing decryption logic from the original main.rs
    // This is where you would:
    // 1. Read the encrypted book file
    // 2. Extract the key using device_id and user_idx
    // 3. Decrypt the content
    // 4. Write the decrypted file to the output location
    
    if verbose {
        println!("🔓 Decrypting: {} ({})", book.path.display(), book.format_file_size());
    }
    
    // Placeholder - replace with actual decryption
    std::thread::sleep(std::time::Duration::from_millis(100)); // Simulate work
    
    Ok(())
}

/// Run diagnostic checks
fn run_diagnostics() -> Result<(), ProcessingError> {
    let finder = LibraryFinder::new(true);
    finder.diagnose()?;
    
    println!("🔧 Configuration Diagnostics:");
    match Config::load() {
        Ok(config) => {
            println!("  ✅ Config file found");
            println!("     Device ID: {}", 
                config.device_id.as_deref().unwrap_or("Not set"));
            println!("     User Index: {}", 
                config.user_idx.as_deref().unwrap_or("Not set"));
        }
        Err(_) => {
            println!("  ⚠️  No config file found (will be created on first run)");
        }
    }
    
    Ok(())
}

/// Launch GUI interface
fn launch_gui() -> Result<(), ProcessingError> {
    println!("🖥️  Launching GUI interface...");
    
    // Check if GUI script exists
    let gui_script = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .map(|dir| dir.join("ridiculous_gui_enhanced.sh"));
    
    if let Some(script_path) = gui_script {
        if script_path.exists() {
            // Execute the GUI script
            let output = std::process::Command::new("bash")
                .arg(&script_path)
                .output();
            
            match output {
                Ok(_) => return Ok(()),
                Err(e) => {
                    eprintln!("Failed to launch GUI: {}", e);
                }
            }
        }
    }
    
    eprintln!("❌ GUI script not found. Please install the GUI components or use CLI mode.");
    Err(ProcessingError::FileNotFound("GUI script not available".to_string()))
}