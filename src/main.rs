use aes::cipher::KeyIvInit;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use miette::{IntoDiagnostic, miette};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

mod types;
mod library_finder;
mod credential_manager;

use types::*;
use library_finder::LibraryFinder;
use credential_manager::CredentialManager;

#[derive(Parser, Debug)]
#[command(name = "ridiculous")]
#[command(about = "Enhanced RIDI book decryption tool")]
#[command(version = "0.3.0")]
struct Args {
    #[arg(short, long)]
    device_id: Option<String>,
    
    #[arg(short, long)]
    user_idx: Option<String>,
    
    #[arg(short, long)]
    verbose: bool,
    
    #[arg(long)]
    diagnose: bool,
    
    #[arg(long)]
    validate_only: bool,
    
    #[arg(long, default_value = "4")]
    parallel: usize,
    
    #[arg(long)]
    batch_mode: bool,
    
    #[arg(long)]
    resume: bool,
    
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
    
    #[arg(long)]
    config_path: Option<PathBuf>,
    
    #[arg(long)]
    force: bool,
    
    #[arg(long)]
    organize: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct ProcessingState {
    completed: Vec<String>,
    failed: Vec<(String, String)>, // book_id, error
    in_progress: Vec<String>,
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self {
            completed: Vec::new(),
            failed: Vec::new(),
            in_progress: Vec::new(),
        }
    }
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    // Set up panic hook for better error messages
    std::panic::set_hook(Box::new(|info| {
        eprintln!("💥 Critical error occurred:");
        eprintln!("{}", info);
        eprintln!("\n🔧 This might help:");
        eprintln!("   1. Try running with --verbose for more details");
        eprintln!("   2. Run with --diagnose to check your setup");
        eprintln!("   3. Check that RIDI is properly installed");
    }));

    let args = Args::parse();
    
    if args.verbose {
        print_welcome();
    }
    
    // Handle special modes first
    if args.diagnose {
        return run_diagnostics(&args).await;
    }
    
    if args.validate_only {
        let config = load_or_create_config(&args)?;
        return validate_credentials(&config).await.map_err(|e| miette::miette!("{}", e));
    }
    
    // Load or create config
    let config = load_or_create_config(&args)?;
    
    // Load processing state for resume functionality
    let mut state = if args.resume {
        load_processing_state().unwrap_or_default()
    } else {
        ProcessingState::default()
    };
    
    // Find books using library finder
    let library_finder = LibraryFinder::new();
    let books = library_finder.find_books(&config)?;
    
    if books.is_empty() {
        println!("❌ No books found. Make sure RIDI is installed and books are downloaded.");
        return Ok(());
    }
    
    // Filter out already processed books if resuming, or if not forcing re-decryption
    let books_to_process: Vec<_> = books.into_iter()
        .filter(|book| {
            if args.force {
                true
            } else if args.resume {
                !state.completed.contains(&book.id)
            } else {
                !book.is_already_decrypted()
            }
        })
        .collect();
    
    if books_to_process.is_empty() {
        println!("✅ All books already decrypted. Use --force to re-decrypt.");
        return Ok(());
    }
    
    println!("📚 Found {} books to process", books_to_process.len());
    
    if args.batch_mode {
        process_books_batch(books_to_process, &config, &mut state, args.parallel).await?;
    } else {
        process_books_interactive(books_to_process, &config, &mut state).await?;
    }
    
    // Save final state
    save_processing_state(&state)?;
    
    print_summary(&state);
    Ok(())
}

fn print_welcome() {
    println!("{}", console::style("
🚀 ═══════════════════════════════════════════════════════════════
   RIDICULOUS ENHANCED - Smart RIDI Books DRM Removal v0.3.0
   ═══════════════════════════════════════════════════════════════").cyan().bold());
    println!();
}

async fn process_books_batch(
    books: Vec<BookInfo>,
    config: &Config,
    state: &mut ProcessingState,
    max_parallel: usize,
) -> miette::Result<()> {
    let multi_progress = MultiProgress::new();
    let semaphore = Arc::new(Semaphore::new(max_parallel));
    
    let overall_pb = multi_progress.add(ProgressBar::new(books.len() as u64));
    overall_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} books ({msg})")
            .unwrap()
    );
    overall_pb.set_message("Processing books...");
    
    let mut handles = Vec::new();
    
    for book in books {
        let semaphore = semaphore.clone();
        let config = config.clone();
        let multi_progress = multi_progress.clone();
        let overall_pb = overall_pb.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            let pb = multi_progress.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} {msg} [{bar:30.cyan/blue}] {percent}%")
                    .unwrap()
            );
            pb.set_message(format!("📖 {}", book.get_display_name()));
            
            let result = process_single_book(&book, &config, &pb).await;
            
            pb.finish_with_message(match &result {
                Ok(_) => format!("✅ {}", book.get_display_name()),
                Err(e) => format!("❌ {} - {}", book.get_display_name(), e),
            });
            
            overall_pb.inc(1);
            
            (book.id.clone(), result)
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks and collect results
    for handle in handles {
        let (book_id, result) = handle.await.unwrap();
        match result {
            Ok(_) => state.completed.push(book_id),
            Err(e) => state.failed.push((book_id, e.to_string())),
        }
        
        // Periodically save state
        if (state.completed.len() + state.failed.len()) % 5 == 0 {
            let _ = save_processing_state(state);
        }
    }
    
    overall_pb.finish_with_message("🎉 Batch processing complete!");
    Ok(())
}

async fn process_books_interactive(
    books: Vec<BookInfo>,
    config: &Config,
    state: &mut ProcessingState,
) -> miette::Result<()> {
    for (i, book) in books.iter().enumerate() {
        println!("\n📖 Processing book {}/{}: {}", 
                 i + 1, books.len(), book.get_display_name());
        
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
                .unwrap()
        );
        
        match process_single_book(book, config, &pb).await {
            Ok(_) => {
                pb.finish_with_message("✅ Complete");
                state.completed.push(book.id.clone());
                println!("✅ Successfully processed: {}", book.get_display_name());
            }
            Err(e) => {
                pb.finish_with_message("❌ Failed");
                state.failed.push((book.id.clone(), e.to_string()));
                eprintln!("❌ Failed to process {}: {}", book.get_display_name(), e);
                
                // Ask if user wants to continue
                println!("Continue with next book? (y/n)");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).into_diagnostic()?;
                if input.trim().to_lowercase() != "y" {
                    break;
                }
            }
        }
        
        save_processing_state(state).map_err(|e| miette::miette!("{}", e))?;
    }
    
    Ok(())
}

async fn process_single_book(
    book: &BookInfo,
    config: &Config,
    pb: &ProgressBar,
) -> Result<()> {
    pb.set_message("Reading book file...");
    pb.set_position(10);
    
    // Retry logic for file operations
    let mut retries = 3;
    while retries > 0 {
        match decrypt_book_with_original_logic(book, &config.device_id, pb).await {
            Ok(_) => return Ok(()),
            Err(e) if retries > 1 && is_retryable_error(&e) => {
                let message = format!("Retrying... ({} attempts left)", retries - 1);
                pb.set_message(&message);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    }
    
    unreachable!()
}

// Core RIDI decryption functions (from original code)
async fn decrypt_book_with_original_logic(
    book: &BookInfo, 
    device_id: &str, 
    pb: &ProgressBar
) -> Result<()> {
    pb.set_message("Extracting decryption key...");
    pb.set_position(20);
    
    // Get the decryption key using original logic
    let key = decrypt_key(book, device_id)?;
    
    pb.set_message("Decrypting book content...");
    pb.set_position(50);
    
    // Decrypt the book using original logic
    let decrypted_content = decrypt_book_content(book, &key)?;
    
    pb.set_message("Writing decrypted file...");
    pb.set_position(80);
    
    // Write the decrypted content
    let output_path = get_output_path(book)?;
    
    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(&output_path, decrypted_content)?;
    
    pb.set_position(100);
    
    if let Some(file_name) = output_path.file_name() {
        pb.set_message(format!("Saved: {}", file_name.to_string_lossy()));
    }
    
    Ok(())
}

// Original decrypt_key function adapted
fn decrypt_key(book_info: &BookInfo, device_id: &str) -> Result<[u8; 16]> {
    let data_file_path = book_info.get_data_file_path();
    let data_file = fs::read(&data_file_path)
        .with_context(|| format!("Failed to read data file: {}", data_file_path.display()))?;

    if data_file.len() < 32 {
        return Err(anyhow::anyhow!("Data file too small: {} bytes", data_file.len()));
    }

    let mut key = [0; 16];
    let device_bytes = device_id.as_bytes();
    let key_len = std::cmp::min(16, device_bytes.len());
    key[..key_len].copy_from_slice(&device_bytes[..key_len]);

    let mut iv = [0; 16];
    iv.copy_from_slice(&data_file[0..16]);

    let plaintext = cbc::Decryptor::<aes::Aecs7.decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>s128>::new(&key.into(), &iv.into())
        (&data_file[16..])
        .map_err(|error| anyhow::anyhow!("Decryption failed: {}", error))?;

    let plaintext_str = std::str::from_utf8(&plaintext)
        .context("Invalid UTF-8 in decrypted data")?;
    
    if plaintext_str.len() < 84 {
        return Err(anyhow::anyhow!("Decrypted data too short: {} chars", plaintext_str.len()));
    }

    let mut result = [0; 16];
    let key_slice = &plaintext_str[68..84];
    let key_bytes = key_slice.as_bytes();
    let copy_len = std::cmp::min(16, key_bytes.len());
    result[..copy_len].copy_from_slice(&key_bytes[..copy_len]);

    Ok(result)
}

// Original decrypt_book function adapted
fn decrypt_book_content(book_info: &BookInfo, key: &[u8; 16]) -> Result<Vec<u8>> {
    let book_file_path = book_info.get_book_file_path();
    let book_file = fs::read(&book_file_path)
        .with_context(|| format!("Failed to read book file: {}", book_file_path.display()))?;

    if book_file.len() < 16 {
        return Err(anyhow::anyhow!("Book file too small: {} bytes", book_file.len()));
    }

    let mut iv = [0; 16];
    iv.copy_from_slice(&book_file[0..16]);

    let decrypted = cbc::Decryptor::<aes::Aes128>::new(key.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&book_file[16..])
        .map_err(|error| anyhow::anyhow!("Book decryption failed: {}", error))?;

    Ok(decrypted)
}

fn get_output_path(book: &BookInfo) -> Result<PathBuf> {
    let file_name = book.get_output_filename();
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(file_name))
}

fn is_retryable_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("timeout") || 
    error_str.contains("connection") ||
    error_str.contains("network") ||
    error_str.contains("temporary") ||
    error_str.contains("io error")
}

async fn run_diagnostics(args: &Args) -> miette::Result<()> {
    println!("🔍 Running diagnostics...\n");
    
    // Check library locations
    println!("1. Checking library locations...");
    let finder = LibraryFinder::new();
    let locations = finder.find_library_locations();
    
    if locations.is_empty() {
        println!("   ❌ No RIDI library locations found");
        println!("   💡 Make sure RIDI app is installed and you've downloaded books");
    } else {
        for location in locations {
            println!("   📁 Found: {} (confidence: {}%)", 
                    location.path.display(), 
                    (location.confidence * 100.0) as u32);
        }
    }
    
    // Check credentials if provided
    if let (Some(device_id), Some(user_idx)) = (&args.device_id, &args.user_idx) {
        println!("\n2. Checking credentials...");
        let config = Config {
            device_id: device_id.clone(),
            user_idx: user_idx.clone(),
            ..Default::default()
        };
        
        match validate_credentials(&config).await {
            Ok(_) => println!("   ✅ Credentials valid"),
            Err(e) => println!("   ❌ Credential error: {}", e),
        }
        
        // Try to find books
        println!("\n3. Checking books...");
        match finder.find_books(&config) {
            Ok(books) => {
                println!("   📚 Found {} books", books.len());
                for book in books.iter().take(3) {
                    println!("     - {} ({})", book.get_display_name(), book.format.as_str());
                }
                if books.len() > 3 {
                    println!("     ... and {} more", books.len() - 3);
                }
            }
            Err(e) => println!("   ❌ Error finding books: {}", e),
        }
    } else {
        println!("\n2. Credentials not provided - skipping validation");
        println!("   💡 Use --device-id and --user-idx to test credentials");
    }
    
    println!("\n🎯 Diagnostics complete!");
    Ok(())
}

async fn validate_credentials(config: &Config) -> Result<()> {
    let cred_manager = CredentialManager::new();
    cred_manager.validate(&config.device_id, &config.user_idx).await
        .context("Invalid credentials")
}

fn load_or_create_config(args: &Args) -> miette::Result<Config> {
    let config_path = args.config_path.clone()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".ridiculous.toml"));
    
    let mut config = if config_path.exists() {
        let content = fs::read_to_string(&config_path).into_diagnostic()?;
        toml::from_str(&content).into_diagnostic()?
    } else {
        Config::default()
    };
    
    // Override with CLI args
    if let Some(device_id) = &args.device_id {
        config.device_id = device_id.clone();
    }
    if let Some(user_idx) = &args.user_idx {
        config.user_idx = user_idx.clone();
    }
    if let Some(output_dir) = &args.output_dir {
        config.output_directory = Some(output_dir.to_string_lossy().to_string());
    }
    config.verbose = args.verbose;
    config.organize_output = args.organize;
    
    // Validate required fields
    if config.device_id.is_empty() || config.user_idx.is_empty() {
        return Err(miette!(
            "Missing credentials. Run with --device-id and --user-idx or use config file.\n\
             Get credentials from: https://account.ridibooks.com/api/user-devices/app"
        ));
    }
    
    Ok(config)
}

fn load_processing_state() -> Result<ProcessingState> {
    let state_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ridiculous_state.json");
    
    if state_path.exists() {
        let content = fs::read_to_string(state_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(ProcessingState::default())
    }
}

fn save_processing_state(state: &ProcessingState) -> Result<()> {
    let state_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ridiculous_state.json");
    
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(state)?;
    fs::write(state_path, content)?;
    Ok(())
}

fn print_summary(state: &ProcessingState) {
    println!("\n📊 Processing Summary:");
    println!("   ✅ Completed: {}", state.completed.len());
    println!("   ❌ Failed: {}", state.failed.len());
    
    if !state.failed.is_empty() {
        println!("\n❌ Failed books:");
        for (book_id, error) in &state.failed {
            println!("   - {}: {}", book_id, error);
        }
        println!("\n💡 Use --resume to retry failed books");
    }
}