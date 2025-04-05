// This file is kept for future use but currently commented out
// to avoid unused import warnings

// This file contains utility functions for testing the API
// These are currently not used but will be useful for future test expansion

// Keeping commented to fix warnings while preserving the code for future use
/*
/// Create a test app with the specified client and database
pub async fn create_test_app(
    client: TencentMeetingClient,
    database: Arc<DatabaseService>,
) -> axum::Router {
    // Create app state
    let app_state = Arc::new(AppState {
        client,
        database: Arc::clone(&database),
        user_field_name: "user_field_name".to_string(),
        dept_field_name: "department_field_name".to_string(),
        xa_room_id: "room1".to_string(),  // Xi'an room ID
        cd_room_id: "room2".to_string(),  // Chengdu room ID
        skip_meeting_creation: false,
        skip_room_booking: false,
        webhook_auth_token: None,         // No auth required for tests
    });
    
    // Create the router
    create_router(app_state, false)
}

/// Create a test request with the specified method, uri, and body
pub fn create_test_request(
    method: &str,
    uri: &str,
    body: String,
    headers: &[(&str, &str)],
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json");

    for (name, value) in headers {
        request = request.header(
            HeaderName::from_str(name).expect("Invalid header name"),
            HeaderValue::from_str(value).expect("Invalid header value"),
        );
    }

    request
        .body(Body::from(body))
        .expect("Failed to create request")
}

/// Create a test webhook request with a form submission
pub fn create_form_webhook_request(
    form_submission: &FormSubmission,
    auth_token: Option<&str>,
) -> Request<Body> {
    let uri = match auth_token {
        Some(token) => format!("/webhook/form-submission?auth={}", token),
        None => "/webhook/form-submission".to_string(),
    };

    let body = serde_json::to_string(form_submission).expect("Failed to serialize form submission");
    
    create_test_request("POST", &uri, body, &[])
}

/// Execute a request against the test app and return the response
pub async fn execute_test_request(
    app: &axum::Router,
    request: Request<Body>,
) -> axum::http::Response<Body> {
    app.clone().oneshot(request).await.unwrap()
}
*/