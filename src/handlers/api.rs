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
use crate::models::meeting::{MeetingResult, WebhookResponse};
use crate::services::database::DatabaseService;
use crate::services::time_slots::{
    create_meeting_with_time_slot, create_merged_meeting, find_mergeable_groups, parse_time_slot,
    get_room_id_for_form, get_operator_info,
};

// AppState struct containing shared resources
pub struct AppState {
    pub client: TencentMeetingClient,
    pub user_field_name: String, // Used to identify the operator
    pub dept_field_name: String,
    pub database: Arc<DatabaseService>,
    pub xa_room_id: String,     // Xi'an meeting room ID
    pub cd_room_id: String,     // Chengdu meeting room ID
    pub skip_meeting_creation: bool, // Toggle to only store in CSV without creating meetings
    pub skip_room_booking: bool,     // Toggle to create meetings but not book rooms
}

// List meeting rooms endpoint
#[axum::debug_handler]
pub async fn list_meeting_rooms(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<crate::client::MeetingRoomsResponse>, StatusCode> {
    info!(
        "Received request to list meeting rooms with page={}, page_size={}",
        params.page, params.page_size
    );

    // Fetch meeting rooms from the Tencent Meeting API
    match state
        .client
        .list_rooms(params.page, params.page_size)
        .await
    {
        Ok(response) => {
            info!(
                "Successfully retrieved {} meeting rooms",
                response.meeting_room_list.len()
            );

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to retrieve meeting rooms: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Create a new meeting endpoint
#[axum::debug_handler]
pub async fn create_meeting(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<CreateMeetingRequest>,
) -> Result<Json<CreateMeetingResponse>, StatusCode> {
    info!(
        "Received request to create new meeting: {}",
        request.subject
    );

    // Call the Tencent Meeting API to create the meeting
    match state.client.create_meeting(&request).await {
        Ok(response) => {
            info!("Successfully created {} meetings", response.meeting_number);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to create meeting: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Cancel an existing meeting endpoint
#[axum::debug_handler]
pub async fn cancel_meeting(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(request): ExtractJson<CancelMeetingRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to cancel meeting: {}", meeting_id);

    // Call the Tencent Meeting API to cancel the meeting
    match state.client.cancel_meeting(&meeting_id, &request).await {
        Ok(_) => {
            info!("Successfully cancelled meeting: {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to cancel meeting: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Book meeting rooms for a meeting
#[axum::debug_handler]
pub async fn book_rooms(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(request): ExtractJson<BookRoomsRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Received request to book rooms for meeting: {}", meeting_id);

    // Call the Tencent Meeting API to book rooms
    match state.client.book_rooms(&meeting_id, &request).await {
        Ok(_) => {
            info!("Successfully booked rooms for meeting: {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to book rooms: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Release meeting rooms for a meeting
#[axum::debug_handler]
pub async fn release_rooms(
    State(state): State<Arc<AppState>>,
    Path(meeting_id): Path<String>,
    ExtractJson(request): ExtractJson<ReleaseRoomsRequest>,
) -> Result<StatusCode, StatusCode> {
    info!(
        "Received request to release rooms for meeting: {}",
        meeting_id
    );

    // Call the Tencent Meeting API to release rooms
    match state.client.release_rooms(&meeting_id, &request).await {
        Ok(_) => {
            info!("Successfully released rooms for meeting: {}", meeting_id);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            error!("Failed to release rooms: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Form webhook endpoint for meeting creation
#[axum::debug_handler]
pub async fn handle_form_submission(
    State(state): State<Arc<AppState>>,
    ExtractJson(form_submission): ExtractJson<FormSubmission>,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Get the appropriate room ID for this form
    let form_specific_room_id = get_room_id(&state, &form_submission);
    // Check if this is a cancellation request
    if form_submission
        .entry
        .reservation_status_fsf_field
        .to_lowercase()
        .contains("取消")
    {
        info!(
            "Form submission with token {} is a cancellation request",
            form_submission.entry.token
        );

        // Look up meeting IDs and room IDs in database
        match state.database.cancel_meeting(&form_submission.entry.token) {
            Ok(cancelled_meetings) if !cancelled_meetings.is_empty() => {
                info!(
                    "Found {} meetings to cancel with token: {}",
                    cancelled_meetings.len(), form_submission.entry.token
                );

                // Check if we're in simulation mode
                if state.skip_meeting_creation || cancelled_meetings.iter().any(|(id, _)| id.starts_with("simulation-")) {
                    let meeting_ids: Vec<String> = cancelled_meetings.iter().map(|(id, _)| id.clone()).collect();
                    info!("Simulation mode: {} meetings marked as cancelled in database: {:?}", 
                        cancelled_meetings.len(), meeting_ids);
                    return Ok(Json(WebhookResponse {
                        success: true,
                        message: format!("Simulation: {} meetings cancelled successfully", 
                            cancelled_meetings.len()),
                        meetings_count: 0,
                        meetings: Vec::new(),
                    }));
                }

                // Track cancellation results
                let mut successful_cancellations = 0;
                let mut failed_cancellations = 0;

                // Process each meeting that needs to be cancelled
                for (meeting_id, room_id) in &cancelled_meetings {
                    // Step 1: Release the meeting room
                    let release_request = ReleaseRoomsRequest {
                        operator_id: state.client.get_operator_id().to_string(),
                        operator_id_type: 1,
                        meeting_room_id_list: vec![room_id.clone()],
                    };

                    // Call the Tencent Meeting API to release the room
                    match state
                        .client
                        .release_rooms(meeting_id, &release_request)
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "Successfully released room {} for meeting {}",
                                room_id, meeting_id
                            );

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
                            match state
                                .client
                                .cancel_meeting(meeting_id, &cancel_request)
                                .await
                            {
                                Ok(_) => {
                                    info!("Successfully cancelled meeting with ID: {}", meeting_id);
                                    successful_cancellations += 1;
                                }
                                Err(err) => {
                                    error!("Failed to cancel meeting {}: {}", meeting_id, err);
                                    failed_cancellations += 1;
                                }
                            }
                        }
                        Err(err) => {
                            error!("Failed to release room {} for meeting {}: {}", room_id, meeting_id, err);
                            failed_cancellations += 1;
                        }
                    }
                }
                
                // Return summary of all cancellations
                if failed_cancellations == 0 {
                    info!("Successfully cancelled all {} meetings", successful_cancellations);
                    return Ok(Json(WebhookResponse {
                        success: true,
                        message: format!(
                            "Successfully cancelled {} meetings",
                            successful_cancellations
                        ),
                        meetings_count: 0,
                        meetings: Vec::new(),
                    }));
                } else {
                    warn!("Cancelled {} meetings, but {} failed", successful_cancellations, failed_cancellations);
                    return Ok(Json(WebhookResponse {
                        success: successful_cancellations > 0,
                        message: format!(
                            "Cancelled {} meetings, but {} failed",
                            successful_cancellations, failed_cancellations
                        ),
                        meetings_count: 0,
                        meetings: Vec::new(),
                    }));
                }
            }
            Ok(_) => {
                warn!(
                    "No active meetings found with token: {}",
                    form_submission.entry.token
                );
                return Ok(Json(WebhookResponse {
                    success: false,
                    message: format!(
                        "No active meetings found with token: {}",
                        form_submission.entry.token
                    ),
                    meetings_count: 0,
                    meetings: Vec::new(),
                }));
            }
            Err(e) => {
                error!("Failed to lookup meetings for cancellation: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // This is a reservation request, not a cancellation
    info!("Processing form submission for new meeting creation");

    // Extract time slots from the form submission
    let field1 = &form_submission.entry.field_1;

    info!("Form contains {} time slot entries", field1.len());

    // Parse all time slots from the form
    let mut time_slots = Vec::new();
    for (i, reservation) in field1.iter().enumerate() {
        info!(
            "Processing time slot {}: {}",
            i + 1,
            reservation.scheduled_label
        );

        // Parse the time slot
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

        // Check if we're in simulation mode (skip meeting creation)
        if state.skip_meeting_creation {
            // In simulation mode, store directly in database without creating a meeting
            info!(
                "Simulation mode: Storing form submission in database without creating a meeting"
            );

            // Create time slot labels
            let time_slot_labels: Vec<String> = time_slots
                .iter()
                .map(|slot| slot.scheduled_label.clone())
                .collect();

            // Create a simulated result
            let result = MeetingResult {
                meeting_id: Some("simulation-merged-meeting".to_string()),
                merged: true,
                room_name: time_slots[0].item_name.clone(),
                time_slots: time_slot_labels,
                success: true,
            };

            // Store directly in database with merged time slot info
            let room_id = get_room_id(&state, &form_submission);
            // Get operator information
            let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
            
            if let Err(e) = state.database.store_merged_meeting(
                &form_submission,
                "simulation-merged-meeting",
                &time_slots[0].item_name,
                &room_id,
                &time_slots,
                &operator_name,
                &operator_id,
            ) {
                error!("Failed to store simulated meeting record: {}", e);
            }

            meeting_results.push(result);
        } else {
            // Normal flow - create the actual merged meeting
            match create_merged_meeting(
                &state.client,
                &state.dept_field_name,
                &form_submission,
                &time_slots,
                &state.user_field_name,
            )
            .await
            {
                Ok(result) => {
                    all_successful = all_successful && result.success;

                    // Check if we're in simulation mode (skip meeting creation)
                    if state.skip_meeting_creation {
                        // In simulation mode, store directly in database without creating a meeting
                        info!("Simulation mode: Storing form submission in database without creating a meeting");
                        let room_id = get_room_id(&state, &form_submission);
                        // Get operator information
                        let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                        
                        if let Err(e) = state.database.store_meeting(
                            &form_submission,
                            "simulation-meeting-id",
                            &result.room_name,
                            &room_id,
                            &operator_name,
                            &operator_id,
                        ) {
                            error!("Failed to store simulated meeting record: {}", e);
                        }
                    }
                    // Store in database if we have a meeting ID
                    else if let Some(meeting_id) = &result.meeting_id {
                        // Check if we should book rooms
                        if !state.skip_room_booking {
                            // Get the appropriate room ID based on the form name
                            let room_id = get_room_id_for_form(
                                &form_submission.form_name, 
                                &state.xa_room_id, 
                                &state.cd_room_id
                            );
                            
                            // Book meeting room
                            let book_request = BookRoomsRequest {
                                operator_id: state.client.get_operator_id().to_string(),
                                operator_id_type: 1,
                                meeting_room_id_list: vec![room_id.clone()],
                                subject_visible: Some(true),
                            };

                            match state.client.book_rooms(meeting_id, &book_request).await {
                                Ok(_) => {
                                    info!(
                                        "Successfully booked room {} for meeting {}",
                                        form_specific_room_id, meeting_id
                                    );
                                }
                                Err(err) => {
                                    error!("Failed to book room for meeting: {}", err);
                                    // Continue with other operations, don't fail completely
                                }
                            }
                        } else {
                            info!(
                                "Room booking disabled: Skipping room booking for meeting {}",
                                meeting_id
                            );
                        }

                        // Store meeting info in database with room ID (whether or not room was booked)
                        let room_id = get_room_id(&state, &form_submission);
                        // Get operator information
                        let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                        
                        if let Err(e) = state.database.store_merged_meeting(
                            &form_submission,
                            meeting_id,
                            &result.room_name,
                            &room_id,
                            &time_slots,
                            &operator_name,
                            &operator_id,
                        ) {
                            error!("Failed to store meeting record: {}", e);
                            // Continue processing even if database storage fails
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

                    // Check if we're in simulation mode
                    if state.skip_meeting_creation {
                        info!(
                            "Simulation mode: Storing merged time slots without creating a meeting"
                        );

                        // Create simulated time slot labels
                        let time_slot_labels: Vec<String> = group
                            .iter()
                            .map(|slot| slot.scheduled_label.clone())
                            .collect();

                        // Create a simulated result
                        let result = MeetingResult {
                            meeting_id: Some(format!("simulation-merged-meeting-{}", i)),
                            merged: true,
                            room_name: group[0].item_name.clone(),
                            time_slots: time_slot_labels,
                            success: true,
                        };

                        // Store directly in database with merged time slot info
                        let room_id = get_room_id(&state, &form_submission);
                        // Get operator information
                        let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                        
                        if let Err(e) = state.database.store_merged_meeting(
                            &form_submission,
                            &format!("simulation-merged-meeting-{}", i),
                            &group[0].item_name,
                            &room_id,
                            group,
                            &operator_name,
                            &operator_id,
                        ) {
                            error!("Failed to store simulated merged meeting record: {}", e);
                        }

                        meeting_results.push(result);
                        all_successful = all_successful && true;
                    } else {
                        // Normal flow - create the merged meeting
                        match create_merged_meeting(
                            &state.client,
                            &state.dept_field_name,
                            &form_submission,
                            group,
                            &state.user_field_name,
                        )
                        .await
                        {
                            Ok(result) => {
                                all_successful = all_successful && result.success;

                                // Store in database if we have a meeting ID
                                if let Some(meeting_id) = &result.meeting_id {
                                    // Only book rooms if not skipped
                                    if !state.skip_room_booking {
                                        // Step 2: Book the meeting room
                                        // Get the appropriate room ID
                                        
                                        let book_request = BookRoomsRequest {
                                            operator_id: state.client.get_operator_id().to_string(),
                                            operator_id_type: 1,
                                            meeting_room_id_list: vec![form_specific_room_id.clone()],
                                            subject_visible: Some(true),
                                        };

                                        match state
                                            .client
                                            .book_rooms(meeting_id, &book_request)
                                            .await
                                        {
                                            Ok(_) => {
                                                info!(
                                                    "Successfully booked room {} for meeting {}",
                                                    form_specific_room_id, meeting_id
                                                );
                                            }
                                            Err(err) => {
                                                error!("Failed to book room for meeting: {}", err);
                                                // Continue with other operations, don't fail completely
                                            }
                                        }
                                    } else {
                                        info!("Room booking disabled: Skipping room booking for meeting {}", meeting_id);
                                    }

                                    // Always store meeting in database with merged time slot info
                                    // Get the appropriate room ID
                                    let room_id = get_room_id(&state, &form_submission);
                                    
                                    // Get operator information
                                    let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                                    
                                    if let Err(e) = state.database.store_merged_meeting(
                                        &form_submission,
                                        meeting_id,
                                        &result.room_name,
                                        &room_id,
                                        group,
                                        &operator_name,
                                        &operator_id,
                                    ) {
                                        error!("Failed to store meeting record: {}", e);
                                        // Continue processing even if database storage fails
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
                }
                std::cmp::Ordering::Equal => {
                    // Create a single meeting for this slot (exactly 1 slot)
                    info!("Creating single meeting for time slot in group {}", i + 1);
                    // Check if we're in simulation mode first
                    if state.skip_meeting_creation {
                        info!(
                            "Simulation mode: Storing single time slot without creating a meeting"
                        );

                        // Create a simulated result
                        let result = MeetingResult {
                            meeting_id: Some(format!("simulation-meeting-id-{}", i)),
                            merged: false,
                            room_name: group[0].item_name.clone(),
                            time_slots: vec![group[0].scheduled_label.clone()],
                            success: true,
                        };

                        // Store directly in database with specific time slot
                        let room_id = get_room_id(&state, &form_submission);
                        // Get operator information
                        let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                        
                        if let Err(e) = state.database.store_meeting_with_time_slot(
                            &form_submission,
                            &format!("simulation-meeting-id-{}", i),
                            &group[0].item_name,
                            &room_id,
                            &group[0],
                            &operator_name,
                            &operator_id,
                        ) {
                            error!("Failed to store simulated meeting record: {}", e);
                        }

                        meeting_results.push(result);
                        all_successful = all_successful && true;
                    } else {
                        // Normal flow - create the meeting
                        match create_meeting_with_time_slot(
                            &state.client,
                            &state.dept_field_name,
                            &form_submission,
                            &group[0],
                            &state.user_field_name,
                        )
                        .await
                        {
                            Ok(result) => {
                                all_successful = all_successful && result.success;

                                // Store in database if we have a meeting ID
                                if let Some(meeting_id) = &result.meeting_id {
                                    // Only book rooms if not skipped
                                    if !state.skip_room_booking {
                                        // Step 2: Book the meeting room
                                        // Get the appropriate room ID
                                        
                                        let book_request = BookRoomsRequest {
                                            operator_id: state.client.get_operator_id().to_string(),
                                            operator_id_type: 1,
                                            meeting_room_id_list: vec![form_specific_room_id.clone()],
                                            subject_visible: Some(true),
                                        };

                                        match state
                                            .client
                                            .book_rooms(meeting_id, &book_request)
                                            .await
                                        {
                                            Ok(_) => {
                                                info!(
                                                    "Successfully booked room {} for meeting {}",
                                                    form_specific_room_id, meeting_id
                                                );
                                            }
                                            Err(err) => {
                                                error!("Failed to book room for meeting: {}", err);
                                                // Continue with other operations, don't fail completely
                                            }
                                        }
                                    } else {
                                        info!("Room booking disabled: Skipping room booking for meeting {}", meeting_id);
                                    }

                                    // Always store in database with specific time slot
                                    let room_id = get_room_id(&state, &form_submission);
                                    
                                    // Get operator information
                                    let (operator_name, operator_id) = get_operator_info(&state.client, &form_submission, &state.user_field_name);
                                    
                                    if let Err(e) = state.database.store_meeting_with_time_slot(
                                        &form_submission,
                                        meeting_id,
                                        &result.room_name,
                                        &room_id,
                                        &group[0],
                                        &operator_name,
                                        &operator_id,
                                    ) {
                                        error!("Failed to store meeting record: {}", e);
                                        // Continue processing even if database storage fails
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

// Helper function to get the room ID to use for a form submission
fn get_room_id(state: &AppState, form_submission: &FormSubmission) -> String {
    get_room_id_for_form(
        &form_submission.form_name,
        &state.xa_room_id,
        &state.cd_room_id
    )
}