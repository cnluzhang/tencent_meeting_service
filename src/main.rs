use std::net::SocketAddr;
use std::sync::Arc;
use std::env;
use std::time::Duration;

use tower::{BoxError, ServiceBuilder};
use tower_http::{
    trace::{TraceLayer},
    cors::{CorsLayer, Any},
};
use tracing::{info, Level};
use axum::{
    http::StatusCode,
    error_handling::HandleErrorLayer,
};

use tencent_meeting_service::{
    TencentMeetingClient, 
    AppState,
    create_router,
};

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

    // Create router with all routes
    let app = create_router(app_state)
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