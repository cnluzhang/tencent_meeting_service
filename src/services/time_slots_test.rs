#[cfg(test)]
mod time_slots_tests {
    use chrono::{DateTime, TimeZone, Utc};
    use std::thread;
    use std::time::Duration as StdDuration;
    
    use crate::services::time_slots::{parse_time_slot, find_mergeable_groups};
    use crate::models::form::FormField1Item;
    use crate::models::meeting::TimeSlot;
    
    #[test]
    fn test_parse_time_slot() {
        // Test standard time format
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            scheduled_at: "2025-04-01T01:00:00.000Z".to_string(), // UTC time
            api_code: "CODE1".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_ok());
        
        let time_slot = result.unwrap();
        assert_eq!(time_slot.item_name, "Test Room");
        assert_eq!(time_slot.scheduled_label, "2025-04-01 09:00-10:00");
        
        // Check that duration is 1 hour
        let duration = time_slot.end_time - time_slot.start_time;
        assert_eq!(duration.num_hours(), 1);
        
        // Test multi-hour format
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 09:00-11:00".to_string(), // 2-hour meeting
            number: 1,
            scheduled_at: "2025-04-01T01:00:00.000Z".to_string(),
            api_code: "CODE1".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_ok());
        
        let time_slot = result.unwrap();
        
        // Check that duration is 2 hours
        let duration = time_slot.end_time - time_slot.start_time;
        assert_eq!(duration.num_hours(), 2);
    }
    
    #[test]
    fn test_find_mergeable_groups_empty() {
        let slots: Vec<TimeSlot> = Vec::new();
        let result = find_mergeable_groups(&slots);
        assert!(result.is_empty());
    }
    
    #[test]
    fn test_find_mergeable_groups_single() {
        let start_time = Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap();
        let end_time = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        
        let slot = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time,
            end_time,
            api_code: "CODE1".to_string(),
        };
        
        let slots = vec![slot];
        let result = find_mergeable_groups(&slots);
        
        // Should have one group with one slot
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].scheduled_label, "2025-04-01 09:00-10:00");
    }
    
    #[test]
    fn test_find_mergeable_groups_consecutive() {
        let start_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap();
        let end_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        
        let slot1 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time: start_time1,
            end_time: end_time1,
            api_code: "CODE1".to_string(),
        };
        
        let start_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        let end_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 11, 0, 0).unwrap();
        
        let slot2 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 10:00-11:00".to_string(),
            number: 2,
            start_time: start_time2,
            end_time: end_time2,
            api_code: "CODE2".to_string(),
        };
        
        let slots = vec![slot1, slot2];
        let result = find_mergeable_groups(&slots);
        
        // Should have one group with two slots (they're mergeable)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
    }
    
    #[test]
    fn test_find_mergeable_groups_non_consecutive() {
        let start_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap();
        let end_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        
        let slot1 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time: start_time1,
            end_time: end_time1,
            api_code: "CODE1".to_string(),
        };
        
        // Gap between meetings
        let start_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 11, 0, 0).unwrap(); 
        let end_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 12, 0, 0).unwrap();
        
        let slot2 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 11:00-12:00".to_string(),
            number: 2,
            start_time: start_time2,
            end_time: end_time2,
            api_code: "CODE2".to_string(),
        };
        
        let slots = vec![slot1, slot2];
        let result = find_mergeable_groups(&slots);
        
        // Should have two separate groups (slots are not mergeable)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[1].len(), 1);
    }
    
    #[test]
    fn test_find_mergeable_groups_different_rooms() {
        let start_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap();
        let end_time1 = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        
        let slot1 = TimeSlot {
            item_name: "Room A".to_string(), // First room
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time: start_time1,
            end_time: end_time1,
            api_code: "CODE1".to_string(),
        };
        
        let start_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap();
        let end_time2 = Utc.with_ymd_and_hms(2025, 4, 1, 11, 0, 0).unwrap();
        
        let slot2 = TimeSlot {
            item_name: "Room B".to_string(), // Different room
            scheduled_label: "2025-04-01 10:00-11:00".to_string(),
            number: 2,
            start_time: start_time2,
            end_time: end_time2,
            api_code: "CODE2".to_string(),
        };
        
        let slots = vec![slot1, slot2];
        let result = find_mergeable_groups(&slots);
        
        // Should have two separate groups (different rooms)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[1].len(), 1);
    }
    
    #[test]
    fn test_find_mergeable_groups_complex() {
        // Create a complex scenario with multiple rooms and consecutive slots
        
        // Room A, 9:00-10:00
        let slot1 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 1,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
            api_code: "CODE1".to_string(),
        };
        
        // Room A, 10:00-11:00 (consecutive with slot1)
        let slot2 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 10:00-11:00".to_string(),
            number: 2,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 11, 0, 0).unwrap(),
            api_code: "CODE2".to_string(),
        };
        
        // Room B, 9:00-10:00
        let slot3 = TimeSlot {
            item_name: "Room B".to_string(),
            scheduled_label: "2025-04-01 09:00-10:00".to_string(),
            number: 3,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 9, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 10, 0, 0).unwrap(),
            api_code: "CODE3".to_string(),
        };
        
        // Room B, 10:30-11:30 (non-consecutive with slot3)
        let slot4 = TimeSlot {
            item_name: "Room B".to_string(),
            scheduled_label: "2025-04-01 10:30-11:30".to_string(),
            number: 4,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 10, 30, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 11, 30, 0).unwrap(),
            api_code: "CODE4".to_string(),
        };
        
        let slots = vec![slot1, slot2, slot3, slot4];
        let result = find_mergeable_groups(&slots);
        
        // Should have three groups:
        // 1. Room A 9:00-11:00 (merged slots 1 and 2)
        // 2. Room B 9:00-10:00 (slot 3)
        // 3. Room B 10:30-11:30 (slot 4)
        assert_eq!(result.len(), 3);
        
        // First group should have 2 slots (Room A merged)
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0].item_name, "Room A");
        
        // The other groups should have 1 slot each (Room B, non-consecutive)
        assert_eq!(result[1].len(), 1);
        assert_eq!(result[1][0].item_name, "Room B");
        
        assert_eq!(result[2].len(), 1);
        assert_eq!(result[2][0].item_name, "Room B");
    }
    
    #[test]
    fn test_parse_time_slot_with_minutes() {
        // Test time slot with 30-minute precision
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 14:00-14:30".to_string(), // 30 minutes
            number: 1,
            scheduled_at: "2025-04-01T06:00:00.000Z".to_string(),
            api_code: "CODE1".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_ok());
        
        let time_slot = result.unwrap();
        
        // Check that duration is 30 minutes
        let duration = time_slot.end_time - time_slot.start_time;
        assert_eq!(duration.num_minutes(), 30);
        
        // Test another 30-minute slot
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: "2025-04-01 14:30-15:00".to_string(), // 30 minutes
            number: 2,
            scheduled_at: "2025-04-01T06:30:00.000Z".to_string(),
            api_code: "CODE2".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_ok());
        
        let time_slot = result.unwrap();
        
        // Check that duration is 30 minutes
        let duration = time_slot.end_time - time_slot.start_time;
        assert_eq!(duration.num_minutes(), 30);
    }
    
    #[test]
    fn test_consecutive_30min_slots_are_mergeable() {
        // Create two consecutive 30-minute slots in the same room
        let slot1 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 14:00-14:30".to_string(),
            number: 1,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 14, 0, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 14, 30, 0).unwrap(),
            api_code: "CODE1".to_string(),
        };
        
        let slot2 = TimeSlot {
            item_name: "Room A".to_string(),
            scheduled_label: "2025-04-01 14:30-15:00".to_string(),
            number: 2,
            start_time: Utc.with_ymd_and_hms(2025, 4, 1, 14, 30, 0).unwrap(),
            end_time: Utc.with_ymd_and_hms(2025, 4, 1, 15, 0, 0).unwrap(),
            api_code: "CODE2".to_string(),
        };
        
        let slots = vec![slot1, slot2];
        let result = find_mergeable_groups(&slots);
        
        // Should have one group with two slots (they're mergeable)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
        
        // Verify the time slots are in the right order
        assert_eq!(result[0][0].scheduled_label, "2025-04-01 14:00-14:30");
        assert_eq!(result[0][1].scheduled_label, "2025-04-01 14:30-15:00");
    }
    
    #[test]
    fn test_past_time_adjustment() {
        // Create a time slot with a past time and future end time
        let now = Utc::now();
        let past_time = now - chrono::Duration::hours(1); // 1 hour in the past
        let future_time = now + chrono::Duration::hours(1); // 1 hour in the future
        
        // Create a time string that would normally span 2 hours
        let past_rfc3339 = past_time.to_rfc3339();
        let time_label = format!(
            "{}-{}", 
            past_time.format("%Y-%m-%d %H:%M"),
            future_time.format("%H:%M")
        );
        
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: time_label,
            number: 1,
            scheduled_at: past_rfc3339,
            api_code: "CODE1".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_ok());
        
        let time_slot = result.unwrap();
        
        // Check that start time is adjusted to now + 2 minutes
        assert!(time_slot.start_time > now);
        let start_diff = (time_slot.start_time - now).num_seconds();
        // Allow for a small margin of error in the test due to execution time
        assert!(start_diff >= 115 && start_diff <= 125); // ~120 seconds (2 minutes)
        
        // Check that end time is preserved from the original time
        // It should be approximately the same as our future_time
        let end_diff = (time_slot.end_time - future_time).num_seconds().abs();
        assert!(end_diff < 5); // Allow small difference due to string formatting/parsing
        
        // Duration should be shorter than the original 2 hours
        // since start time was pushed forward but end time stayed the same
        let duration = time_slot.end_time - time_slot.start_time;
        assert!(duration.num_minutes() < 120);
        assert!(duration.num_minutes() > 50); // Approximately 60 minutes
    }
    
    #[test]
    fn test_past_time_with_past_end_time() {
        // Test when both start and end times are in the past
        let now = Utc::now();
        let past_start = now - chrono::Duration::minutes(30); // 30 minutes in the past
        let past_end = now - chrono::Duration::minutes(15); // 15 minutes in the past
        
        // Create a time string for a past time slot
        let past_rfc3339 = past_start.to_rfc3339();
        let time_label = format!(
            "{}-{}", 
            past_start.format("%Y-%m-%d %H:%M"),
            past_end.format("%H:%M")
        );
        
        let item = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: time_label,
            number: 1,
            scheduled_at: past_rfc3339,
            api_code: "CODE1".to_string(),
        };
        
        let result = parse_time_slot(&item);
        assert!(result.is_err());
        
        // Verify error message
        let error = result.unwrap_err();
        assert!(error.contains("Time slot is entirely in the past"));
    }
    
    #[test]
    fn test_consecutive_past_time_slots() {
        // Test that time slots with past start but future end times still work properly
        let now = Utc::now();
        
        // We can no longer test a completely past time slot as it will error
        // First time slot: 15 minutes in the past to 15 minutes in the future (spans current time)
        let span_slot = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: format!("{}-{}",
                (now - chrono::Duration::minutes(15)).format("%Y-%m-%d %H:%M"),
                (now + chrono::Duration::minutes(15)).format("%H:%M")
            ),
            number: 1,
            scheduled_at: (now - chrono::Duration::minutes(15)).to_rfc3339(),
            api_code: "CODE1".to_string(),
        };
        
        // Second time slot: 0 minutes in the future to 30 minutes in the future
        let future_slot = FormField1Item {
            item_name: "Test Room".to_string(),
            scheduled_label: format!("{}-{}",
                now.format("%Y-%m-%d %H:%M"),
                (now + chrono::Duration::minutes(30)).format("%H:%M")
            ),
            number: 2,
            scheduled_at: now.to_rfc3339(),
            api_code: "CODE2".to_string(),
        };
        
        // Parse the slots
        let span_result = parse_time_slot(&span_slot).unwrap();
        let future_result = parse_time_slot(&future_slot).unwrap();
        
        // Create mergeable groups
        let slots = vec![span_result.clone(), future_result.clone()];
        let result = find_mergeable_groups(&slots);
        
        // They should merge into a single group with 2 slots
        assert_eq!(result.len(), 1, "Should have 1 group of merged slots");
        assert_eq!(result[0].len(), 2, "Group should contain 2 slots");
        
        // The order should be preserved based on adjusted times
        assert_eq!(result[0][0].number, span_result.number);
        assert_eq!(result[0][1].number, future_result.number);
    }
}