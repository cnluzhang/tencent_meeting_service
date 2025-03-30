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
    let meeting_start_time = match DateTime::parse_from_rfc3339(scheduled_at_str) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(e) => {
            return Err(format!("Failed to parse scheduled_at time: {}", e));
        }
    };

    // Parse the scheduled label to determine meeting duration
    // Format expected: "2025-03-30 09:00-10:00" or similar
    let scheduled_label = &reservation.scheduled_label;
    let parts: Vec<&str> = scheduled_label.split(' ').collect();
    let mut meeting_end_time = meeting_start_time + chrono::Duration::hours(1); // Default 1 hour

    if parts.len() > 1 {
        let time_parts: Vec<&str> = parts[1].split('-').collect();
        if time_parts.len() > 1 {
            let start_time_str = time_parts[0];
            let end_time_str = time_parts[1];

            // Parse hour difference
            if let (Some(start_hour), Some(end_hour)) = (
                start_time_str
                    .split(':')
                    .next()
                    .and_then(|h| h.parse::<i64>().ok()),
                end_time_str
                    .split(':')
                    .next()
                    .and_then(|h| h.parse::<i64>().ok()),
            ) {
                let hours_diff = if end_hour > start_hour {
                    end_hour - start_hour
                } else {
                    24 + end_hour - start_hour // Handle overnight meetings
                };

                meeting_end_time = meeting_start_time + chrono::Duration::hours(hours_diff);
            }
        }
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
        time_zone: Some("Asia/Shanghai".to_string()),
        allow_enterprise_intranet_only: None,
        guests: None,
    };

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
        time_zone: Some("Asia/Shanghai".to_string()),
        allow_enterprise_intranet_only: None,
        guests: None,
    };

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
