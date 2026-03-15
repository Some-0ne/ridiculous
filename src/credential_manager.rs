use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub struct CredentialManager {
    client: Client,
}

impl CredentialManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("ridiculous/0.3.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Extracts device_id and user_idx from the Ridibooks Sentry scope file
    ///
    /// The Ridibooks Electron app logs breadcrumbs to a local Sentry file.
    /// After opening any book, both device_id and user_idx are reliably present.
    pub fn extract_credentials_from_sentry() -> Result<(String, String)> {
        let sentry_path = Self::get_sentry_scope_path()?;

        if !sentry_path.exists() {
            return Err(anyhow::anyhow!(
                "❌ Sentry scope file not found at: {}\n\
                 \n\
                 💡 To fix this:\n\
                 1. Make sure the Ridibooks app is installed\n\
                 2. Open the Ridibooks app\n\
                 3. Open and close any book in your library\n\
                 4. Try running this tool again\n\
                 \n\
                 The device ID is automatically logged when you open a book.",
                sentry_path.display()
            ));
        }

        let content = fs::read_to_string(&sentry_path)
            .with_context(|| format!("Failed to read Sentry scope file: {}", sentry_path.display()))?;

        let json: Value = serde_json::from_str(&content)
            .context("Failed to parse Sentry scope file as JSON")?;

        // Extract user_idx from _user.id
        let user_idx = json.get("_user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!(
                "❌ No user_idx found in Sentry scope file\n\
                 💡 Open and close a book in the Ridibooks app first."
            ))?;

        // Navigate to _breadcrumbs array
        let breadcrumbs = json.get("_breadcrumbs")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!(
                "❌ No breadcrumbs found in Sentry scope file\n\
                 💡 Open and close a book in the Ridibooks app first."
            ))?;

        // Find the breadcrumb with http category and reading-data-api URL
        for breadcrumb in breadcrumbs {
            let category = breadcrumb.get("category")
                .and_then(|v| v.as_str());

            let url = breadcrumb.get("data")
                .and_then(|d| d.get("url"))
                .and_then(|v| v.as_str());

            if category == Some("http") && url.map_or(false, |u| u.contains("reading-data-api.ridibooks.com/progress/positions")) {
                // Extract device_id from http.query field
                // Note: The key is literally "http.query" (with a dot in the key name)
                if let Some(query) = breadcrumb.get("data")
                    .and_then(|d| d.get("http.query"))
                    .and_then(|q| q.as_str())
                {
                    // Parse query string to find device_id parameter
                    for param in query.split('&') {
                        if let Some(device_id) = param.strip_prefix("device_id=") {
                            return Ok((device_id.to_string(), user_idx));
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "❌ No device_id found in Sentry breadcrumbs\n\
             \n\
             💡 To fix this:\n\
             1. Open the Ridibooks app\n\
             2. Open and close any book in your library\n\
             3. Try running this tool again\n\
             \n\
             The device ID is logged when you open a book."
        ))
    }

    /// Extracts only device_id from the Ridibooks Sentry scope file (for backward compatibility)
    pub fn extract_device_id_from_sentry() -> Result<String> {
        Self::extract_credentials_from_sentry().map(|(device_id, _)| device_id)
    }

    /// Returns the platform-appropriate path to the Sentry scope file
    fn get_sentry_scope_path() -> Result<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
            Ok(home.join("Library/Application Support/Ridibooks/sentry/scope_v2.json"))
        }

        #[cfg(target_os = "windows")]
        {
            let app_data = std::env::var("APPDATA")
                .map(PathBuf::from)
                .or_else(|_| {
                    dirs::home_dir()
                        .map(|h| h.join("AppData").join("Roaming"))
                        .ok_or_else(|| anyhow::anyhow!("Could not determine AppData directory"))
                })?;
            Ok(app_data.join("Ridibooks/sentry/scope_v2.json"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(anyhow::anyhow!(
                "Automatic device ID extraction from Sentry is only supported on macOS and Windows.\n\
                 Please provide device_id manually using --device-id flag."
            ))
        }
    }

    pub async fn validate(&self, device_id: &str, user_idx: &str) -> Result<()> {
        // Validate input format first
        if device_id.len() != 36 {
            return Err(anyhow::anyhow!("Invalid device ID format (expected 36 characters)"));
        }
        
        if user_idx.is_empty() {
            return Err(anyhow::anyhow!("User index cannot be empty"));
        }
        
        let url = "https://account.ridibooks.com/api/user-devices/app";
        
        let response = self.client
            .get(url)
            .header("X-Device-Id", device_id)
            .header("X-User-Idx", user_idx)
            .send()
            .await
            .context("Failed to connect to RIDI API")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Invalid credentials: HTTP {} - Check your device_id and user_idx", 
                response.status()
            ));
        }
        
        let json: Value = response.json().await
            .context("Failed to parse RIDI API response")?;
        
        // Check if response contains valid device data
        if let Some(result) = json.get("result") {
            if result.as_array().map_or(false, |arr| !arr.is_empty()) {
                return Ok(());
            }
        }
        
        Err(anyhow::anyhow!("No valid devices found for these credentials"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_credentials_from_json() {
        // Sample Sentry breadcrumb data (matching actual Ridibooks format)
        let json_data = r#"{
            "_user": {
                "id": "1234567"
            },
            "_breadcrumbs": [
                {
                    "category": "navigation",
                    "data": {
                        "from": "/",
                        "to": "/library"
                    }
                },
                {
                    "category": "http",
                    "data": {
                        "status_code": 204,
                        "url": "https://reading-data-api.ridibooks.com/progress/positions/12345",
                        "http.method": "POST",
                        "http.query": "device_id=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                    },
                    "type": "http"
                }
            ]
        }"#;

        let json: Value = serde_json::from_str(json_data).unwrap();

        // Extract user_idx
        let user_idx = json.get("_user")
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_str())
            .unwrap();

        let breadcrumbs = json.get("_breadcrumbs").unwrap().as_array().unwrap();

        // Extract device_id
        let mut found_device_id = None;
        for breadcrumb in breadcrumbs {
            let category = breadcrumb.get("category").and_then(|v| v.as_str());
            let url = breadcrumb.get("data")
                .and_then(|d| d.get("url"))
                .and_then(|v| v.as_str());

            if category == Some("http") && url.map_or(false, |u| u.contains("reading-data-api.ridibooks.com/progress/positions")) {
                if let Some(query) = breadcrumb.get("data")
                    .and_then(|d| d.get("http.query"))
                    .and_then(|q| q.as_str())
                {
                    for param in query.split('&') {
                        if let Some(device_id) = param.strip_prefix("device_id=") {
                            found_device_id = Some(device_id.to_string());
                            break;
                        }
                    }
                }
            }
        }

        assert_eq!(found_device_id, Some("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx".to_string()));
        assert_eq!(user_idx, "1234567");
    }

    #[test]
    fn test_extract_device_id_no_breadcrumbs() {
        let json_data = r#"{
            "_breadcrumbs": []
        }"#;

        let json: Value = serde_json::from_str(json_data).unwrap();
        let breadcrumbs = json.get("_breadcrumbs").unwrap().as_array().unwrap();

        assert!(breadcrumbs.is_empty());
    }

    #[test]
    fn test_extract_device_id_wrong_category() {
        let json_data = r#"{
            "_breadcrumbs": [
                {
                    "category": "navigation",
                    "data": {
                        "url": "https://reading-data-api.ridibooks.com/progress/positions",
                        "http": {
                            "query": "device_id=test-device-id"
                        }
                    }
                }
            ]
        }"#;

        let json: Value = serde_json::from_str(json_data).unwrap();
        let breadcrumbs = json.get("_breadcrumbs").unwrap().as_array().unwrap();

        // Should not find device_id because category is not "http"
        let mut found_device_id = None;
        for breadcrumb in breadcrumbs {
            let category = breadcrumb.get("category").and_then(|v| v.as_str());
            if category == Some("http") {
                found_device_id = Some("should not reach here".to_string());
            }
        }

        assert_eq!(found_device_id, None);
    }
}