use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
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