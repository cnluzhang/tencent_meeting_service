use dotenv::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::error::Error;
use std::fmt;
use tracing::{debug, error, info};

// Using fully qualified path for auth module
use crate::auth::TencentAuth;

// Define a custom error type to handle different error scenarios
#[derive(Debug)]
pub struct TencentApiError {
    pub message: String,
}

impl fmt::Display for TencentApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tencent API Error: {}", self.message)
    }
}

impl Error for TencentApiError {}

impl From<serde_json::Error> for TencentApiError {
    fn from(err: serde_json::Error) -> Self {
        TencentApiError {
            message: format!("JSON parsing error: {}", err),
        }
    }
}

// Meeting room response types
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Meeting creation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub userid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_anonymous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guest {
    pub area: String,
    pub phone_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_enable_type_join: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_enable_join: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unmute_self: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_ivr_on_leave: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_ivr_on_join: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_in_before_host: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_in_waiting_room: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_screen_shared_watermark: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_mark_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_enterprise_user_allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_user_join_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_record_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant_join_auto_record: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_host_pause_auto_record: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_multi_device: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_nickname: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_date: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customized_recurring_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customized_recurring_step: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customized_recurring_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveWatermark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_opt: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_live_password: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_live_im: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_live_replay: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_watermark: Option<LiveWatermark>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_addr: Option<String>, // Only in response
}

// Request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMeetingRequest {
    pub userid: String,
    pub instanceid: i32,
    pub subject: String,
    #[serde(skip_serializing)]
    pub type_: i32, // renamed from 'type' which is a reserved keyword
    #[serde(rename = "type")]
    pub _type: i32, // This is to map type_ to "type" in JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guests: Option<Vec<Guest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitees: Option<Vec<User>>,
    pub start_time: String,
    pub end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<MeetingSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_rule: Option<RecurringRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_config: Option<LiveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_doc_upload_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_set_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_interpreter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_enroll: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_host_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_to_wework: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_enterprise_intranet_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingInfo {
    pub subject: String,
    pub meeting_id: String,
    pub meeting_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participants: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_non_registered: Option<Vec<String>>,
    pub start_time: String,
    pub end_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<MeetingSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_config: Option<LiveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMeetingResponse {
    pub meeting_number: i32,
    pub meeting_info_list: Vec<MeetingInfo>,
}

// Meeting cancellation types
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelMeetingRequest {
    pub userid: String,
    pub instanceid: i32,
    pub reason_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_type: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_meeting_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
}

// Meeting room booking types
#[derive(Debug, Serialize, Deserialize)]
pub struct BookRoomsRequest {
    pub operator_id: String,
    pub operator_id_type: i32,
    pub meeting_room_id_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_visible: Option<bool>,
}

// Meeting room release types
#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseRoomsRequest {
    pub operator_id: String,
    pub operator_id_type: i32,
    pub meeting_room_id_list: Vec<String>,
}

use crate::models::meeting::Operator;

/// Client for Tencent Meeting API
pub struct TencentMeetingClient {
    client: Client,
    app_id: String,
    secret_id: String,
    secret_key: String,
    endpoint: String,
    sdk_id: String,
    operators: Vec<Operator>,
    default_operator_id: String,
}

impl Default for TencentMeetingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TencentMeetingClient {
    /// Create a new Tencent Meeting client from environment variables
    pub fn new() -> Self {
        dotenv().ok();

        // Parse the operators from the environment variable
        let operators = Self::parse_operators_from_env();

        // Get the default operator ID (first one or "admin")
        let default_operator_id = operators
            .first()
            .map(|op| op.id.clone())
            .unwrap_or_else(|| "admin".to_string());

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
            sdk_id: env::var("TENCENT_MEETING_SDK_ID").unwrap_or_default(),
            operators,
            default_operator_id,
        }
    }

    /// Parse operators from environment variable
    /// Format: "name1:id1,name2:id2,name3:id3"
    fn parse_operators_from_env() -> Vec<Operator> {
        match env::var("TENCENT_MEETING_OPERATOR_ID") {
            Ok(value) => {
                let mut operators = Vec::new();

                for pair in value.split(',') {
                    let parts: Vec<&str> = pair.trim().split(':').collect();
                    if parts.len() == 2 {
                        operators.push(Operator {
                            name: parts[0].trim().to_string(),
                            id: parts[1].trim().to_string(),
                        });
                    }
                }

                // Log the number of operators loaded
                info!("Loaded {} operators from environment", operators.len());
                operators
            }
            Err(_) => {
                info!("No operators defined in environment, using default");
                vec![Operator {
                    name: "admin".to_string(),
                    id: "admin".to_string(),
                }]
            }
        }
    }

    /// Get the operator ID for a given name
    pub fn get_operator_id_by_name(&self, name: &str) -> String {
        for operator in &self.operators {
            if operator.name.eq_ignore_ascii_case(name) {
                return operator.id.clone();
            }
        }

        // If no match found, return the default operator ID
        info!("No operator found for name '{}', using default", name);
        self.default_operator_id.clone()
    }

    /// Get the default operator ID configured for this client
    pub fn get_operator_id(&self) -> &str {
        &self.default_operator_id
    }

    /// Get all available operators
    pub fn get_operators(&self) -> &[Operator] {
        &self.operators
    }

    /// Generate signature for Tencent Meeting API requests
    fn generate_signature(
        &self,
        method: &str,
        uri: &str,
        timestamp: i64,
        nonce: &str,
        body: &str,
    ) -> String {
        TencentAuth::generate_signature(
            &self.secret_id,
            &self.secret_key,
            method,
            uri,
            timestamp,
            nonce,
            body,
        )
    }

    /// Build a request with all required headers
    fn build_request(
        &self,
        method: reqwest::Method,
        url: &str,
        timestamp: i64,
        nonce: &str,
        signature: &str,
        body: Option<String>,
    ) -> reqwest::RequestBuilder {
        let mut request = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json")
            .header("X-TC-Key", &self.secret_id)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Nonce", nonce)
            .header("X-TC-Signature", signature)
            .header("AppId", &self.app_id)
            .header("X-TC-Registered", "1");

        // Add SdkId header if not empty
        if !self.sdk_id.is_empty() {
            request = request.header("SdkId", &self.sdk_id);
        }

        // Add request body if provided
        if let Some(body_str) = body {
            request = request.body(body_str);
        }

        request
    }

    /// List meeting rooms from the Tencent Meeting API
    pub async fn list_rooms(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<MeetingRoomsResponse, Box<dyn Error + Send + Sync>> {
        let method = "GET";
        let uri = "/v1/meeting-rooms";
        let query = format!(
            "?page={}&page_size={}&operator_id={}&operator_id_type=1",
            page, page_size, &self.default_operator_id
        );
        let full_uri = format!("{}{}", uri, query);
        let url = format!("{}{}", self.endpoint, full_uri);

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();
        let request_body = ""; // Empty for GET request

        let signature = self.generate_signature(method, &full_uri, timestamp, &nonce, request_body);

        info!("Making request to list meeting rooms");
        debug!("API URL: {}", url);

        // Build and send the request
        let request = self.build_request(
            reqwest::Method::GET,
            &url,
            timestamp,
            &nonce,
            &signature,
            None,
        );

        let res = request.send().await?;
        info!("Response received with status: {}", res.status());

        // Get response body for logging
        let status = res.status();
        let response_text = res.text().await?;
        debug!("API Response: {}", response_text);

        // For non-successful responses, log the details before trying to parse
        if !status.is_success() {
            error!("API request failed with status: {}", status);
            error!("Request URL: {}", url);
            error!("Response body: {}", response_text);
            error!("This could be due to incorrect credentials or API parameters");
        }

        // Parse the response JSON using the ? operator for cleaner error handling
        serde_json::from_str::<MeetingRoomsResponse>(&response_text).map_err(|e| {
            error!("Failed to parse response JSON: {}", e);
            Box::new(e) as Box<dyn Error + Send + Sync>
        })
    }

    /// Create a new meeting using the Tencent Meeting API
    pub async fn create_meeting(
        &self,
        meeting_request: &CreateMeetingRequest,
    ) -> Result<CreateMeetingResponse, Box<dyn Error + Send + Sync>> {
        let method = "POST";
        let uri = "/v1/meetings";
        let url = format!("{}{}", self.endpoint, uri);

        // The type field needs proper handling since it's a reserved keyword in Rust
        // We'll use the clone with _type populated correctly from type_ in the request
        let mut request_to_send = meeting_request.clone();
        request_to_send._type = meeting_request.type_; // Copy the value from type_ to _type for serialization

        let request_body = serde_json::to_string(&request_to_send).map_err(|e| {
            Box::<dyn Error + Send + Sync>::from(format!(
                "Failed to serialize meeting request: {}",
                e
            ))
        })?;

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();

        let signature = self.generate_signature(method, uri, timestamp, &nonce, &request_body);

        info!("Making request to create meeting");
        debug!("API URL: {}", url);
        debug!("Request body: {}", request_body);

        // Build and send the request
        let request = self.build_request(
            reqwest::Method::POST,
            &url,
            timestamp,
            &nonce,
            &signature,
            Some(request_body),
        );

        let res = request.send().await?;
        info!("Response received with status: {}", res.status());

        // Get response body for logging
        let status = res.status();
        let response_text = res.text().await?;
        debug!("API Response: {}", response_text);

        // Log any errors but still try to parse the response
        if !status.is_success() {
            error!("Create meeting failed with status: {}", status);
            error!("Response body: {}", response_text);
        }

        serde_json::from_str::<CreateMeetingResponse>(&response_text).map_err(|e| {
            error!("Failed to parse response JSON: {}", e);
            Box::new(e) as Box<dyn Error + Send + Sync>
        })
    }

    /// Cancel a meeting using the Tencent Meeting API
    pub async fn cancel_meeting(
        &self,
        meeting_id: &str,
        cancel_request: &CancelMeetingRequest,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let method = "POST";
        let uri = format!("/v1/meetings/{}/cancel", meeting_id);
        let url = format!("{}{}", self.endpoint, uri);

        let request_body = serde_json::to_string(&cancel_request)
            .expect("Failed to serialize cancellation request");

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();

        let signature = self.generate_signature(method, &uri, timestamp, &nonce, &request_body);

        info!("Making request to cancel meeting {}", meeting_id);
        debug!("API URL: {}", url);

        // Build the request with all required headers
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-TC-Key", &self.secret_id)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Nonce", &nonce)
            .header("X-TC-Signature", signature)
            .header("AppId", &self.app_id)
            .header("X-TC-Registered", "1")
            .body(request_body);

        // Add SdkId header if not empty
        if !self.sdk_id.is_empty() {
            request = request.header("SdkId", &self.sdk_id);
        }

        // Send the request
        let res = request.send().await?;
        info!("Response received with status: {}", res.status());

        // Get response body for logging
        let status = res.status();
        let response_text = res.text().await?;
        debug!("API Response: {}", response_text);

        // Log any errors but let the JSON parsing handle failures
        if !status.is_success() {
            error!("Cancel meeting failed with status: {}", status);
            error!("Response body: {}", response_text);
        }

        // For successful cancellation, the response body is empty
        Ok(())
    }

    /// Book meeting rooms for a meeting using the Tencent Meeting API
    pub async fn book_rooms(
        &self,
        meeting_id: &str,
        book_request: &BookRoomsRequest,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let method = "POST";
        let uri = format!("/v1/meetings/{}/book-rooms", meeting_id);
        let url = format!("{}{}", self.endpoint, uri);

        let request_body =
            serde_json::to_string(&book_request).expect("Failed to serialize book rooms request");

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();

        let signature = self.generate_signature(method, &uri, timestamp, &nonce, &request_body);

        info!("Making request to book rooms for meeting {}", meeting_id);
        debug!("API URL: {}", url);
        debug!("Request body: {}", request_body);

        // Build the request with all required headers
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-TC-Key", &self.secret_id)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Nonce", &nonce)
            .header("X-TC-Signature", signature)
            .header("AppId", &self.app_id)
            .header("X-TC-Registered", "1")
            .body(request_body);

        // Add SdkId header if not empty
        if !self.sdk_id.is_empty() {
            request = request.header("SdkId", &self.sdk_id);
        }

        // Send the request
        let res = request.send().await?;
        info!("Response received with status: {}", res.status());

        // Get response body for logging
        let status = res.status();
        let response_text = res.text().await?;
        debug!("API Response: {}", response_text);

        // Log any errors but let the JSON parsing handle failures
        if !status.is_success() {
            error!("Book rooms failed with status: {}", status);
            error!("Response body: {}", response_text);
        }

        // For successful booking, the response body is typically empty
        Ok(())
    }

    /// Release meeting rooms for a meeting using the Tencent Meeting API
    pub async fn release_rooms(
        &self,
        meeting_id: &str,
        release_request: &ReleaseRoomsRequest,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let method = "POST";
        let uri = format!("/v1/meetings/{}/release-rooms", meeting_id);
        let url = format!("{}{}", self.endpoint, uri);

        let request_body = serde_json::to_string(&release_request)
            .expect("Failed to serialize release rooms request");

        let timestamp = TencentAuth::get_timestamp();
        let nonce = TencentAuth::generate_nonce();

        let signature = self.generate_signature(method, &uri, timestamp, &nonce, &request_body);

        info!("Making request to release rooms for meeting {}", meeting_id);
        debug!("API URL: {}", url);
        debug!("Request body: {}", request_body);

        // Build the request with all required headers
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-TC-Key", &self.secret_id)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Nonce", &nonce)
            .header("X-TC-Signature", signature)
            .header("AppId", &self.app_id)
            .header("X-TC-Registered", "1")
            .body(request_body);

        // Add SdkId header if not empty
        if !self.sdk_id.is_empty() {
            request = request.header("SdkId", &self.sdk_id);
        }

        // Send the request
        let res = request.send().await?;
        info!("Response received with status: {}", res.status());

        // Get response body for logging
        let status = res.status();
        let response_text = res.text().await?;
        debug!("API Response: {}", response_text);

        // Log any errors but let the JSON parsing handle failures
        if !status.is_success() {
            error!("Release rooms failed with status: {}", status);
            error!("Response body: {}", response_text);
        }

        // For successful room release, the response body is typically empty
        Ok(())
    }
}
