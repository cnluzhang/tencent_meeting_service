use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{error, info, debug, warn};

use crate::client::{CreateMeetingRequest, MeetingSettings, TencentMeetingClient, User};
use crate::models::form::FormField1Item;
use crate::models::form::FormSubmission;
use crate::models::meeting::{MeetingResult, TimeSlot};

// Helper function to determine location based on form name
fn get_location_for_form(form_name: &str, room_name: &str) -> String {
    debug!("Getting location for form: {}, room: {}", form_name, room_name);
    
    match form_name {
        name if name == "西安会议室预约" => "西安-大会议室".to_string(),
        name if name == "成都会议室预约" => "成都-天府广场".to_string(),
        _ => format!("{} (Unknown Location)", room_name) // Default fallback
    }
}

// Helper function to determine which room ID to use based on form name
pub fn get_room_id_for_form(form_name: &str, xa_room_id: &str, cd_room_id: &str) -> String {
    debug!("Getting room ID for form: {}", form_name);
    
    match form_name {
        name if name == "西安会议室预约" => xa_room_id.to_string(),
        name if name == "成都会议室预约" => cd_room_id.to_string(),
        _ => {
            warn!("Unknown form name: {}, using Xi'an room ID as default", form_name);
            xa_room_id.to_string() // Default to Xi'an room ID
        }
    }
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
    _dept_field_name: &str,
    form_submission: &FormSubmission,
    time_slot: &TimeSlot,
) -> Result<MeetingResult, StatusCode> {
    // Create meeting request with the operator_id from the client
    let meeting_request = CreateMeetingRequest {
        userid: client.get_operator_id().to_string(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        hosts: Some(vec![User {
            userid: client.get_operator_id().to_string(),
            is_anonymous: None,
            nick_name: None,
        }]),
        invitees: None,
        start_time: time_slot.start_time.timestamp().to_string(),
        end_time: time_slot.end_time.timestamp().to_string(),
        password: None,
        settings: Some(MeetingSettings {
            mute_enable_join: Some(true),
            mute_enable_type_join: Some(2),
            allow_unmute_self: Some(true),
            allow_in_before_host: Some(true),
            auto_in_waiting_room: None,
            allow_screen_shared_watermark: None,
            water_mark_type: None,
            only_enterprise_user_allowed: None,
            only_user_join_type: Some(1),
            auto_record_type: None,
            participant_join_auto_record: None,
            enable_host_pause_auto_record: None,
            allow_multi_device: Some(true),
            change_nickname: None,
            play_ivr_on_leave: None,
            play_ivr_on_join: None,
        }),
        location: Some(get_location_for_form(&form_submission.form_name, time_slot.item_name.as_str())),
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
        "Creating meeting for room: {} with time range: {}-{}",
        time_slot.item_name, time_slot.start_time, time_slot.end_time
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
    _dept_field_name: &str,
    form_submission: &FormSubmission,
    time_slots: &[TimeSlot],
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

    // Log merged slot details
    info!(
        "Creating merged time slot for room: {}, slots: {}, time range: {}-{}",
        room_name,
        time_slots.len(),
        start_time,
        end_time
    );

    // Create meeting request with merged time
    let meeting_request = CreateMeetingRequest {
        userid: client.get_operator_id().to_string(),
        instanceid: 32,
        subject: form_submission.entry.field_8.clone(),
        type_: 0, // Scheduled meeting
        _type: 0,
        hosts: Some(vec![User {
            userid: client.get_operator_id().to_string(),
            is_anonymous: None,
            nick_name: None,
        }]),
        invitees: None,
        start_time: start_time.timestamp().to_string(),
        end_time: end_time.timestamp().to_string(),
        password: None,
        settings: Some(MeetingSettings {
            mute_enable_join: Some(true),
            mute_enable_type_join: Some(2),
            allow_unmute_self: Some(true),
            allow_in_before_host: Some(true),
            auto_in_waiting_room: None,
            allow_screen_shared_watermark: None,
            water_mark_type: None,
            only_enterprise_user_allowed: None,
            only_user_join_type: Some(1),
            auto_record_type: None,
            participant_join_auto_record: None,
            enable_host_pause_auto_record: None,
            allow_multi_device: Some(true),
            change_nickname: None,
            play_ivr_on_leave: None,
            play_ivr_on_join: None,
        }),
        location: Some(get_location_for_form(&form_submission.form_name, room_name.as_str())),
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
