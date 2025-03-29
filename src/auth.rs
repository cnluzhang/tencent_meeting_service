use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::engine::{general_purpose, Engine};
use tracing::debug;
use chrono::Utc;
use rand::Rng;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

/// Authentication utilities for Tencent Meeting API
pub struct TencentAuth;

impl TencentAuth {
    /// Generate a random nonce for API requests
    pub fn generate_nonce() -> String {
        rand::thread_rng().gen_range(10000000..99999999).to_string()
    }
    
    /// Get current timestamp for API requests
    pub fn get_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    /// Generate signature for Tencent Meeting API requests
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