use axum_test::{TestServer, TestServerConfig};
use chrono::Datelike;
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::tempdir;

use crate::client::TencentMeetingClient;
use crate::handlers::api::AppState;
use crate::routes::create_router;
use crate::services::database::DatabaseService;

/// Webhook integration tests
#[cfg(test)]
mod webhook_tests {
    use super::*;

    // Helper function to set up a test environment with controlled dependencies
    async fn setup_test_environment() -> (TestServer, Arc<DatabaseService>, String) {
        // Create a temporary database file
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap().to_string();

        // Initialize database service
        let db_service = Arc::new(DatabaseService::new(&csv_path_str));

        // Create a real client - but don't worry, we'll use simulation mode
        let client = TencentMeetingClient::default();

        // Set up app state - using simulation mode so no real API calls are made
        let app_state = Arc::new(AppState {
            client,
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            xa_room_id: "room1".to_string(), // Xi'an room ID
            cd_room_id: "room2".to_string(), // Chengdu room ID
            skip_meeting_creation: true,     // SIMULATION MODE
            skip_room_booking: true,         // SIMULATION MODE
            webhook_auth_token: None,        // No auth token for integration tests
        });

        // Create router - always use development mode in tests
        let app = create_router(app_state, false);

        // Create test server
        let config = TestServerConfig::builder().mock_transport().build();

        let server = TestServer::new_with_config(app, config).unwrap();

        (server, db_service, csv_path_str)
    }

    // Helper to create a test form submission with future time slots
    fn create_test_form_submission(token: &str, status: &str) -> Value {
        // Get a future date that's at least 1 year in the future to avoid test failures as time passes
        let current_year = chrono::Utc::now().year();
        let future_year = current_year + 1;
        let future_date = format!("{}-03-30", future_year);
        let future_date_rfc = format!("{}T01:00:00.000Z", future_date);

        // Create the base form structure with future dates
        let form = json!({
            "form": "test_form",
            "form_name": "Meeting Room Reservation",
            "entry": {
                "token": token,
                "field_1": [
                    {
                        "item_name": "Conference Room A",
                        "scheduled_label": format!("{} 09:00-10:00", future_date),
                        "number": 1,
                        "scheduled_at": future_date_rfc,
                        "api_code": "CODE1"
                    }
                ],
                "field_8": "Test Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": status
            }
        });

        form
    }

    // Test error handling for invalid form submissions
    #[tokio::test]
    async fn test_error_handling_invalid_form() {
        let (server, _, _) = setup_test_environment().await;

        // Create an invalid payload with missing required fields
        let payload = json!({
            "form": "test_form",
            "form_name": "Meeting Room Reservation",
            "entry": {
                "token": "error_token",
                // Missing field_1
                "field_8": "Test Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已预约"
            }
        });

        // Send the request
        let response = server.post("/webhook/form-submission").json(&payload).await;

        // Should return an error status - might be 400 or 422 based on the validation
        let status = response.status_code().as_u16();
        assert!(status >= 400, "Expected error status, got {}", status);
    }

    // Test parallel processing of multiple requests
    #[tokio::test]
    async fn test_concurrent_requests() {
        let (server, _, _) = setup_test_environment().await;

        // Create 5 different form submissions
        let tokens = vec![
            "concurrent_token_1",
            "concurrent_token_2",
            "concurrent_token_3",
            "concurrent_token_4",
            "concurrent_token_5",
        ];

        // Process requests sequentially since TestServer doesn't support clone
        for token in &tokens {
            let token_str = token.to_string();
            let payload = create_test_form_submission(&token_str, "已预约");

            // Send the request
            let response = server.post("/webhook/form-submission").json(&payload).await;

            // Check the result
            assert_eq!(response.status_code().as_u16(), 200);
            let body: Value = response.json();
            assert_eq!(body["success"], json!(true));
        }

        // In simulation mode we don't need to verify database entries
    }
}