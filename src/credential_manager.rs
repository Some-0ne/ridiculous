use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::process::Command;
use aes::cipher::{BlockDecrypt, KeyInit};
use base64::Engine;
use sha1::{Sha1, Digest};

pub struct CredentialManager {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct RidiCredentials {
    pub device_id: String,
    pub user_idx: u64,
}

impl CredentialManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("ridiculous/0.3.5")
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

    /// Extracts credentials from encrypted Settings file and Sentry (permanent storage)
    ///
    /// This is the preferred method as it doesn't require opening a book first.
    /// The device_id is stored in an AES-ECB encrypted Settings file, with the
    /// encryption key stored in the system keychain.
    pub fn extract_credentials_permanent() -> Result<RidiCredentials> {
        // Extract device_id from encrypted Settings file
        let device_id = Self::extract_device_id_from_settings()?;

        // Extract user_idx from Sentry file
        let user_idx = Self::extract_user_idx_from_sentry()?;

        Ok(RidiCredentials {
            device_id,
            user_idx,
        })
    }

    /// Extracts device_id from the encrypted Settings file
    fn extract_device_id_from_settings() -> Result<String> {
        // Step 1: Get encryption key from keychain
        let encryption_key = Self::get_keychain_password()?;

        // Step 2: Read Settings file
        let settings_path = Self::get_settings_file_path()?;
        let encrypted_data = fs::read(&settings_path)
            .with_context(|| format!(
                "Failed to read Settings file: {}\n\
                 💡 Make sure you've logged into the Ridibooks app at least once.",
                settings_path.display()
            ))?;

        // Step 3: Validate file format
        Self::validate_settings_file(&encrypted_data)?;

        // Step 4: Decrypt the data
        let decrypted_json = Self::decrypt_settings(&encrypted_data, &encryption_key)?;

        // Step 5: Parse JSON and extract device_id
        let json: Value = serde_json::from_str(&decrypted_json)
            .context("Failed to parse decrypted Settings as JSON")?;

        let device_id = json.get("data")
            .and_then(|d| d.get("device"))
            .and_then(|d| d.get("deviceId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!(
                "Device ID not found in Settings file.\n\
                 💡 Try logging into the Ridibooks app again."
            ))?;

        Ok(device_id.to_string())
    }

    /// Gets the encryption key from the system keychain
    #[cfg(target_os = "macos")]
    fn get_keychain_password() -> Result<String> {
        let output = Command::new("security")
            .args(&["find-generic-password", "-s", "com.ridi.books", "-a", "global", "-w"])
            .output()
            .context("Failed to run security command. Make sure you're on macOS.")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to get encryption key from keychain.\n\
                 💡 Make sure you've logged into the Ridibooks app at least once.\n\
                 Error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let base64_key = String::from_utf8(output.stdout)
            .context("Keychain output is not valid UTF-8")?
            .trim()
            .to_string();

        // Base64 decode to get the UTF-8 key string
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(&base64_key)
            .context("Failed to base64-decode keychain password")?;

        let key_string = String::from_utf8(key_bytes)
            .context("Decoded keychain password is not valid UTF-8")?;

        Ok(key_string)
    }

    /// Gets the encryption key from Windows Credential Manager via Win32 CredRead API
    #[cfg(target_os = "windows")]
    fn get_keychain_password() -> Result<String> {
        // Electron's keytar stores credentials with target "<service>/<account>".
        // Read via CredReadW (Win32 GENERIC credential, type 1) through a short
        // inline C# P/Invoke embedded in PowerShell — no extra modules required.
        let script = r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WinCred {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct CREDENTIAL {
        public uint    Flags;
        public uint    Type;
        public string  TargetName;
        public string  Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public uint    CredentialBlobSize;
        public IntPtr  CredentialBlob;
        public uint    Persist;
        public uint    AttributeCount;
        public IntPtr  Attributes;
        public string  TargetAlias;
        public string  UserName;
    }
    [DllImport("Advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool CredRead(string target, uint type, int flags, out IntPtr pcred);
    [DllImport("Advapi32.dll")]
    public static extern void CredFree(IntPtr buffer);
    public static string GetPassword(string target) {
        IntPtr p;
        if (!CredRead(target, 1, 0, out p)) return null;
        var c = (CREDENTIAL)Marshal.PtrToStructure(p, typeof(CREDENTIAL));
        var bytes = new byte[(int)c.CredentialBlobSize];
        Marshal.Copy(c.CredentialBlob, bytes, 0, (int)c.CredentialBlobSize);
        CredFree(p);
        return Encoding.Unicode.GetString(bytes);
    }
}
'@
[WinCred]::GetPassword("com.ridi.books/global")
"#;

        let output = Command::new("powershell")
            .args(&["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .context("Failed to run PowerShell command")?;

        let raw = String::from_utf8(output.stdout)
            .context("PowerShell output is not valid UTF-8")?;
        let base64_key = raw.trim().to_string();

        if base64_key.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to get encryption key from Windows Credential Manager.\n\
                 💡 Make sure you've logged into the Ridibooks app at least once.\n\
                 Error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Base64 decode to get the UTF-8 key string
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(&base64_key)
            .context("Failed to base64-decode credential from Credential Manager")?;

        let key_string = String::from_utf8(key_bytes)
            .context("Decoded credential is not valid UTF-8")?;

        Ok(key_string)
    }

    /// Validates the Settings file format
    fn validate_settings_file(data: &[u8]) -> Result<()> {
        if data.len() < 256 {
            return Err(anyhow::anyhow!(
                "Settings file too small ({} bytes, expected at least 256)",
                data.len()
            ));
        }

        // Check magic bytes (0x64617461 = "data")
        let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x64617461 {
            return Err(anyhow::anyhow!(
                "Invalid Settings file magic bytes: 0x{:08x} (expected 0x64617461)",
                magic
            ));
        }

        // Read version
        let version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if version != 4 {
            eprintln!("⚠️  Warning: Unexpected Settings file version {} (expected 4)", version);
        }

        // Validate SHA1 checksum (bytes 108–147 = 40 hex chars covering bytes 256+)
        if data.len() >= 148 {
            let stored_sha1 = std::str::from_utf8(&data[108..148])
                .context("Invalid SHA1 checksum field in Settings file (not UTF-8)")?
                .trim();
            let encrypted_data = &data[256..];
            let actual_sha1 = format!("{:x}", Sha1::digest(encrypted_data));
            if actual_sha1 != stored_sha1 {
                return Err(anyhow::anyhow!(
                    "Settings file SHA1 checksum mismatch — the file may be corrupted.\n\
                     Stored:   {}\n\
                     Computed: {}",
                    stored_sha1, actual_sha1
                ));
            }
        }

        Ok(())
    }

    /// Decrypts the Settings file
    fn decrypt_settings(data: &[u8], key_string: &str) -> Result<String> {
        // Extract encrypted portion (starts at byte 256)
        let encrypted = &data[256..];

        // Process key: UTF-8 encode, PKCS7-pad to next multiple of 16, use first 32 bytes for AES-256
        let key_bytes = key_string.as_bytes();
        let padded_len = ((key_bytes.len() + 15) / 16) * 16;
        let pad_byte = (padded_len - key_bytes.len()) as u8;
        let mut padded_key = key_bytes.to_vec();
        padded_key.resize(padded_len, pad_byte);

        // First 32 bytes of the PKCS7-padded key form the AES-256 key
        if padded_key.len() < 32 {
            return Err(anyhow::anyhow!(
                "Keychain password is too short after padding ({} bytes, need at least 32)",
                padded_key.len()
            ));
        }
        let aes_key: [u8; 32] = padded_key[..32].try_into().unwrap();

        // Decrypt with AES-256-ECB
        let mut decrypted = encrypted.to_vec();

        // ECB mode - decrypt each 16-byte block independently
        let cipher = aes::Aes256::new(&aes_key.into());

        for chunk in decrypted.chunks_mut(16) {
            if chunk.len() == 16 {
                let block = aes::cipher::Block::<aes::Aes256>::from_mut_slice(chunk);
                cipher.decrypt_block(block);
            }
        }

        // Strip PKCS7 padding
        if decrypted.is_empty() {
            return Err(anyhow::anyhow!("Decrypted data is empty"));
        }

        let pad_len = decrypted[decrypted.len() - 1] as usize;
        if pad_len == 0 || pad_len > 16 || pad_len > decrypted.len() {
            return Err(anyhow::anyhow!(
                "Invalid PKCS7 padding: pad_len = {}",
                pad_len
            ));
        }

        decrypted.truncate(decrypted.len() - pad_len);

        // Parse as UTF-8
        let json_str = String::from_utf8(decrypted)
            .context("Decrypted data is not valid UTF-8")?;

        Ok(json_str)
    }

    /// Returns the path to the Settings file
    #[cfg(target_os = "macos")]
    fn get_settings_file_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join("Library/Application Support/Ridibooks/datastores/global/Settings"))
    }

    #[cfg(target_os = "windows")]
    fn get_settings_file_path() -> Result<PathBuf> {
        let app_data = std::env::var("APPDATA")
            .map(PathBuf::from)
            .or_else(|_| {
                dirs::home_dir()
                    .map(|h| h.join("AppData").join("Roaming"))
                    .ok_or_else(|| anyhow::anyhow!("Could not determine AppData directory"))
            })?;
        Ok(app_data.join("Ridibooks/datastores/global/Settings"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn get_settings_file_path() -> Result<PathBuf> {
        Err(anyhow::anyhow!(
            "Settings file extraction is only supported on macOS and Windows"
        ))
    }

    /// Extracts user_idx from Sentry scope file as a u64
    fn extract_user_idx_from_sentry() -> Result<u64> {
        let sentry_path = Self::get_sentry_scope_path()?;

        if !sentry_path.exists() {
            return Err(anyhow::anyhow!(
                "Sentry scope file not found at: {}\n\
                 💡 Make sure you've logged into the Ridibooks app at least once.",
                sentry_path.display()
            ));
        }

        let content = fs::read_to_string(&sentry_path)
            .with_context(|| format!("Failed to read Sentry scope file: {}", sentry_path.display()))?;

        let json: Value = serde_json::from_str(&content)
            .context("Failed to parse Sentry scope file as JSON")?;

        let id_value = json.get("_user")
            .and_then(|u| u.get("id"))
            .ok_or_else(|| anyhow::anyhow!(
                "User ID not found in Sentry scope file.\n\
                 💡 Try logging into the Ridibooks app again."
            ))?;

        // id may be stored as a JSON number or as a numeric string
        let user_idx = id_value.as_u64()
            .or_else(|| id_value.as_str().and_then(|s| s.parse().ok()))
            .ok_or_else(|| anyhow::anyhow!(
                "User ID in Sentry scope file is not a valid u64 (got: {})\n\
                 💡 Try logging into the Ridibooks app again.",
                id_value
            ))?;

        Ok(user_idx)
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