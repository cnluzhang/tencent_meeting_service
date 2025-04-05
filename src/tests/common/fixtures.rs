use crate::models::form::{FormEntry, FormField1Item, FormSubmission};
use crate::services::database::MeetingRecord;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde_json::Value;

// Status constants for test fixtures - kept for reference
// and possible future use in helper functions
const _STATUS_RESERVED: &str = "已预约";
const _STATUS_CANCELLED: &str = "已取消";

// This function is currently not used but kept for reference
// This is a good example test fixture for form submissions
#[allow(dead_code)]
pub fn generate_test_form_submission(token: &str, status: &str) -> FormSubmission {
    // Create the form item
    let item = FormField1Item {
        item_name: "Conference Room A".to_string(),
        scheduled_label: "2035-03-30 09:00-10:00".to_string(),
        number: 1,
        scheduled_at: "2035-03-30T01:00:00.000Z".to_string(),
        api_code: "CODE1".to_string(),
    };
    
    // Create extra fields
    let mut extra_fields = HashMap::new();
    extra_fields.insert("user_field_name".to_string(), Value::String("Test User".to_string()));
    extra_fields.insert("department_field_name".to_string(), Value::String("Test Department".to_string()));
    
    // Create the form entry
    let entry = FormEntry {
        token: token.to_string(),
        field_1: vec![item],
        field_8: "Test Meeting".to_string(),
        extra_fields,
        reservation_status_fsf_field: status.to_string(),
    };
    
    // Create the form submission
    FormSubmission {
        form: "test_form".to_string(),
        form_name: "Test Form".to_string(),
        entry,
    }
}

// Used by integration tests for creating sample data
#[allow(dead_code)]
pub fn generate_test_meeting(
    id: &str,
    token: &str,
    status: &str,
    start_time: DateTime<Utc>,
    _end_time: DateTime<Utc>,
) -> MeetingRecord {
    MeetingRecord {
        entry_token: token.to_string(),
        form_id: "test_form".to_string(),
        form_name: "Test Form".to_string(),
        subject: "Test Meeting".to_string(),
        room_name: "Test Room".to_string(),
        scheduled_at: start_time.to_rfc3339(),
        scheduled_label: "2035-03-30 09:00-10:00".to_string(),
        status: status.to_string(),
        meeting_id: id.to_string(),
        room_id: "123456".to_string(),
        created_at: Utc::now().to_rfc3339(),
        cancelled_at: "".to_string(),
        operator_name: "Test Operator".to_string(),
        operator_id: "test_operator".to_string(),
    }
}