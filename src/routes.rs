use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::info;

use crate::handlers::api::{
    book_rooms, cancel_meeting, create_meeting, handle_form_submission, list_meeting_rooms,
    release_rooms, AppState,
};
use crate::handlers::test::health_check;

pub fn create_router(app_state: Arc<AppState>, is_production: bool) -> Router {
    let mut router = Router::new();
    
    // Health check is always available
    let health_route = Router::new().route("/health", get(health_check));
    router = router.merge(health_route);
    
    // Webhook endpoint is always available
    let webhook_route = Router::new().route("/webhook/form-submission", post(handle_form_submission));
    router = router.merge(webhook_route);
    
    // Only add management API routes if not in production mode
    if !is_production {
        // Management API routes
        let api_routes = Router::new()
            .route("/meeting-rooms", get(list_meeting_rooms))
            .route("/meetings", post(create_meeting))
            .route("/meetings/:meeting_id/cancel", post(cancel_meeting))
            .route("/meetings/:meeting_id/book-rooms", post(book_rooms))
            .route("/meetings/:meeting_id/release-rooms", post(release_rooms));
        
        router = router.merge(api_routes);
        
        info!("Management API routes enabled - server running in development mode");
    } else {
        info!("Running in production mode - only webhook and health endpoints exposed");
    }

    router.with_state(app_state)
}
