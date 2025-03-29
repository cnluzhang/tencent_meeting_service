use axum::{
    extract::{Json as ExtractJson, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::client::{
    BookRoomsRequest, CancelMeetingRequest, CreateMeetingRequest, CreateMeetingResponse, 
    ReleaseRoomsRequest, TencentMeetingClient,
};
use crate::models::common::PaginationParams;
use crate::models::form::FormSubmission;
use crate::models::meeting::WebhookResponse;
use crate::services::database::DatabaseService;
use crate::services::time_slots::{
    create_meeting_with_time_slot, create_merged_meeting, find_mergeable_groups, parse_time_slot,
};

// AppState struct containing shared resources
pub struct AppState {
    pub client: TencentMeetingClient,
    #[allow(dead_code)]
    pub user_field_name: String, // Reserved for future use
    pub dept_field_name: String,
    pub database: Arc<DatabaseService>,
    pub default_room_id: String,
}

// List meeting rooms endpoint
pub async fn list_meeting_rooms(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<crate::client::MeetingRoomsResponse>, StatusCode> {
    info!(
        "Received request to list meeting rooms with page={}, page_size={}",
        params.page, params.page_size
    );

    match state.client.list_rooms(params.page, params.page_size).await {
        Ok(response) => {
            info!(
                "Successfully retrieved {} meeting rooms",
                response.meeting_room_list.len()
            );
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
    info!(
        "Received request to create meeting: {}",
        meeting_request.subject
    );

    match state.client.create_meeting(&meeting_request).await {
        Ok(response) => {
            info!(
                "Successfully created {} meetings",
                response.meeting_info_list.len()
            );
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

    match state
        .client
        .cancel_meeting(&meeting_id, &cancel_request)
        .await
    {
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

// Book rooms for a meeting endpoint
pub async fn book_rooms(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(book_request): ExtractJson<BookRoomsRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to book rooms for meeting: {}", meeting_id);
    info!("Room IDs to book: {:?}", book_request.meeting_room_id_list);

    match state.client.book_rooms(&meeting_id, &book_request).await {
        Ok(_) => {
            info!("Successfully booked rooms for meeting {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(err) => {
            error!("Failed to book rooms: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Release rooms for a meeting endpoint
pub async fn release_rooms(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(release_request): ExtractJson<ReleaseRoomsRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to release rooms for meeting: {}", meeting_id);
    info!("Room IDs to release: {:?}", release_request.meeting_room_id_list);

    match state.client.release_rooms(&meeting_id, &release_request).await {
        Ok(_) => {
            info!("Successfully released rooms for meeting {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(err) => {
            error!("Failed to release rooms: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Form submission webhook handler
pub async fn handle_form_submission(
    State(state): State<Arc<AppState>>,
    ExtractJson(form_submission): ExtractJson<FormSubmission>,
) -> Result<Json<WebhookResponse>, StatusCode> {
    info!(
        "Received form submission for form: {} ({})",
        form_submission.form_name, form_submission.form
    );

    // Check if the status indicates a cancellation
    if form_submission.entry.reservation_status_fsf_field == "已取消" {
        info!(
            "Processing cancellation request for entry token: {}",
            form_submission.entry.token
        );

        // Look up meeting ID and room ID in database 
        match state.database.cancel_meeting(&form_submission.entry.token) {
            Ok(Some((meeting_id, room_id))) => {
                info!("Found meeting to cancel with ID: {} and room ID: {}", meeting_id, room_id);

                // Step 1: Release the meeting room
                let release_request = ReleaseRoomsRequest {
                    operator_id: state.client.get_operator_id().to_string(),
                    operator_id_type: 1,
                    meeting_room_id_list: vec![room_id.clone()],
                };

                // Call the Tencent Meeting API to release the room
                match state.client.release_rooms(&meeting_id, &release_request).await {
                    Ok(_) => {
                        info!("Successfully released room {} for meeting {}", room_id, meeting_id);
                        
                        // Step 2: Cancel the meeting
                        let cancel_request = CancelMeetingRequest {
                            userid: state.client.get_operator_id().to_string(),
                            instanceid: 32,
                            reason_code: 1, // Cancellation reason code
                            meeting_type: None,
                            sub_meeting_id: None,
                            reason_detail: Some("Form submission cancelled".to_string()),
                        };

                        // Call the Tencent Meeting API to cancel the meeting
                        match state.client.cancel_meeting(&meeting_id, &cancel_request).await {
                            Ok(_) => {
                                info!("Successfully cancelled meeting with ID: {}", meeting_id);
                                return Ok(Json(WebhookResponse {
                                    success: true,
                                    message: format!("Meeting {} cancelled and room {} released successfully", 
                                        meeting_id, room_id),
                                    meetings_count: 0,
                                    meetings: Vec::new(),
                                }));
                            }
                            Err(err) => {
                                error!("Failed to cancel meeting: {}", err);
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed to release room: {}", err);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
            Ok(None) => {
                error!(
                    "No active meeting found for token: {}",
                    form_submission.entry.token
                );
                return Ok(Json(WebhookResponse {
                    success: false,
                    message: format!(
                        "No active meeting found for token: {}",
                        form_submission.entry.token
                    ),
                    meetings_count: 0,
                    meetings: Vec::new(),
                }));
            }
            Err(e) => {
                error!("Database error when looking up meeting to cancel: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // For regular meeting creation requests
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

    info!(
        "Parsed {} time slots from form submission",
        time_slots.len()
    );

    // Try to find mergeable groups
    let mergeable_groups = find_mergeable_groups(&time_slots);

    // Results storage
    let mut meeting_results = Vec::new();
    let mut all_successful = true;

    // If there's only one group and it includes all slots, we can fully merge
    if mergeable_groups.len() == 1 && mergeable_groups[0].len() == time_slots.len() {
        info!("All time slots can be merged into a single meeting");
        match create_merged_meeting(
            &state.client,
            &state.user_field_name,
            &state.dept_field_name,
            &form_submission,
            &time_slots,
        )
        .await
        {
            Ok(result) => {
                all_successful = all_successful && result.success;

                // Store in database if we have a meeting ID
                if let Some(meeting_id) = &result.meeting_id {
                    // Step 2: Book the meeting room
                    let book_request = BookRoomsRequest {
                        operator_id: state.client.get_operator_id().to_string(),
                        operator_id_type: 1,
                        meeting_room_id_list: vec![state.default_room_id.clone()],
                        subject_visible: Some(true),
                    };
                    
                    match state.client.book_rooms(meeting_id, &book_request).await {
                        Ok(_) => {
                            info!("Successfully booked room {} for meeting {}", 
                                state.default_room_id, meeting_id);
                            
                            // Store meeting info in database with room ID
                            if let Err(e) = state.database.store_meeting(
                                &form_submission,
                                meeting_id,
                                &result.room_name,
                                &state.default_room_id,
                            ) {
                                error!("Failed to store meeting record: {}", e);
                                // Continue processing even if database storage fails
                            }
                        }
                        Err(err) => {
                            error!("Failed to book room for meeting: {}", err);
                            // Continue with other operations, don't fail completely
                        }
                    }
                }

                meeting_results.push(result);
            }
            Err(e) => {
                error!("Failed to create merged meeting: {:?}", e);
                // No need to set all_successful since we're returning immediately
                return Err(e);
            }
        }
    } else {
        // Process each mergeable group
        info!("Found {} mergeable groups", mergeable_groups.len());

        for (i, group) in mergeable_groups.iter().enumerate() {
            match group.len().cmp(&1) {
                std::cmp::Ordering::Greater => {
                    // Create a merged meeting for this group (more than 1 slot)
                    info!(
                        "Creating merged meeting for {} slots in group {}",
                        group.len(),
                        i + 1
                    );
                    match create_merged_meeting(
                        &state.client,
                        &state.user_field_name,
                        &state.dept_field_name,
                        &form_submission,
                        group,
                    )
                    .await
                    {
                        Ok(result) => {
                            all_successful = all_successful && result.success;

                            // Store in database if we have a meeting ID
                            if let Some(meeting_id) = &result.meeting_id {
                                // Step 2: Book the meeting room
                                let book_request = BookRoomsRequest {
                                    operator_id: state.client.get_operator_id().to_string(),
                                    operator_id_type: 1,
                                    meeting_room_id_list: vec![state.default_room_id.clone()],
                                    subject_visible: Some(true),
                                };
                                
                                match state.client.book_rooms(meeting_id, &book_request).await {
                                    Ok(_) => {
                                        info!("Successfully booked room {} for meeting {}", 
                                            state.default_room_id, meeting_id);
                                        
                                        // Store meeting info in database with room ID
                                        if let Err(e) = state.database.store_meeting(
                                            &form_submission,
                                            meeting_id,
                                            &result.room_name,
                                            &state.default_room_id,
                                        ) {
                                            error!("Failed to store meeting record: {}", e);
                                            // Continue processing even if database storage fails
                                        }
                                    }
                                    Err(err) => {
                                        error!("Failed to book room for meeting: {}", err);
                                        // Continue with other operations, don't fail completely
                                    }
                                }
                            }

                            meeting_results.push(result);
                        }
                        Err(e) => {
                            error!(
                                "Failed to create merged meeting in group {}: {:?}",
                                i + 1,
                                e
                            );
                            all_successful = false;
                            // Continue processing other groups even if one fails
                        }
                    }
                }
                std::cmp::Ordering::Equal => {
                    // Create a single meeting for this slot (exactly 1 slot)
                    info!("Creating single meeting for time slot in group {}", i + 1);
                    match create_meeting_with_time_slot(
                        &state.client,
                        &state.user_field_name,
                        &state.dept_field_name,
                        &form_submission,
                        &group[0],
                    )
                    .await
                    {
                        Ok(result) => {
                            all_successful = all_successful && result.success;

                            // Store in database if we have a meeting ID
                            if let Some(meeting_id) = &result.meeting_id {
                                // Step 2: Book the meeting room
                                let book_request = BookRoomsRequest {
                                    operator_id: state.client.get_operator_id().to_string(),
                                    operator_id_type: 1,
                                    meeting_room_id_list: vec![state.default_room_id.clone()],
                                    subject_visible: Some(true),
                                };
                                
                                match state.client.book_rooms(meeting_id, &book_request).await {
                                    Ok(_) => {
                                        info!("Successfully booked room {} for meeting {}", 
                                            state.default_room_id, meeting_id);
                                        
                                        // Store meeting info in database with room ID
                                        if let Err(e) = state.database.store_meeting(
                                            &form_submission,
                                            meeting_id,
                                            &result.room_name,
                                            &state.default_room_id,
                                        ) {
                                            error!("Failed to store meeting record: {}", e);
                                            // Continue processing even if database storage fails
                                        }
                                    }
                                    Err(err) => {
                                        error!("Failed to book room for meeting: {}", err);
                                        // Continue with other operations, don't fail completely
                                    }
                                }
                            }

                            meeting_results.push(result);
                        }
                        Err(e) => {
                            error!(
                                "Failed to create single meeting in group {}: {:?}",
                                i + 1,
                                e
                            );
                            all_successful = false;
                            // Continue processing other groups even if one fails
                        }
                    }
                }
                std::cmp::Ordering::Less => {
                    // This case shouldn't happen as we should never have empty groups
                    warn!("Found empty meeting group at index {}", i);
                }
            }
        }
    }

    // Generate summary message
    let successful_count = meeting_results
        .iter()
        .filter(|r| r.meeting_id.is_some())
        .count();

    let merged_count = meeting_results.iter().filter(|r| r.merged).count();

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
