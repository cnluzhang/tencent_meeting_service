mod client;

use axum::{
    routing::get,
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    error_handling::HandleErrorLayer,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    trace::{TraceLayer},
    cors::{CorsLayer, Any},
};
use tracing::{info, error, Level};
use client::TencentMeetingClient;
use std::time::Duration;

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
        .route("/health", get(health_check))
        .route("/test", get(test_endpoint))
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
