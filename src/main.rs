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
use std::net::SocketAddr;
use std::sync::Arc;
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
    CancelMeetingRequest
};
use std::time::Duration;
use chrono;

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

// Define application state
struct AppState {
    client: TencentMeetingClient,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Initialize the Tencent Meeting API client
    let client = TencentMeetingClient::new();
    
    // Create shared application state
    let app_state = Arc::new(AppState { client });

    // Create router with routes
    let app = Router::new()
        .route("/meeting-rooms", get(list_meeting_rooms))
        .route("/meetings", post(create_meeting))
        .route("/meetings/:meeting_id/cancel", post(cancel_meeting))
        .route("/health", get(health_check))
        .route("/test", get(test_endpoint))
        .route("/test-meetings", get(test_meetings))
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
        instanceid: 1,
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
    ];
    
    Json(TestMeetingResponse {
        sample_create_request: sample_create,
        sample_cancel_request: sample_cancel,
        api_endpoints: endpoints,
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
