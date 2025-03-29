use axum::{
    extract::{Path, Query, State, Json as ExtractJson},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{info, error};

use crate::client::{
    TencentMeetingClient,
    CreateMeetingRequest,
    CreateMeetingResponse,
    CancelMeetingRequest,
};
use crate::models::common::PaginationParams;
use crate::models::form::FormSubmission;
use crate::models::meeting::{WebhookResponse};
use crate::services::time_slots::{
    parse_time_slot,
    find_mergeable_groups,
    create_meeting_with_time_slot,
    create_merged_meeting,
};

// AppState struct containing shared resources
pub struct AppState {
    pub client: TencentMeetingClient,
    #[allow(dead_code)]
    pub user_field_name: String,  // Reserved for future use
    pub dept_field_name: String,
}

// List meeting rooms endpoint
pub async fn list_meeting_rooms(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<crate::client::MeetingRoomsResponse>, StatusCode> {
    info!("Received request to list meeting rooms with page={}, page_size={}", 
          params.page, params.page_size);
    
    match state.client.list_rooms(params.page, params.page_size).await {
        Ok(response) => {
            info!("Successfully retrieved {} meeting rooms", response.meeting_room_list.len());
            Ok(Json(response))
        }
        Err(err) => {
            error!("Failed to retrieve meeting rooms: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Create meeting endpoint
pub async fn create_meeting(
    State(state): State<Arc<AppState>>,
    ExtractJson(meeting_request): ExtractJson<CreateMeetingRequest>,
) -> Result<Json<CreateMeetingResponse>, StatusCode> {
    info!("Received request to create meeting: {}", meeting_request.subject);
    
    match state.client.create_meeting(&meeting_request).await {
        Ok(response) => {
            info!("Successfully created {} meetings", response.meeting_info_list.len());
            Ok(Json(response))
        }
        Err(err) => {
            error!("Failed to create meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Cancel meeting endpoint
pub async fn cancel_meeting(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(cancel_request): ExtractJson<CancelMeetingRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to cancel meeting: {}", meeting_id);
    
    match state.client.cancel_meeting(&meeting_id, &cancel_request).await {
        Ok(_) => {
            info!("Successfully cancelled meeting {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(err) => {
            error!("Failed to cancel meeting: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Form submission webhook handler
pub async fn handle_form_submission(
    State(state): State<Arc<AppState>>,
    ExtractJson(form_submission): ExtractJson<FormSubmission>,
) -> Result<Json<WebhookResponse>, StatusCode> {
    info!("Received form submission for form: {} ({})", form_submission.form_name, form_submission.form);
    
    // Check if we have at least one scheduled item
    if form_submission.entry.field_1.is_empty() {
        error!("Form submission has no scheduled items");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Parse all time slots
    let mut time_slots = Vec::new();
    for reservation in &form_submission.entry.field_1 {
        match parse_time_slot(reservation) {
            Ok(slot) => time_slots.push(slot),
            Err(e) => {
                error!("Failed to parse time slot from reservation: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    
    info!("Parsed {} time slots from form submission", time_slots.len());
    
    // Try to find mergeable groups
    let mergeable_groups = find_mergeable_groups(&time_slots);
    
    // Results storage
    let mut meeting_results = Vec::new();
    let mut all_successful = true;
    
    // If there's only one group and it includes all slots, we can fully merge
    if mergeable_groups.len() == 1 && mergeable_groups[0].len() == time_slots.len() {
        info!("All time slots can be merged into a single meeting");
        let result = create_merged_meeting(
            &state.client,
            &state.user_field_name,
            &state.dept_field_name,
            &form_submission, 
            &time_slots
        ).await?;
        all_successful = all_successful && result.success;
        meeting_results.push(result);
    } else {
        // Process each mergeable group
        info!("Found {} mergeable groups", mergeable_groups.len());
        
        for (i, group) in mergeable_groups.iter().enumerate() {
            if group.len() > 1 {
                // Create a merged meeting for this group
                info!("Creating merged meeting for {} slots in group {}", group.len(), i+1);
                match create_merged_meeting(
                    &state.client,
                    &state.user_field_name,
                    &state.dept_field_name,
                    &form_submission, 
                    group
                ).await {
                    Ok(result) => {
                        all_successful = all_successful && result.success;
                        meeting_results.push(result);
                    },
                    Err(_) => {
                        all_successful = false;
                        // Continue processing other groups even if one fails
                    }
                }
            } else if group.len() == 1 {
                // Create a single meeting for this slot
                info!("Creating single meeting for time slot in group {}", i+1);
                match create_meeting_with_time_slot(
                    &state.client,
                    &state.user_field_name,
                    &state.dept_field_name,
                    &form_submission, 
                    &group[0]
                ).await {
                    Ok(result) => {
                        all_successful = all_successful && result.success;
                        meeting_results.push(result);
                    },
                    Err(_) => {
                        all_successful = false;
                        // Continue processing other groups even if one fails
                    }
                }
            }
        }
    }
    
    // Generate summary message
    let successful_count = meeting_results.iter()
        .filter(|r| r.meeting_id.is_some())
        .count();
    
    let merged_count = meeting_results.iter()
        .filter(|r| r.merged)
        .count();
    
    let message = if merged_count > 0 {
        format!(
            "Created {} meetings ({} merged) from {} time slots", 
            successful_count, 
            merged_count, 
            time_slots.len()
        )
    } else {
        format!(
            "Created {} meetings from {} time slots", 
            successful_count, 
            time_slots.len()
        )
    };
    
    // Return complete response with all meeting results
    Ok(Json(WebhookResponse {
        success: all_successful && successful_count > 0,
        message,
        meetings_count: meeting_results.len(),
        meetings: meeting_results,
    }))
}