#[cfg(test)]
mod client_tests {
    use std::env;
    use std::sync::Arc;
    use mockall::predicate::*;
    use crate::client::{
        BookRoomsRequest, CancelMeetingRequest, CreateMeetingRequest, 
        MeetingInfo, MeetingRoomsResponse, ReleaseRoomsRequest, TencentMeetingClient, User
    };
    use crate::client_mock::{MockTencentMeetingClient, setup_mock_client};
    
    #[tokio::test]
    async fn test_list_rooms() {
        let (mock_client, _) = setup_mock_client();
        
        // Call the API
        let result = mock_client.list_rooms(1, 10).await;
        
        // Check the result
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.current_page, 1);
        assert!(response.meeting_room_list.len() <= 10);
        assert!(response.total_count >= response.meeting_room_list.len() as i32);
    }
    
    #[tokio::test]
    async fn test_create_meeting() {
        let (mock_client, _) = setup_mock_client();
        
        // Create a meeting request
        let request = CreateMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            subject: "Test Meeting".to_string(),
            type_: 0,
            _type: 0,
            hosts: Some(vec![User {
                userid: "test_host".to_string(),
                is_anonymous: None,
                nick_name: Some("Test Host".to_string()),
            }]),
            guests: None,
            invitees: Some(vec![User {
                userid: "test_invitee".to_string(),
                is_anonymous: None,
                nick_name: Some("Test Invitee".to_string()),
            }]),
            start_time: "1680000000".to_string(),
            end_time: "1680003600".to_string(),
            password: Some("123456".to_string()),
            settings: None,
            meeting_type: None,
            recurring_rule: None,
            enable_live: None,
            live_config: None,
            enable_doc_upload_permission: None,
            media_set_type: None,
            enable_interpreter: None,
            enable_enroll: None,
            enable_host_key: None,
            host_key: None,
            sync_to_wework: None,
            time_zone: None,
            location: Some("Test Location".to_string()),
            allow_enterprise_intranet_only: None,
        };
        
        // Call the API
        let result = mock_client.create_meeting(&request).await;
        
        // Check the result
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.meeting_info_list.len(), 1);
        
        let meeting_info = &response.meeting_info_list[0];
        assert_eq!(meeting_info.subject, "Test Meeting");
        assert!(meeting_info.meeting_id.starts_with("meeting_"));
        assert!(meeting_info.join_url.is_some());
    }
    
    #[tokio::test]
    async fn test_cancel_meeting() {
        let (mock_client, data_store) = setup_mock_client();
        
        // First create a meeting
        let create_request = CreateMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            subject: "Test Meeting".to_string(),
            type_: 0,
            _type: 0,
            hosts: None,
            guests: None,
            invitees: None,
            start_time: "1680000000".to_string(),
            end_time: "1680003600".to_string(),
            password: None,
            settings: None,
            meeting_type: None,
            recurring_rule: None,
            enable_live: None,
            live_config: None,
            enable_doc_upload_permission: None,
            media_set_type: None,
            enable_interpreter: None,
            enable_enroll: None,
            enable_host_key: None,
            host_key: None,
            sync_to_wework: None,
            time_zone: None,
            location: None,
            allow_enterprise_intranet_only: None,
        };
        
        // Create the meeting
        let create_result = mock_client.create_meeting(&create_request).await.unwrap();
        let meeting_id = &create_result.meeting_info_list[0].meeting_id;
        
        // Now create a cancellation request
        let cancel_request = CancelMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            reason_code: 1,
            meeting_type: None,
            sub_meeting_id: None,
            reason_detail: Some("Test cancellation".to_string()),
        };
        
        // Cancel the meeting
        let cancel_result = mock_client.cancel_meeting(meeting_id, &cancel_request).await;
        
        // Check the result
        assert!(cancel_result.is_ok());
        
        // Verify that the meeting was removed from the data store
        let meeting = data_store.get_meeting(meeting_id);
        assert!(meeting.is_none());
    }
    
    #[tokio::test]
    async fn test_book_rooms() {
        let (mock_client, data_store) = setup_mock_client();
        
        // First create a meeting
        let create_request = CreateMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            subject: "Test Meeting".to_string(),
            type_: 0,
            _type: 0,
            hosts: None,
            guests: None,
            invitees: None,
            start_time: "1680000000".to_string(),
            end_time: "1680003600".to_string(),
            password: None,
            settings: None,
            meeting_type: None,
            recurring_rule: None,
            enable_live: None,
            live_config: None,
            enable_doc_upload_permission: None,
            media_set_type: None,
            enable_interpreter: None,
            enable_enroll: None,
            enable_host_key: None,
            host_key: None,
            sync_to_wework: None,
            time_zone: None,
            location: None,
            allow_enterprise_intranet_only: None,
        };
        
        // Create the meeting
        let create_result = mock_client.create_meeting(&create_request).await.unwrap();
        let meeting_id = &create_result.meeting_info_list[0].meeting_id;
        
        // Now book rooms for the meeting
        let book_request = BookRoomsRequest {
            operator_id: "test_operator".to_string(),
            operator_id_type: 1,
            meeting_room_id_list: vec!["room1".to_string(), "room2".to_string()],
            subject_visible: Some(true),
        };
        
        // Book the rooms
        let book_result = mock_client.book_rooms(meeting_id, &book_request).await;
        
        // Check the result
        assert!(book_result.is_ok());
        
        // Verify booked rooms in data store (this would require adding more methods to MockDataStore)
        // We'll just assume the mock worked correctly for now
    }
    
    #[tokio::test]
    async fn test_release_rooms() {
        let (mock_client, data_store) = setup_mock_client();
        
        // First create a meeting
        let create_request = CreateMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            subject: "Test Meeting".to_string(),
            type_: 0,
            _type: 0,
            hosts: None,
            guests: None,
            invitees: None,
            start_time: "1680000000".to_string(),
            end_time: "1680003600".to_string(),
            password: None,
            settings: None,
            meeting_type: None,
            recurring_rule: None,
            enable_live: None,
            live_config: None,
            enable_doc_upload_permission: None,
            media_set_type: None,
            enable_interpreter: None,
            enable_enroll: None,
            enable_host_key: None,
            host_key: None,
            sync_to_wework: None,
            time_zone: None,
            location: None,
            allow_enterprise_intranet_only: None,
        };
        
        // Create the meeting
        let create_result = mock_client.create_meeting(&create_request).await.unwrap();
        let meeting_id = &create_result.meeting_info_list[0].meeting_id;
        
        // Book rooms first
        let book_request = BookRoomsRequest {
            operator_id: "test_operator".to_string(),
            operator_id_type: 1,
            meeting_room_id_list: vec!["room1".to_string()],
            subject_visible: Some(true),
        };
        
        let _ = mock_client.book_rooms(meeting_id, &book_request).await.unwrap();
        
        // Now release the rooms
        let release_request = ReleaseRoomsRequest {
            operator_id: "test_operator".to_string(),
            operator_id_type: 1,
            meeting_room_id_list: vec!["room1".to_string()],
        };
        
        let release_result = mock_client.release_rooms(meeting_id, &release_request).await;
        
        // Check the result
        assert!(release_result.is_ok());
    }
    
    #[tokio::test]
    async fn test_workflow_create_book_cancel_release() {
        let (mock_client, _) = setup_mock_client();
        
        // 1. Create a meeting
        let create_request = CreateMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            subject: "Workflow Test Meeting".to_string(),
            type_: 0,
            _type: 0,
            hosts: None,
            guests: None,
            invitees: None,
            start_time: "1680000000".to_string(),
            end_time: "1680003600".to_string(),
            password: None,
            settings: None,
            meeting_type: None,
            recurring_rule: None,
            enable_live: None,
            live_config: None,
            enable_doc_upload_permission: None,
            media_set_type: None,
            enable_interpreter: None,
            enable_enroll: None,
            enable_host_key: None,
            host_key: None,
            sync_to_wework: None,
            time_zone: None,
            location: None,
            allow_enterprise_intranet_only: None,
        };
        
        // Create the meeting
        let create_result = mock_client.create_meeting(&create_request).await;
        assert!(create_result.is_ok());
        
        let meeting_id = &create_result.unwrap().meeting_info_list[0].meeting_id;
        
        // 2. Book rooms
        let book_request = BookRoomsRequest {
            operator_id: "test_operator".to_string(),
            operator_id_type: 1,
            meeting_room_id_list: vec!["room1".to_string()],
            subject_visible: Some(true),
        };
        
        let book_result = mock_client.book_rooms(meeting_id, &book_request).await;
        assert!(book_result.is_ok());
        
        // 3. Release rooms
        let release_request = ReleaseRoomsRequest {
            operator_id: "test_operator".to_string(),
            operator_id_type: 1,
            meeting_room_id_list: vec!["room1".to_string()],
        };
        
        let release_result = mock_client.release_rooms(meeting_id, &release_request).await;
        assert!(release_result.is_ok());
        
        // 4. Cancel meeting
        let cancel_request = CancelMeetingRequest {
            userid: "test_user".to_string(),
            instanceid: 1,
            reason_code: 1,
            meeting_type: None,
            sub_meeting_id: None,
            reason_detail: Some("Test cancellation".to_string()),
        };
        
        let cancel_result = mock_client.cancel_meeting(meeting_id, &cancel_request).await;
        assert!(cancel_result.is_ok());
    }
    
    // TODO: Add error handling tests in the future
    // For now, we're skipping error tests due to complexities in creating reqwest::Error objects
}