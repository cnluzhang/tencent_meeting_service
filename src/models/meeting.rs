use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

// Structure to represent a parsed time slot
#[derive(Debug, Clone, Serialize)]
pub struct TimeSlot {
    pub item_name: String,
    pub scheduled_label: String,
    pub number: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub api_code: String,
}

// Response structure for meeting results
#[derive(Debug, Serialize)]
pub struct MeetingResult {
    pub meeting_id: Option<String>,
    pub merged: bool,
    pub room_name: String,
    pub time_slots: Vec<String>,
    pub success: bool,
}

// Response structure for webhook endpoint
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub success: bool,
    pub message: String,
    pub meetings_count: usize,
    pub meetings: Vec<MeetingResult>,
}

// Test data structure for mock responses
#[derive(Debug, Serialize)]
pub struct TestMeetingResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub message: String,
}

// Structure to represent an operator mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub name: String,
    pub id: String,
}
