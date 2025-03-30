use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{error_handling::HandleErrorLayer, http::StatusCode};
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};

use tencent_meeting_service::{
    create_router, services::database::create_database_service, AppState, TencentMeetingClient,
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
        .with_max_level(Level::DEBUG)
        .init();

    // Initialize the Tencent Meeting API client
    let client = TencentMeetingClient::new();

    // Load custom field names from environment
    let user_field_name =
        env::var("FORM_USER_FIELD_NAME").expect("FORM_USER_FIELD_NAME must be set in environment");

    let dept_field_name =
        env::var("FORM_DEPT_FIELD_NAME").expect("FORM_DEPT_FIELD_NAME must be set in environment");

    // Load city-specific room IDs
    let xa_room_id =
        env::var("XA_MEETING_ROOM_ID").expect("XA_MEETING_ROOM_ID must be set in environment");

    let cd_room_id =
        env::var("CD_MEETING_ROOM_ID").expect("CD_MEETING_ROOM_ID must be set in environment");

    info!("Using form field mappings and city-specific room IDs from environment variables");

    // Initialize the database service
    let database = create_database_service();
    info!("Database service initialized");

    // Load toggle settings from environment or default to false
    let skip_meeting_creation = env::var("SKIP_MEETING_CREATION")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(false);

    let skip_room_booking = env::var("SKIP_ROOM_BOOKING")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(false);

    if skip_meeting_creation {
        info!("Running in simulation mode: Form submissions will be stored in CSV only, no meetings will be created");
    } else if skip_room_booking {
        info!("Room booking disabled: Meetings will be created but no rooms will be booked");
    }

    // Load webhook auth token from environment if provided
    let webhook_auth_token = env::var("WEBHOOK_AUTH_TOKEN").ok();

    if webhook_auth_token.is_some() {
        info!("Webhook authentication enabled with provided token");
    } else {
        info!("No webhook authentication token provided - authentication disabled");
    }
    
    // Check if running in production mode
    let is_production = env::var("ENVIRONMENT")
        .map(|val| val.to_lowercase() == "production")
        .unwrap_or(false);
        
    if is_production {
        info!("Running in PRODUCTION mode - restricting available endpoints");
    } else {
        info!("Running in DEVELOPMENT mode - all endpoints will be available");
    }

    // Create shared application state
    let app_state = Arc::new(AppState {
        client,
        user_field_name,
        dept_field_name,
        database,
        xa_room_id,
        cd_room_id,
        skip_meeting_creation,
        skip_room_booking,
        webhook_auth_token,
    });

    // Create router with appropriate routes based on environment
    let app = create_router(app_state, is_production).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_error))
            .load_shed()
            .concurrency_limit(64)
            .timeout(Duration::from_secs(10))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::new().allow_origin(Any)),
    );

    // Bind to port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Server listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    // Set up signal handler for graceful shutdown
    let shutdown = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received interrupt signal, starting graceful shutdown");
            },
            _ = terminate => {
                info!("Received terminate signal, starting graceful shutdown");
            },
        }
    };

    // Start server with graceful shutdown
    info!("Server is ready to accept connections");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("Failed to start server");

    info!("Server has been gracefully shut down");
}
