#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;
    use axum_test::{TestServer, TestServerConfig};
    use tempfile::tempdir;
    use serde_json::{json, Value};
    
    use crate::client::TencentMeetingClient;
    use crate::client_mock::{setup_mock_client};
    use crate::handlers::api::AppState;
    use crate::services::database::DatabaseService;
    use crate::routes::create_router;

    // Helper function to set up a test environment with controlled dependencies
    async fn setup_test_environment() -> (TestServer, Arc<DatabaseService>, String) {
        // Create a temporary database file
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap().to_string();
        
        // Initialize database service
        let db_service = Arc::new(DatabaseService::new(&csv_path_str));
        
        // We're using real client directly
        // let (mock_client, _) = setup_mock_client();
        
        // Create a real Tencent client
        let client = TencentMeetingClient::default();
        
        // Set up app state
        let app_state = Arc::new(AppState {
            client,
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: false, // For integration tests, we'll simulate API calls
            skip_room_booking: false,
        });
        
        // Create router
        let app = create_router(app_state);
        
        // Create test server
        let config = TestServerConfig::builder()
            .mock_transport()
            .build();
        
        let server = TestServer::new_with_config(app, config).unwrap();
        
        (server, db_service, csv_path_str)
    }
    
    // Helper to create a test form submission
    fn create_test_form_submission(token: &str, status: &str) -> Value {
        json!({
            "form": "test_form",
            "form_name": "Meeting Room Reservation",
            "entry": {
                "token": token,
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
                "reservation_status_fsf_field": status
            }
        })
    }

    // Test for health endpoint
    #[tokio::test]
    async fn test_health_endpoint() {
        let (server, _, _) = setup_test_environment().await;
        
        let response = server.get("/health").await;
        assert_eq!(response.status_code().as_u16(), 200);
    }
    
    // Test a complete reservation workflow
    #[tokio::test]
    async fn test_complete_reservation_workflow() {
        // Setup test environment
        let (server, db_service, _) = setup_test_environment().await;
        
        // 1. Create a reservation
        let token = "workflow_token_123";
        let payload = create_test_form_submission(token, "已预约");
        
        // Send the reservation request
        let response = server.post("/webhook/form-submission")
            .json(&payload)
            .await;
        
        // Check that the request was successful
        assert_eq!(response.status_code().as_u16(), 200);
        let body: Value = response.json();
        assert_eq!(body["success"], json!(true));
        
        // Check that meeting was stored in database
        let meetings = db_service.find_all_meetings_by_token(token).unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].status, "已预约");
        
        // 2. Cancel the reservation
        let cancel_payload = create_test_form_submission(token, "已取消");
        
        // Send the cancellation request
        let cancel_response = server.post("/webhook/form-submission")
            .json(&cancel_payload)
            .await;
        
        // Check that the request was successful
        assert_eq!(cancel_response.status_code().as_u16(), 200);
        let cancel_body: Value = cancel_response.json();
        assert_eq!(cancel_body["success"], json!(true));
        
        // Check that meeting was marked as cancelled in database
        let meetings_after = db_service.find_all_meetings_by_token(token).unwrap();
        assert_eq!(meetings_after.len(), 1);
        assert_eq!(meetings_after[0].status, "已取消");
    }
    
    // Test multi-slot reservation with merging
    #[tokio::test]
    async fn test_multi_slot_reservation_with_merging() {
        // Setup test environment
        let (server, db_service, _) = setup_test_environment().await;
        
        // Create payload with consecutive time slots that should be merged
        let token = "multi_slot_token";
        let payload = json!({
            "form": "test_form",
            "form_name": "Meeting Room Reservation",
            "entry": {
                "token": token,
                "field_1": [
                    {
                        "item_name": "Conference Room A",
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 1,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE1"
                    },
                    {
                        "item_name": "Conference Room A", // Same room
                        "scheduled_label": "2025-03-30 10:00-11:00", // Consecutive time
                        "number": 2,
                        "scheduled_at": "2025-03-30T02:00:00.000Z",
                        "api_code": "CODE2"
                    },
                    {
                        "item_name": "Conference Room B", // Different room
                        "scheduled_label": "2025-03-30 09:00-10:00",
                        "number": 3,
                        "scheduled_at": "2025-03-30T01:00:00.000Z",
                        "api_code": "CODE3"
                    }
                ],
                "field_8": "Multi Slot Meeting",
                "user_field_name": "Test User",
                "department_field_name": "Test Department",
                "reservation_status_fsf_field": "已预约"
            }
        });
        
        // Send the reservation request
        let response = server.post("/webhook/form-submission")
            .json(&payload)
            .await;
        
        // Check that the request was successful
        assert_eq!(response.status_code().as_u16(), 200);
        let body: Value = response.json();
        assert_eq!(body["success"], json!(true));
        
        // Should have created 2 meetings (one merged for Room A, one for Room B)
        assert_eq!(body["meetings_count"], json!(2));
        
        // Check the database
        let meetings = db_service.find_all_meetings_by_token(token).unwrap();
        assert_eq!(meetings.len(), 2);
        
        // Check for the merged meeting
        let has_merged_meeting = meetings.iter().any(|m| 
            m.room_name == "Conference Room A" && m.scheduled_label == "2025-03-30 09:00-11:00"
        );
        assert!(has_merged_meeting, "Should have a merged meeting for Room A");
        
        // Check for the single meeting
        let has_single_meeting = meetings.iter().any(|m| 
            m.room_name == "Conference Room B" && m.scheduled_label == "2025-03-30 09:00-10:00"
        );
        assert!(has_single_meeting, "Should have a single meeting for Room B");
    }
    
    // Test simulation mode
    #[tokio::test]
    async fn test_simulation_mode() {
        // Create a temporary database file
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap().to_string();
        
        // Initialize database service
        let db_service = Arc::new(DatabaseService::new(&csv_path_str));
        
        // Create a real client
        let client = TencentMeetingClient::default();
        
        // Set up app state with simulation mode enabled
        let app_state = Arc::new(AppState {
            client: client,
            database: Arc::clone(&db_service),
            user_field_name: "user_field_name".to_string(),
            dept_field_name: "department_field_name".to_string(),
            default_room_id: "room1".to_string(),
            skip_meeting_creation: true, // SIMULATION MODE
            skip_room_booking: true,     // SIMULATION MODE
        });
        
        // Create router
        let app = create_router(app_state);
        
        // Create test server
        let config = TestServerConfig::builder()
            .mock_transport()
            .build();
        
        let server = TestServer::new_with_config(app, config).unwrap();
        
        // Create reservation request
        let token = "simulation_token";
        let payload = create_test_form_submission(token, "已预约");
        
        // Send the reservation request
        let response = server.post("/webhook/form-submission")
            .json(&payload)
            .await;
        
        // Check that the request was successful
        assert_eq!(response.status_code().as_u16(), 200);
        let body: Value = response.json();
        assert_eq!(body["success"], json!(true));
        assert!(body["simulation_mode"].as_bool().unwrap());
        
        // Meeting should still be stored in database even in simulation mode
        let meetings = db_service.find_all_meetings_by_token(token).unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].status, "已预约");
        
        // Meeting ID should be "SIMULATION" in simulation mode
        assert_eq!(meetings[0].meeting_id, "SIMULATION");
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
        let response = server.post("/webhook/form-submission")
            .json(&payload)
            .await;
        
        // Should return an error status
        assert_eq!(response.status_code().as_u16(), 400);
        let body: Value = response.json();
        assert_eq!(body["success"], json!(false));
        assert!(body["error"].as_str().unwrap().contains("invalid form submission"));
    }
    
    // Test parallel processing of multiple requests
    #[tokio::test]
    async fn test_concurrent_requests() {
        let (server, db_service, _) = setup_test_environment().await;
        
        // Create 5 different form submissions
        let tokens = vec![
            "concurrent_token_1",
            "concurrent_token_2",
            "concurrent_token_3",
            "concurrent_token_4",
            "concurrent_token_5"
        ];
        
        // Process requests sequentially since TestServer doesn't support clone
        for token in &tokens {
            let token_str = token.to_string();
            let payload = create_test_form_submission(&token_str, "已预约");
            
            // Send the request
            let response = server.post("/webhook/form-submission")
                .json(&payload)
                .await;
            
            // Check the result
            assert_eq!(response.status_code().as_u16(), 200);
            let body: Value = response.json();
            assert_eq!(body["success"], json!(true));
        }
        
        // Verify all entries were stored correctly
        for token in tokens {
            let meetings = db_service.find_all_meetings_by_token(token).unwrap();
            assert_eq!(meetings.len(), 1, "Meeting for token {} not found", token);
            assert_eq!(meetings[0].status, "已预约");
        }
    }
    
    // Test listing meeting rooms
    #[tokio::test]
    async fn test_list_meeting_rooms() {
        let (server, _, _) = setup_test_environment().await;
        
        // Call the meeting rooms endpoint
        let response = server.get("/meeting-rooms?page=1&page_size=10").await;
        
        // Check the response
        assert_eq!(response.status_code().as_u16(), 200);
        
        let body: Value = response.json();
        let meeting_rooms = body["meeting_room_list"].as_array().unwrap();
        
        // Should have at least one room
        assert!(!meeting_rooms.is_empty());
        
        // Check the first room has required fields
        let first_room = &meeting_rooms[0];
        assert!(first_room["meeting_room_id"].is_string());
        assert!(first_room["meeting_room_name"].is_string());
    }
}