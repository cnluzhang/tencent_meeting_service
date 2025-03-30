#[cfg(test)]
mod database_tests {
    use std::fs;
    use std::path::Path;
    use chrono::Utc;
    use tempfile::tempdir;
    use std::collections::HashMap;
    
    use crate::services::database::DatabaseService;
    use crate::models::form::{FormSubmission, FormEntry, FormField1Item};
    use crate::models::meeting::TimeSlot;
    
    fn create_test_form() -> FormSubmission {
        let field_item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            scheduled_at: "2025-04-01T01:00:00.000Z".to_string(),
            api_code: "CODE1".to_string(),
        };
        
        let mut extra_fields = HashMap::new();
        extra_fields.insert("user_field_name".to_string(), "Test User".into());
        extra_fields.insert("department_field_name".to_string(), "Test Dept".into());
        
        FormSubmission {
            form: "test_form".to_string(),
            form_name: "Test Form".to_string(),
            entry: FormEntry {
                token: "test_token".to_string(),
                field_1: vec![field_item],
                field_8: "Test Meeting".to_string(),
                extra_fields,
                reservation_status_fsf_field: "已预约".to_string(),
            },
        }
    }
    
    fn create_time_slot() -> TimeSlot {
        let start_time = Utc::now();
        let end_time = start_time + chrono::Duration::hours(1);
        
        TimeSlot {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time,
            end_time,
            api_code: "CODE1".to_string(),
        }
    }
    
    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Check that the CSV file was created
        assert!(Path::new(csv_path_str).exists());
        
        // Clean up
        dir.close().unwrap();
    }
    
    #[test]
    fn test_store_meeting_with_time_slot() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Create test data
        let form = create_test_form();
        let time_slot = create_time_slot();
        
        // Store meeting
        let result = db.store_meeting_with_time_slot(
            &form, 
            "meeting123", 
            "Test Room", 
            "room123", 
            &time_slot
        );
        
        assert!(result.is_ok());
        
        // Find the meeting
        let retrieved = db.find_meeting_by_token(&form.entry.token);
        assert!(retrieved.is_ok());
        let meeting = retrieved.unwrap();
        assert!(meeting.is_some());
        let meeting = meeting.unwrap();
        
        // Check fields
        assert_eq!(meeting.entry_token, "test_token");
        assert_eq!(meeting.meeting_id, "meeting123");
        assert_eq!(meeting.scheduled_label, "2025-04-01 09:00-10:00");
        assert_eq!(meeting.status, "已预约");
        
        // Clean up
        dir.close().unwrap();
    }
    
    #[test]
    fn test_store_merged_meeting() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Create test data
        let form = create_test_form();
        let mut time_slots = Vec::new();
        
        // Create two consecutive time slots
        let start_time1 = Utc::now();
        let end_time1 = start_time1 + chrono::Duration::hours(1);
        let slot1 = TimeSlot {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time: start_time1,
            end_time: end_time1,
            api_code: "CODE1".to_string(),
        };
        
        let start_time2 = end_time1;
        let end_time2 = start_time2 + chrono::Duration::hours(1);
        let slot2 = TimeSlot {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 10:00-11:00".to_string(),
            number: 2,
            start_time: start_time2,
            end_time: end_time2,
            api_code: "CODE2".to_string(),
        };
        
        time_slots.push(slot1);
        time_slots.push(slot2);
        
        // Store merged meeting
        let result = db.store_merged_meeting(
            &form, 
            "meeting123", 
            "Test Room", 
            "room123", 
            &time_slots
        );
        
        assert!(result.is_ok());
        
        // Find the meeting
        let retrieved = db.find_meeting_by_token(&form.entry.token);
        assert!(retrieved.is_ok());
        let meeting = retrieved.unwrap();
        assert!(meeting.is_some());
        let meeting = meeting.unwrap();
        
        // Check fields
        assert_eq!(meeting.entry_token, "test_token");
        assert_eq!(meeting.meeting_id, "meeting123");
        assert_eq!(meeting.scheduled_label, "2025-04-01 09:00-11:00"); // Combined label
        assert_eq!(meeting.status, "已预约");
        
        // Clean up
        dir.close().unwrap();
    }
    
    #[test]
    fn test_cancel_meeting() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Create test data
        let form = create_test_form();
        let time_slot = create_time_slot();
        
        // Store meeting
        let result = db.store_meeting_with_time_slot(
            &form, 
            "meeting123", 
            "Test Room", 
            "room123", 
            &time_slot
        );
        
        assert!(result.is_ok());
        
        // Cancel the meeting
        let cancelled = db.cancel_meeting(&form.entry.token);
        assert!(cancelled.is_ok());
        let cancelled_ids = cancelled.unwrap();
        assert_eq!(cancelled_ids.len(), 1);
        assert_eq!(cancelled_ids[0].0, "meeting123"); // meeting_id
        assert_eq!(cancelled_ids[0].1, "room123"); // room_id
        
        // Check that status was updated
        let retrieved = db.find_all_meetings_by_token(&form.entry.token);
        assert!(retrieved.is_ok());
        let meetings = retrieved.unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].status, "已取消");
        assert!(!meetings[0].cancelled_at.is_empty());
        
        // Clean up
        dir.close().unwrap();
    }
    
    #[test]
    fn test_multiple_meetings_same_token() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Create test data
        let mut form = create_test_form();
        
        // First time slot
        let slot1 = FormField1Item {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            scheduled_at: "2025-04-01T01:00:00.000Z".to_string(),
            api_code: "CODE1".to_string(),
        };
        
        // Second time slot
        let slot2 = FormField1Item {
            item_name: "Room B".to_string(),
            scheduled_label: "2025-04-01 11:00-12:00".to_string(),
            number: 2,
            scheduled_at: "2025-04-01T03:00:00.000Z".to_string(),
            api_code: "CODE2".to_string(),
        };
        
        form.entry.field_1 = vec![slot1.clone(), slot2.clone()];
        
        // Create time slots
        let time_slot1 = TimeSlot {
            item_name: slot1.item_name.clone(),
            scheduled_label: slot1.scheduled_label.clone(),
            number: slot1.number,
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::hours(1),
            api_code: slot1.api_code.clone(),
        };
        
        let time_slot2 = TimeSlot {
            item_name: slot2.item_name.clone(),
            scheduled_label: slot2.scheduled_label.clone(),
            number: slot2.number,
            start_time: Utc::now() + chrono::Duration::hours(2),
            end_time: Utc::now() + chrono::Duration::hours(3),
            api_code: slot2.api_code.clone(),
        };
        
        // Store two meetings with the same token but different times
        db.store_meeting_with_time_slot(&form, "meeting1", "Room A", "room1", &time_slot1).unwrap();
        db.store_meeting_with_time_slot(&form, "meeting2", "Room B", "room2", &time_slot2).unwrap();
        
        // Find all meetings
        let retrieved = db.find_all_meetings_by_token(&form.entry.token);
        assert!(retrieved.is_ok());
        let meetings = retrieved.unwrap();
        
        // Should have two meetings with same token but different rooms/times
        assert_eq!(meetings.len(), 2);
        
        // Check that they have different room names and scheduled labels
        let meeting_infos: Vec<(String, String)> = meetings
            .iter()
            .map(|m| (m.room_name.clone(), m.scheduled_label.clone()))
            .collect();
        
        assert!(meeting_infos.contains(&("Room A".to_string(), "2025-04-01 09:00-10:00".to_string())));
        assert!(meeting_infos.contains(&("Room B".to_string(), "2025-04-01 11:00-12:00".to_string())));
        
        // Cancel all meetings
        let cancelled = db.cancel_meeting(&form.entry.token);
        assert!(cancelled.is_ok());
        let cancelled_ids = cancelled.unwrap();
        
        // Both meetings should be cancelled
        assert_eq!(cancelled_ids.len(), 2);
        
        // Clean up
        dir.close().unwrap();
    }
    
    #[test]
    fn test_deduplication() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test_meetings.csv");
        let csv_path_str = csv_path.to_str().unwrap();
        
        // Create database service
        let db = DatabaseService::new(csv_path_str);
        
        // Create test data
        let form = create_test_form();
        let time_slot = create_time_slot();
        
        // Store the same meeting twice
        db.store_meeting_with_time_slot(&form, "meeting1", "Test Room", "room1", &time_slot).unwrap();
        db.store_meeting_with_time_slot(&form, "meeting2", "Test Room", "room1", &time_slot).unwrap();
        
        // Find all meetings - should only have one due to deduplication
        let retrieved = db.find_all_meetings_by_token(&form.entry.token);
        assert!(retrieved.is_ok());
        let meetings = retrieved.unwrap();
        
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].meeting_id, "meeting1"); // Only the first one should be stored
        
        // Clean up
        dir.close().unwrap();
    }
}