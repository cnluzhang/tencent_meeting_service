use reqwest::Client;
use dotenv::dotenv;
use std::env;
use serde::{Serialize, Deserialize};
use tracing::{info, debug};

// Using fully qualified path for auth module
use crate::auth::TencentAuth;

// Meeting room response types
#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingRoomItem {
    pub meeting_room_id: String,
    pub meeting_room_name: String,
    pub meeting_room_location: String,
    pub account_new_type: i32,
    pub account_type: i32,
    pub active_code: String,
    pub participant_number: i32,
    pub meeting_room_status: i32,
    pub scheduled_status: i32,
    pub is_allow_call: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingRoomsResponse {
    pub total_count: i32,
    pub current_size: i32,
    pub current_page: i32,
    pub total_page: i32,
    pub meeting_room_list: Vec<MeetingRoomItem>,
}

/// Client for Tencent Meeting API
pub struct TencentMeetingClient {
    client: Client,
    app_id: String,
    secret_id: String,
    secret_key: String,
    endpoint: String,
    sdk_id: String,
    operator_id: String,
}

impl TencentMeetingClient {
    /// Create a new Tencent Meeting client from environment variables
    pub fn new() -> Self {
        dotenv().ok();
        
        Self {
            client: Client::new(),
            app_id: env::var("TENCENT_MEETING_APP_ID")
                .expect("TENCENT_MEETING_APP_ID must be set in environment"),
            secret_id: env::var("TENCENT_MEETING_SECRET_ID")
                .expect("TENCENT_MEETING_SECRET_ID must be set in environment"),
            secret_key: env::var("TENCENT_MEETING_SECRET_KEY")
                .expect("TENCENT_MEETING_SECRET_KEY must be set in environment"),
            endpoint: env::var("TENCENT_MEETING_API_ENDPOINT")
                .unwrap_or_else(|_| "https://api.meeting.qq.com".to_string()),
            sdk_id: env::var("TENCENT_MEETING_SDK_ID")
                .unwrap_or_default(),
            operator_id: env::var("TENCENT_MEETING_OPERATOR_ID")
                .unwrap_or_else(|_| "admin".to_string()),
        }
    }

    /// Generate signature for Tencent Meeting API requests
    fn generate_signature(&self, method: &str, uri: &str, timestamp: i64, nonce: &str, body: &str) -> String {
        TencentAuth::generate_signature(
            &self.secret_id,
            &self.secret_key,
            method,
            uri,
            timestamp,
            nonce,
            body
        )
    }

    /// List meeting rooms from the Tencent Meeting API
    pub async fn list_rooms(&self, page: usize, page_size: usize) -> Result<MeetingRoomsResponse, reqwest::Error> {
        let method = "GET";
        let uri = "/v1/meeting-rooms";
        let query = format!(
            "?page={}&page_size={}&operator_id={}&operator_id_type=1", 
            page, page_size, &self.operator_id
        );
        let full_uri = format!("{}{}", uri, query);
        let url = format!("{}{}", self.endpoint, full_uri);

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();
        let request_body = "";  // Empty for GET request
        
        let signature = self.generate_signature(method, &full_uri, timestamp, &nonce, request_body);

        info!("Making request to list meeting rooms");
        debug!("API URL: {}", url);
        
        // Build the request with all required headers
        let mut request = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("X-TC-Key", &self.secret_id)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Nonce", &nonce)
            .header("X-TC-Signature", signature)
            .header("AppId", &self.app_id)
            .header("X-TC-Registered", "1");
            
        // Add SdkId header if not empty
        if !self.sdk_id.is_empty() {
            request = request.header("SdkId", &self.sdk_id);
        }

        // Send the request
        let res = request.send().await?;
        info!("Response received with status: {}", res.status());
        
        let response = res.json::<MeetingRoomsResponse>().await?;
        Ok(response)
    }
}
