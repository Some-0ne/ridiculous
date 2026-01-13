use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::types::{Config, BookInfo};
use crate::library_finder::LibraryFinder;

#[derive(Default, PartialEq)]
enum AppState {
    #[default]
    Setup,
    Discovering,
    Ready,
    Decrypting,
    Complete,
}

struct DecryptionProgress {
    current: usize,
    total: usize,
    current_book: String,
    successful: usize,
    failed: usize,
    is_complete: bool,
    errors: Vec<(String, String)>, // (book_name, error_message)
}

impl Default for DecryptionProgress {
    fn default() -> Self {
        Self {
            current: 0,
            total: 0,
            current_book: String::new(),
            successful: 0,
            failed: 0,
            is_complete: false,
            errors: Vec::new(),
        }
    }
}

pub struct RidiculousApp {
    // Configuration
    device_id: String,
    user_idx: String,
    library_path: String,

    // State
    state: AppState,
    books: Vec<BookInfo>,
    selected_books: Vec<bool>,

    // Progress tracking (wrapped in Arc<Mutex> for thread safety)
    progress: Arc<Mutex<DecryptionProgress>>,

    // Results
    error_message: String,
}

impl Default for RidiculousApp {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            user_idx: String::new(),
            library_path: String::new(),
            state: AppState::Setup,
            books: Vec::new(),
            selected_books: Vec::new(),
            progress: Arc::new(Mutex::new(DecryptionProgress::default())),
            error_message: String::new(),
        }
    }
}

impl RidiculousApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn discover_books(&mut self) {
        self.state = AppState::Discovering;
        self.books.clear();
        self.error_message.clear();

        let config = Config {
            device_id: self.device_id.clone(),
            user_idx: self.user_idx.clone(),
            verbose: false,
            organize_output: false,
            backup_originals: false,
            output_directory: None,
            library_path: if self.library_path.is_empty() {
                None
            } else {
                Some(self.library_path.clone())
            },
            max_retries: 3,
            timeout_seconds: 30,
        };

        let finder = LibraryFinder::new();

        // Scan for books
        match finder.find_books(&config) {
            Ok(books) => {
                if books.is_empty() {
                    self.error_message = "No books found in library.".to_string();
                    self.state = AppState::Setup;
                } else {
                    // Filter out already-decrypted books (just like CLI does)
                    let books_to_decrypt: Vec<BookInfo> = books.into_iter()
                        .filter(|book| !book.is_already_decrypted(&config))
                        .collect();

                    if books_to_decrypt.is_empty() {
                        self.error_message = "All books are already decrypted!".to_string();
                        self.state = AppState::Setup;
                    } else {
                        self.books = books_to_decrypt;
                        self.selected_books = vec![true; self.books.len()];
                        self.state = AppState::Ready;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Error scanning library: {}", e);
                self.state = AppState::Setup;
            }
        }
    }

    fn start_decryption(&mut self, ctx: egui::Context) {
        if self.device_id.is_empty() || self.user_idx.is_empty() {
            self.error_message = "Please enter both Device ID and User Index".to_string();
            return;
        }

        // Get selected books
        let books_to_decrypt: Vec<BookInfo> = self.books.iter()
            .zip(self.selected_books.iter())
            .filter_map(|(book, &selected)| if selected { Some(book.clone()) } else { None })
            .collect();

        if books_to_decrypt.is_empty() {
            self.error_message = "No books selected".to_string();
            return;
        }

        // Reset progress
        {
            let mut progress = self.progress.lock().unwrap();
            progress.current = 0;
            progress.total = books_to_decrypt.len();
            progress.successful = 0;
            progress.failed = 0;
            progress.is_complete = false;
            progress.errors.clear();
        }

        self.state = AppState::Decrypting;
        self.error_message.clear();

        let progress = Arc::clone(&self.progress);
        let device_id = self.device_id.clone();
        let user_idx = self.user_idx.clone();
        let output_dir = if self.library_path.is_empty() {
            None
        } else {
            Some(self.library_path.clone())
        };

        // Spawn background thread for decryption
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            for (i, book) in books_to_decrypt.iter().enumerate() {
                // Update current book name
                {
                    let mut p = progress.lock().unwrap();
                    p.current_book = book.get_display_name();
                }

                // Decrypt book (calling the actual decryption function)
                let result = rt.block_on(async {
                    decrypt_single_book(book, &device_id, &user_idx, output_dir.as_deref()).await
                });

                // Update progress
                {
                    let mut p = progress.lock().unwrap();
                    p.current = i + 1;
                    match result {
                        Ok(_) => p.successful += 1,
                        Err(e) => {
                            p.failed += 1;
                            p.errors.push((book.get_display_name(), e.to_string()));
                        }
                    }
                }

                // Request repaint
                ctx.request_repaint();
            }

            // Mark as complete
            {
                let mut p = progress.lock().unwrap();
                p.is_complete = true;
            }

            ctx.request_repaint();
        });
    }
}

// Simplified decryption function for GUI
async fn decrypt_single_book(
    book: &BookInfo,
    device_id: &str,
    _user_idx: &str,
    output_dir: Option<&str>
) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::fs;
    use std::io::Read as _;

    // Check if book file is already in plaintext (valid zip)
    if !book.is_v11 && !book.book_filename.contains(".v") {
        let book_path = book.get_book_file_path();
        if book_path.exists() {
            if let Ok(file) = fs::File::open(&book_path) {
                if let Ok(zip) = zip::ZipArchive::new(file) {
                    if zip.len() > 0 {
                        // It's already a valid zip/epub, skip it
                        return Ok(());
                    }
                }
            }
        }
    }

    // Check if already decrypted in output location
    let output_path = if let Some(dir) = output_dir {
        PathBuf::from(dir).join(book.get_output_filename())
    } else if let Some(library_path) = book.path.parent() {
        library_path.join(book.get_output_filename())
    } else {
        book.path.join(book.get_output_filename())
    };

    if output_path.exists() {
        return Ok(()); // Skip already decrypted books
    }

    // Read .dat file
    let dat_path = book.get_data_file_path();
    let mut dat_file = fs::File::open(&dat_path)
        .with_context(|| format!("Failed to open .dat file: {}", dat_path.display()))?;

    let mut dat_data = Vec::new();
    dat_file.read_to_end(&mut dat_data)?;

    // Extract key from .dat file
    let key = extract_key_from_dat(&dat_data, device_id)
        .with_context(|| format!("Key extraction failed. Device ID: {}, .dat file size: {} bytes", device_id, dat_data.len()))?;

    // Decrypt book
    let decrypted_content = if book.is_v11 {
        decrypt_v11_book(book, &key)?
    } else {
        decrypt_v1_book(book, &key)?
    };

    // Write decrypted book
    fs::write(&output_path, &decrypted_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    Ok(())
}

fn extract_key_from_dat(dat_data: &[u8], device_id: &str) -> anyhow::Result<[u8; 16]> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};

    if dat_data.len() < 32 {
        anyhow::bail!("Invalid .dat file: too small");
    }

    let key_bytes = device_id.as_bytes();
    let mut key = [0u8; 16];
    let len = key_bytes.len().min(16);
    key[..len].copy_from_slice(&key_bytes[..len]);

    let mut iv = [0u8; 16];
    iv.copy_from_slice(&dat_data[0..16]);

    let mut encrypted = dat_data[16..].to_vec();

    let decrypted = cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut encrypted)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt .dat file"))?;

    // Convert to UTF-8 string (the .dat contains text data)
    let plaintext_str = std::str::from_utf8(decrypted)
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in decrypted data"))?;

    if plaintext_str.len() < 84 {
        anyhow::bail!("Decrypted .dat data too short: {} chars", plaintext_str.len());
    }

    // Extract key from characters 68-84 (not bytes!)
    let mut book_key = [0u8; 16];
    let key_slice = &plaintext_str[68..84];
    let key_bytes = key_slice.as_bytes();
    let copy_len = std::cmp::min(16, key_bytes.len());
    book_key[..copy_len].copy_from_slice(&key_bytes[..copy_len]);

    Ok(book_key)
}

fn decrypt_v1_book(book: &BookInfo, key: &[u8; 16]) -> anyhow::Result<Vec<u8>> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use anyhow::Context;
    use std::fs;
    use std::io::Read as _;

    let book_path = book.get_book_file_path();
    let mut file = fs::File::open(&book_path)
        .with_context(|| format!("Failed to open book file: {}", book_path.display()))?;

    let mut encrypted_data = Vec::new();
    file.read_to_end(&mut encrypted_data)?;

    if encrypted_data.len() < 16 {
        anyhow::bail!("Book file too small");
    }

    let mut iv = [0u8; 16];
    iv.copy_from_slice(&encrypted_data[0..16]);

    let mut encrypted = encrypted_data[16..].to_vec();

    let decrypted = cbc::Decryptor::<aes::Aes128>::new(key.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut encrypted)
        .map_err(|e| anyhow::anyhow!("Book decryption failed: {}. Wrong device_id for this book? Try credentials from the device where the book was downloaded.", e))?;

    Ok(decrypted.to_vec())
}

fn decrypt_v11_book(book: &BookInfo, key: &[u8; 16]) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;
    use std::fs;
    use std::io::{Read as _, Write as _};
    use zip::ZipArchive;

    let book_file_path = book.get_book_file_path();
    let book_file = fs::File::open(&book_file_path)
        .with_context(|| format!("Failed to open v11 book file: {}", book_file_path.display()))?;

    let mut zip = ZipArchive::new(book_file)
        .context("Failed to read v11 book as ZIP")?;

    // Create output ZIP in memory
    let mut output_buffer = Vec::new();
    {
        let mut output_zip = zip::ZipWriter::new(std::io::Cursor::new(&mut output_buffer));

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let file_name = file.name().to_string();

            // Read encrypted file data
            let mut encrypted_data = Vec::new();
            file.read_to_end(&mut encrypted_data)?;
            drop(file);

            // Decrypt the file
            let decrypted_data = match decrypt_v11_file_content(&encrypted_data, key) {
                Ok(data) => data,
                Err(_) => encrypted_data // Keep original if decryption fails
            };

            // Write to output ZIP
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            output_zip.start_file(&file_name, options)?;
            output_zip.write_all(&decrypted_data)?;
        }

        output_zip.finish()?;
    }

    Ok(output_buffer)
}

fn decrypt_v11_file_content(encrypted_data: &[u8], key: &[u8; 16]) -> anyhow::Result<Vec<u8>> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};

    if encrypted_data.len() < 16 {
        anyhow::bail!("File too small for v11 decryption: {} bytes", encrypted_data.len());
    }

    let mut iv = [0; 16];
    iv.copy_from_slice(&encrypted_data[0..16]);
    let mut encrypted = encrypted_data[16..].to_vec();

    let decrypted = cbc::Decryptor::<aes::Aes128>::new(key.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut encrypted)
        .map_err(|e| anyhow::anyhow!("V11 file decryption failed: {}. Wrong device_id for this book?", e))?;

    Ok(decrypted.to_vec())
}

impl eframe::App for RidiculousApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🔓 Ridiculous - RIDI Book Decryption");
            ui.add_space(10.0);

            match self.state {
                AppState::Setup => {
                    ui.label("Welcome! Enter your RIDI credentials to get started.");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Device ID:");
                        ui.text_edit_singleline(&mut self.device_id)
                            .on_hover_text("Your RIDI device identifier");
                    });

                    ui.horizontal(|ui| {
                        ui.label("User Index:");
                        ui.text_edit_singleline(&mut self.user_idx)
                            .on_hover_text("Your RIDI user index number");
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Library Path:");
                        ui.text_edit_singleline(&mut self.library_path)
                            .on_hover_text("Custom library location (optional)");
                        if ui.button("📁 Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.library_path = path.display().to_string();
                            }
                        }
                    });

                    ui.label("(Leave empty to auto-detect)");
                    ui.add_space(20.0);

                    if !self.error_message.is_empty() {
                        ui.colored_label(egui::Color32::RED, &self.error_message);
                        ui.add_space(10.0);
                    }

                    let can_find = !self.device_id.is_empty() && !self.user_idx.is_empty();
                    if ui.add_enabled(can_find, egui::Button::new("🔍 Find Books")).clicked() {
                        self.discover_books();
                    }

                    if !can_find {
                        ui.label("⚠️ Please enter credentials first");
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label("📝 How to get your credentials:");
                    ui.label("1. Go to https://ridibooks.com and log in");
                    ui.label("2. Visit https://account.ridibooks.com/api/user-devices/app");
                    ui.label("3. Copy your device_id and user_idx from the JSON response");
                }

                AppState::Discovering => {
                    ui.spinner();
                    ui.label("🔎 Searching for books...");
                }

                AppState::Ready => {
                    ui.label(format!("📚 Found {} books in your library", self.books.len()));
                    ui.add_space(10.0);

                    ui.label("Select books to decrypt:");

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (i, book) in self.books.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.selected_books[i], "");
                                    let label = format!(
                                        "{} ({})",
                                        book.get_display_name(),
                                        if book.is_v11 { "v11 DRM" } else { "v1 DRM" }
                                    );
                                    ui.label(label);
                                });
                            }
                        });

                    ui.add_space(20.0);

                    let selected_count = self.selected_books.iter().filter(|&&s| s).count();

                    ui.horizontal(|ui| {
                        if ui.button("✅ Select All").clicked() {
                            self.selected_books.iter_mut().for_each(|s| *s = true);
                        }
                        if ui.button("❌ Deselect All").clicked() {
                            self.selected_books.iter_mut().for_each(|s| *s = false);
                        }
                    });

                    ui.add_space(10.0);

                    if selected_count == 0 {
                        ui.label("⚠️ Please select at least one book to decrypt");
                    } else {
                        if ui.button(format!("🔓 Decrypt {} Book{}", selected_count, if selected_count == 1 { "" } else { "s" })).clicked() {
                            self.start_decryption(ctx.clone());
                        }
                    }

                    ui.add_space(20.0);

                    if ui.button("⬅️ Back").clicked() {
                        self.state = AppState::Setup;
                    }
                }

                AppState::Decrypting => {
                    let (current, total, current_book, is_complete) = {
                        let p = self.progress.lock().unwrap();
                        (p.current, p.total, p.current_book.clone(), p.is_complete)
                    };

                    if is_complete {
                        self.state = AppState::Complete;
                    } else {
                        ui.label("🔄 Decrypting books...");
                        ui.add_space(10.0);

                        let progress = if total > 0 {
                            current as f32 / total as f32
                        } else {
                            0.0
                        };

                        ui.add(egui::ProgressBar::new(progress)
                            .show_percentage()
                            .text(format!("{} / {}", current, total)));

                        if !current_book.is_empty() {
                            ui.add_space(10.0);
                            ui.label(format!("📖 Current: {}", current_book));
                        }

                        ctx.request_repaint();
                    }
                }

                AppState::Complete => {
                    let (successful, failed, errors) = {
                        let p = self.progress.lock().unwrap();
                        (p.successful, p.failed, p.errors.clone())
                    };

                    ui.heading("✅ Decryption Complete!");
                    ui.add_space(20.0);

                    ui.label(format!("✅ Successfully decrypted: {}", successful));
                    if failed > 0 {
                        ui.colored_label(egui::Color32::RED, format!("❌ Failed: {}", failed));
                    } else {
                        ui.label(format!("❌ Failed: {}", failed));
                    }

                    // Show error details if there are any failures
                    if !errors.is_empty() {
                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::RED, "Error Details:");
                        ui.add_space(5.0);

                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for (book_name, error_msg) in &errors {
                                    ui.horizontal(|ui| {
                                        ui.label("❌");
                                        ui.label(format!("{}: {}", book_name, error_msg));
                                    });
                                    ui.add_space(5.0);
                                }
                            });
                    }

                    ui.add_space(20.0);

                    if ui.button("🔄 Decrypt More Books").clicked() {
                        self.state = AppState::Setup;
                        self.books.clear();
                        self.selected_books.clear();
                    }
                }
            }
        });
    }
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 550.0])
            .with_min_inner_size([550.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Ridiculous - RIDI Book Decryption",
        options,
        Box::new(|cc| Ok(Box::new(RidiculousApp::new(cc)))),
    )
}
