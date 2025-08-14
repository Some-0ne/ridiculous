#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[test]
    fn test_book_format_detection() {
        assert_eq!(BookFormat::from_extension("epub"), BookFormat::Epub);
        assert_eq!(BookFormat::from_extension("PDF"), BookFormat::Pdf);
        assert_eq!(BookFormat::from_extension("unknown"), BookFormat::Unknown);
    }
    
    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_seconds, 30);
        assert!(!config.verbose);
        assert!(config.backup_originals);
        assert!(config.device_id.is_empty());
        assert!(config.user_idx.is_empty());
    }
    
    #[test]
    fn test_library_finder_creation() {
        let finder = LibraryFinder::new();
        assert!(!finder.common_paths.is_empty());
    }
    
    #[tokio::test]
    async fn test_credential_validation_format() {
        let cred_manager = CredentialManager::new();
        
        // Test with invalid device ID format
        let result = cred_manager.validate("invalid", "123").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid device ID format"));
        
        // Test with empty user index
        let result = cred_manager.validate("12345678-1234-1234-1234-123456789012", "").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User index cannot be empty"));
    }
    
    #[test]
    fn test_book_info_paths() {
        let temp_dir = tempdir().unwrap();
        let book_dir = temp_dir.path().join("test_book");
        fs::create_dir_all(&book_dir).unwrap();
        
        // Create test files
        fs::write(book_dir.join("test_book.epub"), b"fake epub content").unwrap();
        fs::write(book_dir.join("test_book.dat"), b"fake dat content").unwrap();
        
        let book = BookInfo::new(book_dir.clone()).unwrap();
        
        assert_eq!(book.id, "test_book");
        assert_eq!(book.format, BookFormat::Epub);
        assert!(book.get_data_file_path().ends_with("test_book.dat"));
        assert!(book.get_book_file_path().ends_with("test_book.epub"));
    }
    
    #[test]
    fn test_processing_state_serialization() {
        let state = ProcessingState {
            completed: vec!["book1".to_string(), "book2".to_string()],
            failed: vec![("book3".to_string(), "Network error".to_string())],
            in_progress: vec!["book4".to_string()],
        };
        
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ProcessingState = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.completed.len(), 2);
        assert_eq!(deserialized.failed.len(), 1);
        assert_eq!(deserialized.in_progress.len(), 1);
    }
    
    #[test]
    fn test_retryable_error_detection() {
        use anyhow::anyhow;
        
        assert!(is_retryable_error(&anyhow!("Connection timeout occurred")));
        assert!(is_retryable_error(&anyhow!("Network unreachable")));
        assert!(is_retryable_error(&anyhow!("Temporary failure")));
        assert!(is_retryable_error(&anyhow!("IO error: broken pipe")));
        assert!(!is_retryable_error(&anyhow!("Authentication failed")));
        assert!(!is_retryable_error(&anyhow!("File not found")));
    }
    
    #[test]
    fn test_file_size_formatting() {
        let temp_dir = tempdir().unwrap();
        let book_dir = temp_dir.path().join("test_book");
        fs::create_dir_all(&book_dir).unwrap();
        
        // Create test file with known size
        let test_data = vec![0u8; 1024]; // 1KB
        fs::write(book_dir.join("test_book.epub"), &test_data).unwrap();
        fs::write(book_dir.join("test_book.dat"), b"test").unwrap();
        
        let book = BookInfo::new(book_dir).unwrap();
        let size_str = book.format_file_size();
        
        assert!(size_str.contains("KB") || size_str.contains("B"));
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config {
            device_id: "test-device-id".to_string(),
            user_idx: "12345".to_string(),
            verbose: true,
            organize_output: true,
            backup_originals: false,
            output_directory: Some("/tmp/books".to_string()),
            max_retries: 5,
            timeout_seconds: 60,
        };
        
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(deserialized.device_id, "test-device-id");
        assert_eq!(deserialized.user_idx, "12345");
        assert!(deserialized.verbose);
        assert!(deserialized.organize_output);
        assert!(!deserialized.backup_originals);
        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.timeout_seconds, 60);
    }
}