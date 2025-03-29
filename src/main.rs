mod client;
mod auth;

use axum::{
    routing::{get, post},
    Router,
    extract::{Query, State, Path, Json as ExtractJson},
    http::StatusCode,
    response::Json,
    error_handling::HandleErrorLayer,
};
use serde::{Serialize, Deserialize};
use serde_json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::env;
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    trace::{TraceLayer},
    cors::{CorsLayer, Any},
};
use tracing::{info, error, Level};
use client::{
    TencentMeetingClient, 
    CreateMeetingRequest, 
    CreateMeetingResponse,
    CancelMeetingRequest,
    User,
    MeetingSettings
};
use std::time::Duration;
use chrono::{DateTime, Utc};

// Define pagination query parameters
#[derive(Debug, Deserialize)]
struct PaginationParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

// Form submission data structure
#[derive(Debug, Deserialize, Serialize)]
struct FormField1Item {
    item_name: String,
    scheduled_label: String,
    number: i32,
    scheduled_at: String,
    api_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FormEntry {
    token: String,
    field_1: Vec<FormField1Item>,
    field_8: String,
    #[serde(flatten)]
    extra_fields: std::collections::HashMap<String, serde_json::Value>,
    reservation_status_fsf_field: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FormSubmission {
    form: String,
    form_name: String,
    entry: FormEntry,
}

// Define application state
struct AppState {
    client: TencentMeetingClient,
    #[allow(dead_code)]
    user_field_name: String,  // Reserved for future use
    dept_field_name: String,
}

#[derive(Debug, Serialize)]
struct MeetingResult {
    meeting_id: Option<String>,
    merged: bool,
    room_name: String,
    time_slots: Vec<String>,
    success: bool,
}

#[derive(Debug, Serialize)]
struct WebhookResponse {
    success: bool,
    message: String,
    meetings_count: usize,
    meetings: Vec<MeetingResult>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Initialize the Tencent Meeting API client
    let client = TencentMeetingClient::new();
    
    // Load custom field names from environment
    let user_field_name = env::var("FORM_USER_FIELD_NAME")
        .expect("FORM_USER_FIELD_NAME must be set in environment");
    
    let dept_field_name = env::var("FORM_DEPT_FIELD_NAME")
        .expect("FORM_DEPT_FIELD_NAME must be set in environment");
    
    info!("Using form field mappings from environment variables");
    
    // Create shared application state
    let app_state = Arc::new(AppState { 
        client,
        user_field_name, 
        dept_field_name,
    });

    // Create router with routes
    let app = Router::new()
        .route("/meeting-rooms", get(list_meeting_rooms))
        .route("/meetings", post(create_meeting))
        .route("/meetings/:meeting_id/cancel", post(cancel_meeting))
        .route("/webhook/form-submission", post(handle_form_submission))
        .route("/health", get(health_check))
        .route("/test", get(test_endpoint))
        .route("/test-meetings", get(test_meetings))
        .route("/test-form-submission", get(test_form_submission))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(64)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::new().allow_origin(Any))
        );

    // Bind to port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
        
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

// Test endpoint that returns mock data
async fn test_endpoint() -> Json<client::MeetingRoomsResponse> {
    let mock_room = client::MeetingRoomItem {
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
    
    let mock_response = client::MeetingRoomsResponse {
        total_count: 1,
        current_size: 1,
        current_page: 1,
        total_page: 1,
        meeting_room_list: vec![mock_room],
    };
    
    Json(mock_response)
}

// Test endpoint for meeting creation
#[derive(Debug, Serialize)]
struct TestMeetingResponse {
    sample_create_request: CreateMeetingRequest,
    sample_cancel_request: CancelMeetingRequest,
    api_endpoints: Vec<String>,
}

// Test endpoint that returns sample meeting requests
// Test endpoint to demonstrate meeting creation/cancellation APIs
async fn test_meetings() -> Json<TestMeetingResponse> {
    // Sample meeting creation request
    let sample_create = CreateMeetingRequest {
        userid: "test_user".to_string(),
        instanceid: 32,
        subject: "Test Meeting".to_string(),
        type_: 0, // Scheduled meeting
        _type: 0, // This will be auto-populated later
        hosts: Some(vec![
            client::User {
                userid: "test_host".to_string(),
                is_anonymous: None,
                nick_name: None,
            }
        ]),
        invitees: Some(vec![
            client::User {
                userid: "test_attendee1".to_string(),
                is_anonymous: None, 
                nick_name: None,
            },
            client::User {
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
        settings: Some(client::MeetingSettings {
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
struct TestFormSubmission {
    single_slot_example: FormSubmission,
    multiple_slots_example: FormSubmission,
    mergeable_slots_example: FormSubmission,
    webhook_endpoint: String,
}

// Test endpoint that returns a sample form submission
async fn test_form_submission() -> Json<TestFormSubmission> {
    // Example 1: Single time slot
    let mut extra_fields1 = std::collections::HashMap::new();
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
    let mut extra_fields2 = std::collections::HashMap::new();
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
    let mut extra_fields3 = std::collections::HashMap::new();
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

// List meeting rooms endpoint
async fn list_meeting_rooms(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<client::MeetingRoomsResponse>, StatusCode> {
    info!("Received request to list meeting rooms with page={}, page_size={}", 
          params.page, params.page_size);
    
    match state.client.list_rooms(params.page, params.page_size).await {
        Ok(response) => {
            info!("Successfully retrieved {} meeting rooms", response.meeting_room_list.len());
            Ok(Json(response))
        }
        Err(err) => {
            error!("Failed to retrieve meeting rooms: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Create meeting endpoint
async fn create_meeting(
    State(state): State<Arc<AppState>>,
    ExtractJson(meeting_request): ExtractJson<CreateMeetingRequest>,
) -> Result<Json<CreateMeetingResponse>, StatusCode> {
    info!("Received request to create meeting: {}", meeting_request.subject);
    
    match state.client.create_meeting(&meeting_request).await {
        Ok(response) => {
            info!("Successfully created {} meetings", response.meeting_number);
            Ok(Json(response))
        }
        Err(err) => {
            error!("Failed to create meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Cancel meeting endpoint
async fn cancel_meeting(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(cancel_request): ExtractJson<CancelMeetingRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to cancel meeting: {}", meeting_id);
    
    match state.client.cancel_meeting(&meeting_id, &cancel_request).await {
        Ok(_) => {
            info!("Successfully cancelled meeting {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(err) => {
            error!("Failed to cancel meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Structure to hold time slot information
#[derive(Debug, Clone)]
struct TimeSlot {
    item_name: String,
    label: String,
    start_time: i64,
    end_time: i64,
    #[allow(dead_code)]
    api_code: String,
}

// Parse a scheduled time from a form field item
fn parse_time_slot(reservation: &FormField1Item) -> Result<TimeSlot, String> {
    // Parse the scheduled time
    let scheduled_at_str = &reservation.scheduled_at;
    let meeting_start_time = match DateTime::parse_from_rfc3339(scheduled_at_str) {
        Ok(dt) => dt.with_timezone(&Utc).timestamp(),
        Err(e) => {
            return Err(format!("Failed to parse scheduled_at time: {}", e));
        }
    };
    
    // Parse the scheduled label to determine meeting duration
    // Format expected: "2025-03-30 09:00-10:00" or similar
    let scheduled_label = &reservation.scheduled_label;
    let parts: Vec<&str> = scheduled_label.split(' ').collect();
    let mut meeting_end_time = meeting_start_time + 3600; // Default 1 hour
    
    if parts.len() > 1 {
        let time_parts: Vec<&str> = parts[1].split('-').collect();
        if time_parts.len() > 1 {
            let start_time_str = time_parts[0];
            let end_time_str = time_parts[1];
            
            // Parse hour difference
            if let (Some(start_hour), Some(end_hour)) = (
                start_time_str.split(':').next().and_then(|h| h.parse::<i64>().ok()),
                end_time_str.split(':').next().and_then(|h| h.parse::<i64>().ok())
            ) {
                let hours_diff = if end_hour > start_hour { 
                    end_hour - start_hour 
                } else { 
                    24 + end_hour - start_hour // Handle overnight meetings
                };
                
                meeting_end_time = meeting_start_time + (hours_diff * 3600);
            }
        }
    }
    
    Ok(TimeSlot {
        item_name: reservation.item_name.clone(),
        label: reservation.scheduled_label.clone(),
        start_time: meeting_start_time,
        end_time: meeting_end_time,
        api_code: reservation.api_code.clone(),
    })
}

// This function is kept for reference in case it's needed later
#[allow(dead_code)]
fn can_merge_time_slots(slots: &[TimeSlot]) -> bool {
    if slots.len() <= 1 {
        return true; // Single slot or empty list is already "merged"
    }
    
    // All slots must be for the same room
    let first_room = &slots[0].item_name;
    if !slots.iter().all(|slot| &slot.item_name == first_room) {
        return false;
    }
    
    // Sort time slots by start_time
    let mut sorted_slots = slots.to_vec();
    sorted_slots.sort_by_key(|slot| slot.start_time);
    
    // Check for continuity (end time of one slot equals start time of the next)
    for i in 0..sorted_slots.len() - 1 {
        // If there's a gap or overlap, we can't merge
        if sorted_slots[i].end_time != sorted_slots[i+1].start_time {
            return false;
        }
    }
    
    true
}

// Attempt to find mergeable groups in time slots
fn find_mergeable_groups(slots: &[TimeSlot]) -> Vec<Vec<TimeSlot>> {
    if slots.is_empty() {
        return Vec::new();
    }
    
    // Group slots by room name
    let mut room_groups: std::collections::HashMap<String, Vec<TimeSlot>> = std::collections::HashMap::new();
    for slot in slots {
        room_groups.entry(slot.item_name.clone())
            .or_insert_with(Vec::new)
            .push(slot.clone());
    }
    
    let mut mergeable_groups = Vec::new();
    
    // Process each room's slots
    for (_, mut room_slots) in room_groups {
        // Sort by start time
        room_slots.sort_by_key(|slot| slot.start_time);
        
        // Find continuous groups
        let mut current_group = vec![room_slots[0].clone()];
        
        for i in 1..room_slots.len() {
            let last_slot = &current_group.last().unwrap();
            
            // If this slot starts exactly when the previous one ends, merge them
            if last_slot.end_time == room_slots[i].start_time {
                current_group.push(room_slots[i].clone());
            } else {
                // Otherwise start a new group
                if !current_group.is_empty() {
                    mergeable_groups.push(current_group);
                }
                current_group = vec![room_slots[i].clone()];
            }
        }
        
        // Add the last group if not empty
        if !current_group.is_empty() {
            mergeable_groups.push(current_group);
        }
    }
    
    mergeable_groups
}

// Create a meeting with the given time slot
async fn create_meeting_with_time_slot(
    state: &AppState,
    form_submission: &FormSubmission,
    time_slot: &TimeSlot,
) -> Result<MeetingResult, StatusCode> {
    // Create meeting request with the operator_id from the client
    let meeting_request = CreateMeetingRequest {
        userid: state.client.get_operator_id().to_string(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        hosts: Some(vec![
            User {
                userid: state.client.get_operator_id().to_string(),
                is_anonymous: None,
                nick_name: None,
            }
        ]),
        invitees: None,
        start_time: time_slot.start_time.to_string(),
        end_time: time_slot.end_time.to_string(),
        password: None,
        settings: Some(MeetingSettings {
            mute_enable_join: Some(true),
            mute_enable_type_join: Some(2),
            allow_unmute_self: Some(true),
            allow_in_before_host: Some(true),
            auto_in_waiting_room: None,
            allow_screen_shared_watermark: None,
            water_mark_type: None,
            only_enterprise_user_allowed: None,
            only_user_join_type: Some(1),
            auto_record_type: None,
            participant_join_auto_record: None,
            enable_host_pause_auto_record: None,
            allow_multi_device: Some(true),
            change_nickname: None,
            play_ivr_on_leave: None, 
            play_ivr_on_join: None,
        }),
        location: Some(format!("{} ({})", 
            time_slot.item_name,
            form_submission.entry.extra_fields.get(&state.dept_field_name)
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Department")
        )),
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
        time_zone: Some("Asia/Shanghai".to_string()),
        allow_enterprise_intranet_only: None,
        guests: None,
    };
    
    info!("Creating meeting for room: {} with time range: {}-{}", 
        time_slot.item_name,
        time_slot.start_time,
        time_slot.end_time
    );
    
    // Call the Tencent Meeting API to create the meeting
    match state.client.create_meeting(&meeting_request).await {
        Ok(response) => {
            if response.meeting_info_list.is_empty() {
                error!("Meeting created but no meeting info returned");
                Ok(MeetingResult {
                    meeting_id: None,
                    merged: false,
                    room_name: time_slot.item_name.clone(),
                    time_slots: vec![time_slot.label.clone()],
                    success: true,
                })
            } else {
                let meeting_info = &response.meeting_info_list[0];
                info!(
                    "Successfully created meeting: {} with ID: {}", 
                    meeting_info.subject, 
                    meeting_info.meeting_id
                );
                
                Ok(MeetingResult {
                    meeting_id: Some(meeting_info.meeting_id.clone()),
                    merged: false,
                    room_name: time_slot.item_name.clone(),
                    time_slots: vec![time_slot.label.clone()],
                    success: true,
                })
            }
        },
        Err(err) => {
            error!("Failed to create meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Create a merged meeting from multiple time slots
async fn create_merged_meeting(
    state: &AppState,
    form_submission: &FormSubmission,
    time_slots: &[TimeSlot],
) -> Result<MeetingResult, StatusCode> {
    if time_slots.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Sort time slots to ensure correct merging
    let mut sorted_slots = time_slots.to_vec();
    sorted_slots.sort_by_key(|slot| slot.start_time);
    
    // Use the earliest start time and latest end time to create a merged meeting
    let start_time = sorted_slots.first().unwrap().start_time;
    let end_time = sorted_slots.last().unwrap().end_time;
    let room_name = &sorted_slots[0].item_name;
    
    // Collect all time slot labels for reporting
    let time_slot_labels: Vec<String> = sorted_slots.iter()
        .map(|slot| slot.label.clone())
        .collect();
    
    // Log merged slot details
    info!("Creating merged time slot for room: {}, slots: {}, time range: {}-{}", 
        room_name, 
        time_slots.len(),
        start_time,
        end_time
    );
    
    // Create meeting request with merged time
    let meeting_request = CreateMeetingRequest {
        userid: state.client.get_operator_id().to_string(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        hosts: Some(vec![
            User {
                userid: state.client.get_operator_id().to_string(),
                is_anonymous: None,
                nick_name: None,
            }
        ]),
        invitees: None,
        start_time: start_time.to_string(),
        end_time: end_time.to_string(),
        password: None,
        settings: Some(MeetingSettings {
            mute_enable_join: Some(true),
            mute_enable_type_join: Some(2),
            allow_unmute_self: Some(true),
            allow_in_before_host: Some(true),
            auto_in_waiting_room: None,
            allow_screen_shared_watermark: None,
            water_mark_type: None,
            only_enterprise_user_allowed: None,
            only_user_join_type: Some(1),
            auto_record_type: None,
            participant_join_auto_record: None,
            enable_host_pause_auto_record: None,
            allow_multi_device: Some(true),
            change_nickname: None,
            play_ivr_on_leave: None, 
            play_ivr_on_join: None,
        }),
        location: Some(format!("{} ({})", 
            room_name,
            form_submission.entry.extra_fields.get(&state.dept_field_name)
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Department")
        )),
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
        time_zone: Some("Asia/Shanghai".to_string()),
        allow_enterprise_intranet_only: None,
        guests: None,
    };
    
    info!("Creating merged meeting for room: {} with time range: {}-{}", 
        room_name,
        start_time,
        end_time
    );
    
    // Call the Tencent Meeting API to create the meeting
    match state.client.create_meeting(&meeting_request).await {
        Ok(response) => {
            if response.meeting_info_list.is_empty() {
                error!("Merged meeting created but no meeting info returned");
                Ok(MeetingResult {
                    meeting_id: None,
                    merged: true,
                    room_name: room_name.clone(),
                    time_slots: time_slot_labels,
                    success: true,
                })
            } else {
                let meeting_info = &response.meeting_info_list[0];
                info!(
                    "Successfully created merged meeting: {} with ID: {}", 
                    meeting_info.subject, 
                    meeting_info.meeting_id
                );
                
                Ok(MeetingResult {
                    meeting_id: Some(meeting_info.meeting_id.clone()),
                    merged: true, 
                    room_name: room_name.clone(),
                    time_slots: time_slot_labels,
                    success: true,
                })
            }
        },
        Err(err) => {
            error!("Failed to create merged meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Form submission webhook handler
async fn handle_form_submission(
    State(state): State<Arc<AppState>>,
    ExtractJson(form_submission): ExtractJson<FormSubmission>,
) -> Result<Json<WebhookResponse>, StatusCode> {
    info!("Received form submission for form: {} ({})", form_submission.form_name, form_submission.form);
    
    // Check if we have at least one scheduled item
    if form_submission.entry.field_1.is_empty() {
        error!("Form submission has no scheduled items");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Parse all time slots
    let mut time_slots = Vec::new();
    for reservation in &form_submission.entry.field_1 {
        match parse_time_slot(reservation) {
            Ok(slot) => time_slots.push(slot),
            Err(e) => {
                error!("Failed to parse time slot from reservation: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    
    info!("Parsed {} time slots from form submission", time_slots.len());
    
    // Try to find mergeable groups
    let mergeable_groups = find_mergeable_groups(&time_slots);
    
    // Results storage
    let mut meeting_results = Vec::new();
    let mut all_successful = true;
    
    // If there's only one group and it includes all slots, we can fully merge
    if mergeable_groups.len() == 1 && mergeable_groups[0].len() == time_slots.len() {
        info!("All time slots can be merged into a single meeting");
        let result = create_merged_meeting(&state, &form_submission, &time_slots).await?;
        all_successful = all_successful && result.success;
        meeting_results.push(result);
    } else {
        // Process each mergeable group
        info!("Found {} mergeable groups", mergeable_groups.len());
        
        for (i, group) in mergeable_groups.iter().enumerate() {
            if group.len() > 1 {
                // Create a merged meeting for this group
                info!("Creating merged meeting for {} slots in group {}", group.len(), i+1);
                match create_merged_meeting(&state, &form_submission, group).await {
                    Ok(result) => {
                        all_successful = all_successful && result.success;
                        meeting_results.push(result);
                    },
                    Err(_) => {
                        all_successful = false;
                        // Continue processing other groups even if one fails
                    }
                }
            } else if group.len() == 1 {
                // Create a single meeting for this slot
                info!("Creating single meeting for time slot in group {}", i+1);
                match create_meeting_with_time_slot(&state, &form_submission, &group[0]).await {
                    Ok(result) => {
                        all_successful = all_successful && result.success;
                        meeting_results.push(result);
                    },
                    Err(_) => {
                        all_successful = false;
                        // Continue processing other groups even if one fails
                    }
                }
            }
        }
    }
    
    // Generate summary message
    let successful_count = meeting_results.iter()
        .filter(|r| r.meeting_id.is_some())
        .count();
    
    let merged_count = meeting_results.iter()
        .filter(|r| r.merged)
        .count();
    
    let message = if merged_count > 0 {
        format!(
            "Created {} meetings ({} merged) from {} time slots", 
            successful_count, 
            merged_count, 
            time_slots.len()
        )
    } else {
        format!(
            "Created {} meetings from {} time slots", 
            successful_count, 
            time_slots.len()
        )
    };
    
    // Return complete response with all meeting results
    Ok(Json(WebhookResponse {
        success: all_successful && successful_count > 0,
        message,
        meetings_count: meeting_results.len(),
        meetings: meeting_results,
    }))
}

// Error handler
async fn handle_error(error: BoxError) -> (StatusCode, String) {
    if error.is::<tokio::time::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", error),
        )
    }
}
