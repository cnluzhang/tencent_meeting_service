use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::handlers::api::{
    list_meeting_rooms, create_meeting, cancel_meeting, handle_form_submission, AppState
};
use crate::handlers::test::{
    health_check, test_endpoint, test_meetings, test_form_submission
};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    // API routes
    let api_routes = Router::new()
        .route("/meeting-rooms", get(list_meeting_rooms))
        .route("/meetings", post(create_meeting))
        .route("/meetings/:meeting_id/cancel", post(cancel_meeting))
        .route("/webhook/form-submission", post(handle_form_submission));

    // Test routes
    let test_routes = Router::new()
        .route("/health", get(health_check))
        .route("/test", get(test_endpoint))
        .route("/test-meetings", get(test_meetings))
        .route("/test-form-submission", get(test_form_submission));

    // Combine routes
    Router::new()
        .merge(api_routes)
        .merge(test_routes)
        .with_state(app_state)
}