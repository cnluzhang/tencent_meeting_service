use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::client::{CreateMeetingRequest, TencentMeetingClient};
use crate::models::form::FormField1Item;
use crate::models::form::FormSubmission;
use crate::models::meeting::{MeetingResult, TimeSlot};

// Helper function to determine location based on form name
fn get_location_for_form(form_name: &str, room_name: &str) -> String {
    debug!(
        "Getting location for form: {}, room: {}",
        form_name, room_name
    );

    match form_name {
        "西安会议室预约" => "西安-大会议室".to_string(),
        "成都会议室预约" => "成都-天府广场".to_string(),
        _ => format!("{} (Unknown Location)", room_name), // Default fallback
    }
}

// Helper function to determine which room ID to use based on form name
pub fn get_room_id_for_form(form_name: &str, xa_room_id: &str, cd_room_id: &str) -> String {
    debug!("Getting room ID for form: {}", form_name);

    match form_name {
        "西安会议室预约" => xa_room_id.to_string(),
        "成都会议室预约" => cd_room_id.to_string(),
        _ => {
            warn!(
                "Unknown form name: {}, using Xi'an room ID as default",
                form_name
            );
            xa_room_id.to_string() // Default to Xi'an room ID
        }
    }
}

// Helper function to get operator name and ID from form submission
pub fn get_operator_info(
    client: &TencentMeetingClient,
    form: &FormSubmission,
    user_field_name: &str,
) -> (String, String) {
    // Extract the user name from the form using the configured field name
    let operator_name = match form.entry.extra_fields.get(user_field_name) {
        Some(value) => {
            // Extract the string value
            if let Some(name_str) = value.as_str() {
                debug!(
                    "Found operator name '{}' in field '{}'",
                    name_str, user_field_name
                );
                name_str.to_string()
            } else {
                // Convert other types to string if possible
                let name = value.to_string().trim_matches('"').to_string();
                debug!(
                    "Converted operator name '{}' from field '{}'",
                    name, user_field_name
                );
                name
            }
        }
        None => {
            // If field is not found, use a default value
            warn!(
                "User field '{}' not found in form submission, using default",
                user_field_name
            );
            "default".to_string()
        }
    };

    // Get the corresponding operator ID
    let operator_id = client.get_operator_id_by_name(&operator_name);

    info!(
        "Resolved operator name '{}' to ID '{}'",
        operator_name, operator_id
    );

    (operator_name, operator_id)
}

// Parse a scheduled time from a form field item
pub fn parse_time_slot(reservation: &FormField1Item) -> Result<TimeSlot, String> {
    // Parse the scheduled time
    let scheduled_at_str = &reservation.scheduled_at;
    let parsed_start_time = match DateTime::parse_from_rfc3339(scheduled_at_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(e) => {
            return Err(format!("Failed to parse scheduled_at time: {}", e));
        }
    };

    // Parse the scheduled label to determine meeting duration and original end time
    // Format expected: "2025-03-30 09:00-10:00" or similar
    let scheduled_label = &reservation.scheduled_label;
    let parts: Vec<&str> = scheduled_label.split(' ').collect();

    // Calculate the original end time based on the label first
    let mut original_end_time = parsed_start_time + chrono::Duration::hours(1); // Default 1 hour

    if parts.len() > 1 {
        let time_parts: Vec<&str> = parts[1].split('-').collect();
        if time_parts.len() > 1 {
            let start_time_str = time_parts[0];
            let end_time_str = time_parts[1];

            // Parse full time including both hours and minutes
            let parse_time = |time_str: &str| {
                let parts: Vec<&str> = time_str.split(':').collect();
                let hour = parts
                    .first()
                    .and_then(|h| h.parse::<i64>().ok())
                    .unwrap_or(0);
                let minute = parts
                    .get(1)
                    .and_then(|m| m.parse::<i64>().ok())
                    .unwrap_or(0);
                (hour, minute)
            };

            let (start_hour, start_min) = parse_time(start_time_str);
            let (end_hour, end_min) = parse_time(end_time_str);

            // Calculate total minutes
            let start_total_mins = start_hour * 60 + start_min;
            let end_total_mins = end_hour * 60 + end_min;

            let duration_mins = if end_total_mins >= start_total_mins {
                end_total_mins - start_total_mins
            } else {
                // Handle overnight meetings
                (24 * 60) + end_total_mins - start_total_mins
            };

            debug!(
                "Time range {}-{} calculated as {} minutes difference",
                start_time_str, end_time_str, duration_mins
            );

            original_end_time = parsed_start_time + chrono::Duration::minutes(duration_mins);
        }
    }

    // Check if both start and end times are in the past
    let now = Utc::now();
    let meeting_start_time;
    let meeting_end_time;

    if parsed_start_time < now && original_end_time < now {
        // Both start and end times are in the past - return error instead of adjusting
        error!(
            "Both start time {} and end time {} are in the past",
            parsed_start_time, original_end_time
        );
        return Err(
            "Time slot is entirely in the past. Cannot create a meeting for past times."
                .to_string(),
        );
    } else if parsed_start_time < now {
        // Only start time is in the past, end time is in the future
        debug!(
            "Scheduled time {} is in the past, using current time + 2 minutes instead",
            parsed_start_time
        );
        meeting_start_time = now + chrono::Duration::minutes(2);
        meeting_end_time = original_end_time; // Keep the original end time
        debug!(
            "Using original end time {} despite adjusted start time",
            meeting_end_time
        );
    } else {
        // Both times are fine
        meeting_start_time = parsed_start_time;
        meeting_end_time = original_end_time;
    }

    Ok(TimeSlot {
        item_name: reservation.item_name.clone(),
        scheduled_label: reservation.scheduled_label.clone(),
        number: reservation.number,
        start_time: meeting_start_time,
        end_time: meeting_end_time,
        api_code: reservation.api_code.clone(),
    })
}

// Attempt to find mergeable groups in time slots
pub fn find_mergeable_groups(slots: &[TimeSlot]) -> Vec<Vec<TimeSlot>> {
    if slots.is_empty() {
        return Vec::new();
    }

    // Group slots by room name
    let mut room_groups: HashMap<String, Vec<TimeSlot>> = HashMap::new();
    for slot in slots {
        room_groups
            .entry(slot.item_name.clone())
            .or_default()
            .push(slot.clone());
    }

    let mut mergeable_groups = Vec::new();

    // Process each room's slots
    for (_, mut room_slots) in room_groups {
        // Sort by start time
        room_slots.sort_by_key(|slot| slot.start_time);

        // Find continuous groups
        let mut current_group = vec![room_slots[0].clone()];

        for slot in room_slots.iter().skip(1) {
            let last_slot = &current_group.last().unwrap();

            // If this slot starts exactly when the previous one ends, merge them
            if last_slot.end_time == slot.start_time {
                current_group.push(slot.clone());
            } else {
                // Otherwise start a new group
                if !current_group.is_empty() {
                    mergeable_groups.push(current_group);
                }
                current_group = vec![slot.clone()];
            }
        }

        // Add the last group if not empty
        if !current_group.is_empty() {
            mergeable_groups.push(current_group);
        }
    }

    mergeable_groups
}

// Create a meeting with the given time slot
pub async fn create_meeting_with_time_slot(
    client: &TencentMeetingClient,
    _dept_field_name: &str, // Preserved for API compatibility
    form_submission: &FormSubmission,
    time_slot: &TimeSlot,
    user_field_name: &str,
) -> Result<MeetingResult, StatusCode> {
    // Get operator information based on the form submission
    let (operator_name, operator_id) = get_operator_info(client, form_submission, user_field_name);

    // Create meeting request with the specific operator ID from form data
    let meeting_request = CreateMeetingRequest {
        userid: operator_id.clone(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        invitees: None,
        start_time: time_slot.start_time.timestamp().to_string(),
        end_time: time_slot.end_time.timestamp().to_string(),
        password: None,
        location: Some(get_location_for_form(
            &form_submission.form_name,
            time_slot.item_name.as_str(),
        )),
        time_zone: Some("Asia/Shanghai".to_string()),
        guests: None,
    };

    // Add additional debug logging
    debug!(
        "Creating meeting with timestamps: start={} ({}), end={} ({}), duration={} mins",
        time_slot.start_time.timestamp(),
        time_slot.start_time,
        time_slot.end_time.timestamp(),
        time_slot.end_time,
        (time_slot.end_time - time_slot.start_time).num_minutes()
    );

    info!(
        "Creating meeting for room: {} with time range: {}-{} with operator: {} (ID: {})",
        time_slot.item_name, time_slot.start_time, time_slot.end_time, operator_name, operator_id
    );

    // Call the Tencent Meeting API to create the meeting
    match client.create_meeting(&meeting_request).await {
        Ok(response) => {
            if response.meeting_info_list.is_empty() {
                error!("Meeting created but no meeting info returned");
                Ok(MeetingResult {
                    meeting_id: None,
                    merged: false,
                    room_name: time_slot.item_name.clone(),
                    time_slots: vec![time_slot.scheduled_label.clone()],
                    success: true,
                })
            } else {
                let meeting_info = &response.meeting_info_list[0];
                info!(
                    "Successfully created meeting: {} with ID: {}",
                    meeting_info.subject, meeting_info.meeting_id
                );

                Ok(MeetingResult {
                    meeting_id: Some(meeting_info.meeting_id.clone()),
                    merged: false,
                    room_name: time_slot.item_name.clone(),
                    time_slots: vec![time_slot.scheduled_label.clone()],
                    success: true,
                })
            }
        }
        Err(err) => {
            error!("Failed to create meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Create a merged meeting from multiple time slots
pub async fn create_merged_meeting(
    client: &TencentMeetingClient,
    _dept_field_name: &str, // Preserved for API compatibility
    form_submission: &FormSubmission,
    time_slots: &[TimeSlot],
    user_field_name: &str,
) -> Result<MeetingResult, StatusCode> {
    if time_slots.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Sort time slots to ensure correct merging
    let mut sorted_slots = time_slots.to_vec();
    sorted_slots.sort_by_key(|slot| slot.start_time);

    // Use the earliest start time and latest end time to create a merged meeting
    let start_time = sorted_slots.first().unwrap().start_time;
    let end_time = sorted_slots.last().unwrap().end_time;
    let room_name = &sorted_slots[0].item_name;

    // Collect all time slot labels for reporting
    let time_slot_labels: Vec<String> = sorted_slots
        .iter()
        .map(|slot| slot.scheduled_label.clone())
        .collect();

    // Get operator information based on the form submission
    let (operator_name, operator_id) = get_operator_info(client, form_submission, user_field_name);

    // Log merged slot details
    info!(
        "Creating merged time slot for room: {}, slots: {}, time range: {}-{} with operator: {} (ID: {})",
        room_name,
        time_slots.len(),
        start_time,
        end_time,
        operator_name,
        operator_id
    );

    // Create meeting request with merged time and specific operator
    let meeting_request = CreateMeetingRequest {
        userid: operator_id.clone(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        invitees: None,
        start_time: start_time.timestamp().to_string(),
        end_time: end_time.timestamp().to_string(),
        password: None,
        location: Some(get_location_for_form(
            &form_submission.form_name,
            room_name.as_str(),
        )),
        time_zone: Some("Asia/Shanghai".to_string()),
        guests: None,
    };

    // Add additional debug logging
    debug!(
        "Creating merged meeting with timestamps: start={} ({}), end={} ({}), duration={} mins",
        start_time.timestamp(),
        start_time,
        end_time.timestamp(),
        end_time,
        (end_time - start_time).num_minutes()
    );

    info!(
        "Creating merged meeting for room: {} with time range: {}-{}",
        room_name, start_time, end_time
    );

    // Call the Tencent Meeting API to create the meeting
    match client.create_meeting(&meeting_request).await {
        Ok(response) => {
            if response.meeting_info_list.is_empty() {
                error!("Merged meeting created but no meeting info returned");
                Ok(MeetingResult {
                    meeting_id: None,
                    merged: true,
                    room_name: room_name.clone(),
                    time_slots: time_slot_labels,
                    success: true,
                })
            } else {
                let meeting_info = &response.meeting_info_list[0];
                info!(
                    "Successfully created merged meeting: {} with ID: {}",
                    meeting_info.subject, meeting_info.meeting_id
                );

                Ok(MeetingResult {
                    meeting_id: Some(meeting_info.meeting_id.clone()),
                    merged: true,
                    room_name: room_name.clone(),
                    time_slots: time_slot_labels,
                    success: true,
                })
            }
        }
        Err(err) => {
            error!("Failed to create merged meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
