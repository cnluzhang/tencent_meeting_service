#[cfg(test)]
mod api_tests {
    use std::sync::Arc;
    use axum::{
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        response::Response,
        Router,
    };
    use axum_test::{TestServer, TestServerConfig};
    use tempfile::tempdir;
    use tower::ServiceExt;
    use serde_json::json;
    
    use crate::client_mock::{MockTencentMeetingClient, setup_mock_client};
    use crate::handlers::api::{AppState, webhook_form_submission};
    use crate::models::form::FormSubmission;
    use crate::services::database::DatabaseService;
    use crate::routes::create_router;

    // Helper function to set up a test server with mock dependencies
    async fn setup_test_server() -> (TestServer, Arc<MockTencentMeetingClient>, Arc<DatabaseService>) {
        // Create a temporary database
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        let db_service = Arc::new(DatabaseService::new(csv_path_str));
        
        // Set up mock client
        let (mock_client, _) = setup_mock_client();
        let mock_client = Arc::new(mock_client);
        
        // Create app state
        let app_state = AppState {
            client: Arc::clone(&mock_client),
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: false,
            skip_room_booking: false,
        };
        
        // Create the router
        let router = create_router(app_state.clone());
        
        // Set up the test server
        let config = TestServerConfig::builder()
            .mock_transport()
            .build();
        let server = TestServer::new_with_config(router, config).unwrap();
        
        (server, mock_client, db_service)
    }

    #[tokio::test]
    async fn test_webhook_form_submission() {
        // Set up test app state with mock client and database
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        let db_service = Arc::new(DatabaseService::new(csv_path_str));
        
        // Set up mock client
        let (mock_client, _) = setup_mock_client();
        let mock_client = Arc::new(mock_client);
        
        // Create app state
        let app_state = AppState {
            client: Arc::clone(&mock_client),
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: false,
            skip_room_booking: false,
        };
        
        // Create a form submission payload
        let payload = json!({
            "form": "test_form",
            "form_name": "Test Form",
            "entry": {
                "token": "test_token_123",
                "field_1": [
                    {
                        "item_name": "Conference Room A",
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 1,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE1"
                    }
                ],
                "field_8": "Test Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已预约"
            }
        });
        
        let form_submission: FormSubmission = serde_json::from_value(payload).unwrap();
        
        // Call the handler directly
        let result = webhook_form_submission(
            State(app_state),
            axum::Json(form_submission),
        ).await;
        
        // Check the response
        assert_eq!(result.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_webhook_form_cancellation() {
        // Set up test app state with mock client and database
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        let db_service = Arc::new(DatabaseService::new(csv_path_str));
        
        // Set up mock client
        let (mock_client, _) = setup_mock_client();
        let mock_client = Arc::new(mock_client);
        
        // Create app state
        let app_state = AppState {
            client: Arc::clone(&mock_client),
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: false,
            skip_room_booking: false,
        };
        
        // First create a meeting
        let reservation_payload = json!({
            "form": "test_form",
            "form_name": "Test Form",
            "entry": {
                "token": "test_token_123",
                "field_1": [
                    {
                        "item_name": "Conference Room A",
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 1,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE1"
                    }
                ],
                "field_8": "Test Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已预约"
            }
        });
        
        let form_submission: FormSubmission = serde_json::from_value(reservation_payload).unwrap();
        
        // Call the reservation handler
        let _ = webhook_form_submission(
            State(app_state.clone()),
            axum::Json(form_submission),
        ).await;
        
        // Now create a cancellation for the same token
        let cancellation_payload = json!({
            "form": "test_form",
            "form_name": "Test Form",
            "entry": {
                "token": "test_token_123",
                "field_1": [
                    {
                        "item_name": "Conference Room A",
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 1,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE1"
                    }
                ],
                "field_8": "Test Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已取消"
            }
        });
        
        let cancellation_submission: FormSubmission = serde_json::from_value(cancellation_payload).unwrap();
        
        // Call the handler again with cancellation
        let result = webhook_form_submission(
            State(app_state),
            axum::Json(cancellation_submission),
        ).await;
        
        // Check the response for cancellation
        assert_eq!(result.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_health_endpoint() {
        let (server, _, _) = setup_test_server().await;
        
        // Call the health endpoint
        let response = server.get("/health").await;
        
        // Check the response
        assert_eq!(response.status_code(), StatusCode::OK);
        let body = response.text();
        assert_eq!(body, "Service is healthy");
    }
    
    #[tokio::test]
    async fn test_meeting_rooms_endpoint() {
        let (server, _, _) = setup_test_server().await;
        
        // Call the meeting rooms endpoint
        let response = server.get("/meeting-rooms?page=1&page_size=10").await;
        
        // Check the response
        assert_eq!(response.status_code(), StatusCode::OK);
        
        // Verify the response contains room data
        let body: serde_json::Value = response.json();
        assert!(body.as_object().unwrap().contains_key("meeting_room_list"));
    }
    
    #[tokio::test]
    async fn test_multiple_time_slots() {
        // Set up test app state
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        let db_service = Arc::new(DatabaseService::new(csv_path_str));
        
        // Set up mock client
        let (mock_client, _) = setup_mock_client();
        let mock_client = Arc::new(mock_client);
        
        // Create app state
        let app_state = AppState {
            client: Arc::clone(&mock_client),
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: false,
            skip_room_booking: false,
        };
        
        // Create payload with multiple time slots - some mergeable, some not
        let payload = json!({
            "form": "test_form",
            "form_name": "Test Form",
            "entry": {
                "token": "multi_slot_token",
                "field_1": [
                    // Room A 9:00-10:00
                    {
                        "item_name": "Room A",
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 1,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE1"
                    },
                    // Room A 10:00-11:00 (mergeable with first)
                    {
                        "item_name": "Room A",
                        "scheduled_label": "2025-03-30 10:00-11:00",
                        "number": 2,
                        "scheduled_at": "2025-03-30T02:00:00.000Z",
                        "api_code": "CODE2"
                    },
                    // Room B 13:00-14:00 (different room)
                    {
                        "item_name": "Room B",
                        "scheduled_label": "2025-03-30 13:00-14:00",
                        "number": 3,
                        "scheduled_at": "2025-03-30T05:00:00.000Z",
                        "api_code": "CODE3"
                    }
                ],
                "field_8": "Multi-Slot Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已预约"
            }
        });
        
        let form_submission: FormSubmission = serde_json::from_value(payload).unwrap();
        
        // Call the handler
        let result = webhook_form_submission(
            State(app_state.clone()),
            axum::Json(form_submission),
        ).await;
        
        // Check the response
        assert_eq!(result.status(), StatusCode::OK);
        
        // Get the response body and parse it
        let bytes = hyper::body::to_bytes(result.into_body()).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        
        // Check the results
        assert_eq!(body["success"], json!(true));
        
        // Should have created 2 meetings: 1 merged for Room A and 1 for Room B
        assert_eq!(body["meetings_count"], json!(2));
        
        // Verify the database records
        let all_records = db_service.find_all_meetings_by_token("multi_slot_token").unwrap();
        assert_eq!(all_records.len(), 2);
        
        // Check for the merged meeting
        let merged_record = all_records.iter().find(|r| r.scheduled_label == "2025-03-30 09:00-11:00");
        assert!(merged_record.is_some());
        
        // Check for the single slot meeting
        let single_record = all_records.iter().find(|r| r.scheduled_label == "2025-03-30 13:00-14:00");
        assert!(single_record.is_some());
    }
}