use axum::response::Json;
use serde::Serialize;
use std::collections::HashMap;
use serde_json;

use crate::client::{
    MeetingRoomItem, 
    MeetingRoomsResponse,
    CreateMeetingRequest, 
    CancelMeetingRequest,
    User,
    MeetingSettings
};
use crate::models::form::{FormSubmission, FormEntry, FormField1Item};

// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

// Test endpoint that returns mock data
pub async fn test_endpoint() -> Json<MeetingRoomsResponse> {
    let mock_room = MeetingRoomItem {
        meeting_room_id: "test-123".to_string(),
        meeting_room_name: "Test Meeting Room".to_string(),
        meeting_room_location: "Test Location".to_string(),
        account_new_type: 1,
        account_type: 1,
        active_code: "TEST-CODE".to_string(),
        participant_number: 10,
        meeting_room_status: 2,
        scheduled_status: 1,
        is_allow_call: true,
    };
    
    let mock_response = MeetingRoomsResponse {
        total_count: 1,
        current_size: 1,
        current_page: 1,
        total_page: 1,
        meeting_room_list: vec![mock_room],
    };
    
    Json(mock_response)
}

// Test data structure for mock responses
#[derive(Debug, Serialize)]
pub struct TestMeetingResponse {
    pub sample_create_request: CreateMeetingRequest,
    pub sample_cancel_request: CancelMeetingRequest,
    pub api_endpoints: Vec<String>,
}

// Test endpoint that returns sample meeting requests
pub async fn test_meetings() -> Json<TestMeetingResponse> {
    // Sample meeting creation request
    let sample_create = CreateMeetingRequest {
        userid: "test_user".to_string(),
        instanceid: 32,
        subject: "Test Meeting".to_string(),
        type_: 0, // Scheduled meeting
        _type: 0, // This will be auto-populated later
        hosts: Some(vec![
            User {
                userid: "test_host".to_string(),
                is_anonymous: None,
                nick_name: None,
            }
        ]),
        invitees: Some(vec![
            User {
                userid: "test_attendee1".to_string(),
                is_anonymous: None, 
                nick_name: None,
            },
            User {
                userid: "test_attendee2".to_string(),
                is_anonymous: None,
                nick_name: None,
            }
        ]),
        start_time: (chrono::Utc::now() + chrono::Duration::hours(1))
            .timestamp().to_string(),
        end_time: (chrono::Utc::now() + chrono::Duration::hours(2))
            .timestamp().to_string(),
        password: Some("123456".to_string()),
        settings: Some(MeetingSettings {
            mute_enable_type_join: Some(2),
            mute_enable_join: Some(true),
            allow_unmute_self: Some(true),
            play_ivr_on_leave: None,
            play_ivr_on_join: None,
            allow_in_before_host: Some(true),
            auto_in_waiting_room: Some(false),
            allow_screen_shared_watermark: Some(false),
            water_mark_type: None,
            only_enterprise_user_allowed: None,
            only_user_join_type: Some(1),
            auto_record_type: None,
            participant_join_auto_record: None,
            enable_host_pause_auto_record: None,
            allow_multi_device: Some(true),
            change_nickname: None,
        }),
        meeting_type: None,
        recurring_rule: None,
        enable_live: None,
        live_config: None,
        enable_doc_upload_permission: None,
        media_set_type: None,
        enable_interpreter: None,
        enable_enroll: None,
        enable_host_key: None,
        host_key: None,
        sync_to_wework: None,
        time_zone: None,
        location: None,
        allow_enterprise_intranet_only: None,
        guests: None,
    };
    
    // Sample meeting cancellation request
    let sample_cancel = CancelMeetingRequest {
        userid: "test_user".to_string(),
        instanceid: 1,
        reason_code: 1,
        meeting_type: None,
        sub_meeting_id: None,
        reason_detail: Some("Test cancellation".to_string()),
    };
    
    // API usage info
    let endpoints = vec![
        "POST /meetings - Create a new meeting".to_string(),
        "POST /meetings/{meeting_id}/cancel - Cancel an existing meeting".to_string(),
        "POST /webhook/form-submission - Webhook endpoint for form submissions".to_string(),
    ];
    
    Json(TestMeetingResponse {
        sample_create_request: sample_create,
        sample_cancel_request: sample_cancel,
        api_endpoints: endpoints,
    })
}

// Sample form submission for testing
#[derive(Debug, Serialize)]
pub struct TestFormSubmission {
    pub single_slot_example: FormSubmission,
    pub multiple_slots_example: FormSubmission,
    pub mergeable_slots_example: FormSubmission,
    pub webhook_endpoint: String,
}

// Test endpoint that returns a sample form submission
pub async fn test_form_submission() -> Json<TestFormSubmission> {
    // Example 1: Single time slot
    let mut extra_fields1 = HashMap::new();
    extra_fields1.insert("user_field".to_string(), serde_json::json!("User Name"));
    extra_fields1.insert("department_field".to_string(), serde_json::json!("Department"));
    
    let single_slot_form = FormSubmission {
        form: "form_id".to_string(),
        form_name: "Meeting Room Reservation".to_string(),
        entry: FormEntry {
            token: "token123".to_string(),
            field_1: vec![
                FormField1Item {
                    item_name: "Conference Room A".to_string(),
                    scheduled_label: "2025-03-30 09:00-10:00".to_string(),
                    number: 1,
                    scheduled_at: "2025-03-30T01:00:00.000Z".to_string(),
                    api_code: "CODE1".to_string(),
                },
            ],
            field_8: "Single Slot Meeting".to_string(),
            extra_fields: extra_fields1,
            reservation_status_fsf_field: "Reserved".to_string(),
        },
    };
    
    // Example 2: Multiple non-contiguous time slots (can't merge)
    let mut extra_fields2 = HashMap::new();
    extra_fields2.insert("user_field".to_string(), serde_json::json!("User Name"));
    extra_fields2.insert("department_field".to_string(), serde_json::json!("Department"));
    
    let multiple_slots_form = FormSubmission {
        form: "form_id".to_string(),
        form_name: "Meeting Room Reservation".to_string(),
        entry: FormEntry {
            token: "token456".to_string(),
            field_1: vec![
                FormField1Item {
                    item_name: "Conference Room A".to_string(),
                    scheduled_label: "2025-03-30 09:00-10:00".to_string(),
                    number: 1,
                    scheduled_at: "2025-03-30T01:00:00.000Z".to_string(),
                    api_code: "CODE1".to_string(),
                },
                FormField1Item {
                    item_name: "Conference Room B".to_string(),  // Different room, can't merge
                    scheduled_label: "2025-03-30 10:00-11:00".to_string(),
                    number: 1,
                    scheduled_at: "2025-03-30T02:00:00.000Z".to_string(),
                    api_code: "CODE2".to_string(),
                },
            ],
            field_8: "Multiple Rooms Meeting".to_string(),
            extra_fields: extra_fields2,
            reservation_status_fsf_field: "Reserved".to_string(),
        },
    };
    
    // Example 3: Multiple contiguous time slots (can merge)
    let mut extra_fields3 = HashMap::new();
    extra_fields3.insert("user_field".to_string(), serde_json::json!("User Name"));
    extra_fields3.insert("department_field".to_string(), serde_json::json!("Department"));
    
    let mergeable_slots_form = FormSubmission {
        form: "form_id".to_string(),
        form_name: "Meeting Room Reservation".to_string(),
        entry: FormEntry {
            token: "token789".to_string(),
            field_1: vec![
                FormField1Item {
                    item_name: "Conference Room A".to_string(),
                    scheduled_label: "2025-03-30 09:00-10:00".to_string(),
                    number: 1,
                    scheduled_at: "2025-03-30T01:00:00.000Z".to_string(),
                    api_code: "CODE1".to_string(),
                },
                FormField1Item {
                    item_name: "Conference Room A".to_string(),  // Same room, contiguous times
                    scheduled_label: "2025-03-30 10:00-11:00".to_string(), 
                    number: 1,
                    scheduled_at: "2025-03-30T02:00:00.000Z".to_string(), // This is 10:00 UTC+8
                    api_code: "CODE1".to_string(),
                },
            ],
            field_8: "Mergeable Time Slots Meeting".to_string(),
            extra_fields: extra_fields3,
            reservation_status_fsf_field: "Reserved".to_string(),
        },
    };
    
    Json(TestFormSubmission {
        single_slot_example: single_slot_form,
        multiple_slots_example: multiple_slots_form,
        mergeable_slots_example: mergeable_slots_form,
        webhook_endpoint: "/webhook/form-submission".to_string(),
    })
}