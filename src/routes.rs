use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::handlers::api::{
    book_rooms, cancel_meeting, create_meeting, handle_form_submission, list_meeting_rooms,
    release_rooms, AppState,
};
use crate::handlers::test::{health_check};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    // API routes
    let api_routes = Router::new()
        .route("/meeting-rooms", get(list_meeting_rooms))
        .route("/meetings", post(create_meeting))
        .route("/meetings/:meeting_id/cancel", post(cancel_meeting))
        .route("/meetings/:meeting_id/book-rooms", post(book_rooms))
        .route("/meetings/:meeting_id/release-rooms", post(release_rooms))
        .route("/webhook/form-submission", post(handle_form_submission));

    // Health check
    let health_route = Router::new()
        .route("/health", get(health_check));

    // Combine routes
    Router::new()
        .merge(api_routes)
        .merge(health_route)
        .with_state(app_state)
}
