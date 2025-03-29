use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::engine::{general_purpose, Engine};
use tracing::debug;
use chrono::Utc;
use rand::Rng;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

/// Authentication utilities for Tencent Meeting API
///
/// This module encapsulates authentication logic for the Tencent Meeting API,
/// providing methods for generating signatures, timestamps, and nonces.
/// It implements the AKSK (AppId, SecretId, SecretKey) authentication method
/// required by Tencent Meeting API.
///
/// # Examples
///
/// ```
/// use tencent_meeting_service::auth::TencentAuth;
///
/// // Generate signature for a request
/// let signature = TencentAuth::generate_signature(
///     "secret_id",
///     "secret_key",
///     "GET",
///     "/v1/meeting-rooms?page=1",
///     1679452800,
///     "12345678",
///     ""
/// );
///
/// // Get current timestamp
/// let timestamp = TencentAuth::get_timestamp();
///
/// // Generate a random nonce
/// let nonce = TencentAuth::generate_nonce();
/// ```
pub struct TencentAuth;

impl TencentAuth {
    /// Generate a random nonce for API requests
    ///
    /// Returns an 8-digit random number as a string.
    /// This ensures request uniqueness and helps prevent replay attacks.
    pub fn generate_nonce() -> String {
        rand::thread_rng().gen_range(10000000..99999999).to_string()
    }
    
    /// Get current timestamp for API requests
    ///
    /// Returns the current Unix timestamp (seconds since epoch).
    /// Used for request freshness validation.
    pub fn get_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    /// Generate signature for Tencent Meeting API requests
    ///
    /// Creates a signature using HMAC-SHA256 following Tencent Meeting API requirements.
    /// The signature is created from the following components:
    /// - HTTP method (GET, POST, etc.)
    /// - Header string containing key, nonce and timestamp
    /// - URI including query parameters
    /// - Request body
    ///
    /// # Arguments
    ///
    /// * `secret_id` - The API secret ID for authentication
    /// * `secret_key` - The API secret key for signature generation
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `uri` - Request URI including query parameters
    /// * `timestamp` - Unix timestamp
    /// * `nonce` - Random nonce string
    /// * `body` - Request body content (empty for GET requests)
    ///
    /// # Returns
    ///
    /// A Base64-encoded string containing the hex representation of the HMAC-SHA256 signature
    pub fn generate_signature(
        secret_id: &str,
        secret_key: &str,
        method: &str, 
        uri: &str, 
        timestamp: i64, 
        nonce: &str, 
        body: &str
    ) -> String {
        // Format the header string part as required by Tencent Meeting API
        let header_string = format!(
            "X-TC-Key={}&X-TC-Nonce={}&X-TC-Timestamp={}",
            secret_id, nonce, timestamp
        );
        
        // Format the full string to sign
        let content = format!(
            "{}\n{}\n{}\n{}",
            method, header_string, uri, body
        );
        
        debug!("String to sign: {}", content);
        
        // Generate HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(content.as_bytes());
        
        // Convert to hex string
        let hex_hash = hex::encode(mac.finalize().into_bytes());
        
        // Base64 encode the hex string
        general_purpose::STANDARD.encode(hex_hash.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_nonce() {
        let nonce = TencentAuth::generate_nonce();
        assert!(nonce.len() == 8);
        assert!(nonce.parse::<u64>().is_ok());
    }
    
    #[test]
    fn test_get_timestamp() {
        let timestamp = TencentAuth::get_timestamp();
        assert!(timestamp > 0);
    }
    
    #[test]
    fn test_generate_signature() {
        let secret_id = "test_secret_id";
        let secret_key = "test_secret_key";
        let method = "GET";
        let uri = "/v1/test";
        let timestamp = 1677721600; // 2023-03-02T00:00:00Z
        let nonce = "12345678";
        let body = "";
        
        let signature = TencentAuth::generate_signature(
            secret_id, secret_key, method, uri, timestamp, nonce, body
        );
        
        // The signature should be a non-empty string
        assert!(!signature.is_empty());
        
        // Basic validation that it's a valid base64 string
        assert!(general_purpose::STANDARD.decode(&signature).is_ok());
    }
}