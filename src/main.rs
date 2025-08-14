use aes::cipher::BlockDecryptMut;
use aes::cipher::KeyIvInit;
use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use miette::{IntoDiagnostic, miette};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
    failed: Vec<(String, String)>,
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

    if args.diagnose {
        return run_diagnostics(&args).await;
    }

    if args.validate_only {
        let config = load_or_create_config(&args)?;
        return validate_credentials(&config).await;
    }

    let config = load_or_create_config(&args)?;
    let mut state = if args.resume {
        load_processing_state().unwrap_or_default()
    } else {
        ProcessingState::default()
    };

    let library_finder = LibraryFinder::new();
    let books = library_finder.find_books(&config)?;

    if books.is_empty() {
        println!("❌ No books found. Make sure RIDI is installed and books are downloaded.");
        return Ok(());
    }

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

    for handle in handles {
        let (book_id, result) = handle.await.unwrap();
        match result {
            Ok(_) => state.completed.push(book_id),
            Err(e) => state.failed.push((book_id, e.to_string())),
        }

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

                println!("Continue with next book? (y/n)");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).into_diagnostic()?;
                if input.trim().to_lowercase() != "y" {
                    break;
                }
            }
        }

        save_processing_state(state)?;
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

    let mut retries = 3;
    while retries > 0 {
        match decrypt_book_with_original_logic(book, &config.device_id, pb).await {
            Ok(_) => return Ok(()),
            Err(e) if retries > 1 && is_retryable_error(&e) => {
                pb.set_message(&format!("Retrying... ({} attempts left)", retries - 1));
                retries -= 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}

async fn decrypt_book_with_original_logic(
    book: &BookInfo, 
    device_id: &str, 
    pb: &ProgressBar
) -> Result<()> {
    pb.set_message("Extracting decryption key...");
    pb.set_position(20);

    let key = decrypt_key(book, device_id)?;

    pb.set_message("Decrypting book content...");
    pb.set_position(50);

    let decrypted_content = decrypt_book_content(book, &key)?;

    pb.set_message("Writing decrypted file...");
    pb.set_position(80);

    let output_path = get_output_path(book)?;

    if let Some(parent) =