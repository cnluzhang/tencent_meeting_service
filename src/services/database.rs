use chrono::Utc;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

use crate::models::form::FormSubmission;
use crate::models::meeting::TimeSlot;
use crate::services::time_slots::parse_time_slot;

// Record to be stored in CSV
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingRecord {
    // Form data
    pub entry_token: String,
    pub form_id: String,
    pub form_name: String,
    pub subject: String,
    pub room_name: String,
    pub scheduled_at: String, // ISO format
    pub scheduled_label: String,
    pub status: String, // "Reserved" or "Cancelled"

    // Tencent Meeting data
    pub meeting_id: String,
    pub room_id: String,      // Room ID used for booking
    pub created_at: String,   // ISO format
    pub cancelled_at: String, // ISO format (empty if not cancelled)

    // Operator information
    pub operator_name: String, // Name of the operator from form submission
    pub operator_id: String,   // ID of the operator used for API calls
}

// Database service for storing form submissions and meeting data
pub struct DatabaseService {
    csv_path: String,
    file_mutex: Mutex<()>,
}

impl DatabaseService {
    pub fn new(csv_path: &str) -> Self {
        // Create the CSV file if it doesn't exist with proper headers
        if !Path::new(csv_path).exists() {
            info!("Creating new meetings database file at {}", csv_path);

            let file = File::create(csv_path).unwrap_or_else(|e| {
                error!("Failed to create database file: {}", e);
                panic!("Failed to create database file: {}", e)
            });

            let mut writer = WriterBuilder::new().has_headers(true).from_writer(file);

            if let Err(e) = writer.write_record([
                "entry_token",
                "form_id",
                "form_name",
                "subject",
                "room_name",
                "scheduled_at",
                "scheduled_label",
                "status",
                "meeting_id",
                "room_id",
                "created_at",
                "cancelled_at",
                "operator_name",
                "operator_id",
            ]) {
                error!("Failed to write headers: {}", e);
                panic!("Failed to write headers: {}", e);
            }

            if let Err(e) = writer.flush() {
                error!("Failed to flush headers: {}", e);
                panic!("Failed to flush headers: {}", e);
            }
        }

        Self {
            csv_path: csv_path.to_string(),
            file_mutex: Mutex::new(()),
        }
    }

    /// Store a meeting record using a specific time slot
    ///
    /// This function creates a database record for a single time slot meeting.
    /// It includes time-specific information from the provided TimeSlot,
    /// which ensures that each meeting has its correct scheduled time.
    ///
    /// Used for individual (non-merged) meetings and distinguishes between
    /// multiple meetings with the same form token but different time slots.
    pub fn store_meeting_with_time_slot(
        &self,
        form: &FormSubmission,
        meeting_id: &str,
        room_name: &str,
        room_id: &str,
        time_slot: &TimeSlot,
        operator_name: &str,
        operator_id: &str,
    ) -> Result<(), String> {
        // Use the specific time slot's label
        let scheduled_label = time_slot.scheduled_label.clone();

        // Check if an identical meeting entry already exists (same token, status, and time)
        let is_duplicate = self
            .find_all_meetings_by_token(&form.entry.token)?
            .into_iter()
            .any(|record| {
                record.status == form.entry.reservation_status_fsf_field
                    && record.scheduled_label == scheduled_label
            });

        if is_duplicate {
            // Entry with same token, status, and time already exists
            info!("Meeting with token {} and status {} for time {} already exists, skipping insertion", 
                form.entry.token, form.entry.reservation_status_fsf_field, scheduled_label);
            return Ok(());
        }

        // Get current time in UTC
        let now = Utc::now();

        // Create a new record
        let record = MeetingRecord {
            entry_token: form.entry.token.clone(),
            form_id: form.form.clone(),
            form_name: form.form_name.clone(),
            subject: form.entry.field_8.clone(),
            room_name: room_name.to_string(),
            scheduled_at: time_slot.start_time.to_rfc3339(),
            scheduled_label,
            status: form.entry.reservation_status_fsf_field.clone(),
            meeting_id: meeting_id.to_string(),
            room_id: room_id.to_string(),
            created_at: now.to_rfc3339(),
            cancelled_at: "".to_string(),
            operator_name: operator_name.to_string(),
            operator_id: operator_id.to_string(),
        };

        self.write_record(&record)
    }

    // Store a meeting record (keep for backward compatibility)
    pub fn store_meeting(
        &self,
        form: &FormSubmission,
        meeting_id: &str,
        room_name: &str,
        room_id: &str,
        operator_name: &str,
        operator_id: &str,
    ) -> Result<(), String> {
        // Get the first available time slot from the form
        if let Some(first_slot) = form.entry.field_1.first() {
            // Create a TimeSlot from the form field
            let parsed_slot = parse_time_slot(first_slot)
                .map_err(|e| format!("Failed to parse time slot: {}", e))?;

            // Call the new method with the parsed slot
            self.store_meeting_with_time_slot(
                form,
                meeting_id,
                room_name,
                room_id,
                &parsed_slot,
                operator_name,
                operator_id,
            )
        } else {
            // Fallback to default behavior if no time slots available
            // Get current time in UTC
            let now = Utc::now();

            // Create a new record
            let record = MeetingRecord {
                entry_token: form.entry.token.clone(),
                form_id: form.form.clone(),
                form_name: form.form_name.clone(),
                subject: form.entry.field_8.clone(),
                room_name: room_name.to_string(),
                scheduled_at: now.to_rfc3339(),
                scheduled_label: "No time specified".to_string(),
                status: form.entry.reservation_status_fsf_field.clone(),
                meeting_id: meeting_id.to_string(),
                room_id: room_id.to_string(),
                created_at: now.to_rfc3339(),
                cancelled_at: "".to_string(),
                operator_name: operator_name.to_string(),
                operator_id: operator_id.to_string(),
            };

            self.write_record(&record)
        }
    }

    /// Store a merged meeting record in the database with custom time slot info
    ///
    /// This function creates a database record for a merged meeting composed of multiple
    /// contiguous time slots. It combines multiple time slots into a single meeting with:
    /// - The start time of the first slot
    /// - A combined scheduled label showing the full time range (e.g., "09:00-11:00")
    ///
    /// This is used when multiple adjacent time slots for the same room can be
    /// merged into a single, longer meeting.
    pub fn store_merged_meeting(
        &self,
        form: &FormSubmission,
        meeting_id: &str,
        room_name: &str,
        room_id: &str,
        time_slots: &[TimeSlot],
        operator_name: &str,
        operator_id: &str,
    ) -> Result<(), String> {
        // Sort time slots to ensure correct ordering
        let mut sorted_slots = time_slots.to_vec();
        sorted_slots.sort_by_key(|slot| slot.start_time);

        // Get the earliest start time and latest end time
        let first_slot = sorted_slots.first().unwrap();
        let last_slot = sorted_slots.last().unwrap();

        // Create combined scheduled_label (e.g., "2025-04-01 09:00-11:00")
        let first_time = first_slot
            .scheduled_label
            .split(' ')
            .nth(1)
            .unwrap_or("")
            .split('-')
            .next()
            .unwrap_or("");
        let last_time = last_slot
            .scheduled_label
            .split(' ')
            .nth(1)
            .unwrap_or("")
            .split('-')
            .nth(1)
            .unwrap_or("");
        let date = first_slot.scheduled_label.split(' ').next().unwrap_or("");
        let combined_label = format!("{} {}-{}", date, first_time, last_time);

        // Check if an identical meeting entry already exists (same token, status, and time)
        let is_duplicate = self
            .find_all_meetings_by_token(&form.entry.token)?
            .into_iter()
            .any(|record| {
                record.status == form.entry.reservation_status_fsf_field
                    && record.scheduled_label == combined_label
            });

        if is_duplicate {
            // Entry with same token, status, and time already exists
            info!("Meeting with token {} and status {} for time {} already exists, skipping insertion", 
                form.entry.token, form.entry.reservation_status_fsf_field, combined_label);
            return Ok(());
        }

        // Get current time in UTC
        let now = Utc::now();

        info!(
            "Creating merged meeting record with label: {}",
            combined_label
        );

        // Create a new record
        let record = MeetingRecord {
            entry_token: form.entry.token.clone(),
            form_id: form.form.clone(),
            form_name: form.form_name.clone(),
            subject: form.entry.field_8.clone(),
            room_name: room_name.to_string(),
            scheduled_at: first_slot.start_time.to_rfc3339(),
            scheduled_label: combined_label,
            status: form.entry.reservation_status_fsf_field.clone(),
            meeting_id: meeting_id.to_string(),
            room_id: room_id.to_string(),
            created_at: now.to_rfc3339(),
            cancelled_at: "".to_string(),
            operator_name: operator_name.to_string(),
            operator_id: operator_id.to_string(),
        };

        self.write_record(&record)
    }

    /// Update meeting status to cancelled for all meetings with the given token
    ///
    /// This function finds and cancels all meetings that match the provided token.
    /// It supports canceling multiple meetings at once (like when a user books
    /// multiple time slots with the same form submission).
    ///
    /// Returns a Vec of (meeting_id, room_id) pairs for all cancelled meetings,
    /// which allows the caller to handle multiple cancellations appropriately.
    pub fn cancel_meeting(&self, entry_token: &str) -> Result<Vec<(String, String)>, String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        // Load all records
        let file = File::open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file: {}", e))?;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);

        let headers = reader
            .headers()
            .map_err(|e| format!("Failed to read headers: {}", e))?
            .clone();

        let mut records: Vec<StringRecord> = Vec::new();
        let mut cancelled_meetings = Vec::new();

        // Find the record with matching token and copy all records
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;

            // Check if this is the record to update - not yet cancelled
            let is_reserved = record.get(7) == Some("Reserved") || record.get(7) == Some("已预约");
            let is_cancelled =
                record.get(7) == Some("Cancelled") || record.get(7) == Some("已取消");
            if record.get(0) == Some(entry_token) && is_reserved && !is_cancelled {
                // Get meeting_id and room_id for cancellation
                if let (Some(meeting_id), Some(room_id)) = (record.get(8), record.get(9)) {
                    // Add to the list of cancelled meetings
                    cancelled_meetings.push((meeting_id.to_string(), room_id.to_string()));

                    let now = Utc::now().to_rfc3339();

                    // Build a new record with updated status and cancelled_at time
                    let mut updated_vec: Vec<String> = record.iter().map(String::from).collect();
                    updated_vec[7] = "已取消".to_string(); // Update status to Chinese "Cancelled"
                    updated_vec[11] = now.clone(); // Update cancelled_at

                    let updated = StringRecord::from(updated_vec);

                    info!(
                        "Marked meeting {} as cancelled for token {}",
                        meeting_id, entry_token
                    );

                    records.push(updated);
                } else {
                    // Record is missing meeting_id or room_id, push it unchanged
                    records.push(record);
                }
            } else {
                records.push(record);
            }
        }

        // If no matching record found
        if cancelled_meetings.is_empty() {
            warn!("No active meetings found for token: {}", entry_token);
            return Ok(Vec::new());
        }

        // Write all records back (overwrite the file)
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file for writing: {}", e))?;

        let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);

        // Write headers
        writer
            .write_record(&headers)
            .map_err(|e| format!("Failed to write headers: {}", e))?;

        // Write all records
        for record in records {
            writer
                .write_record(&record)
                .map_err(|e| format!("Failed to write record: {}", e))?;
        }

        writer
            .flush()
            .map_err(|e| format!("Failed to flush writer: {}", e))?;

        info!(
            "Cancelled {} meetings with token {}",
            cancelled_meetings.len(),
            entry_token
        );

        // Return all cancelled meeting IDs and room IDs
        Ok(cancelled_meetings)
    }

    // Find a meeting by entry token (active/not cancelled)
    pub fn find_meeting_by_token(
        &self,
        entry_token: &str,
    ) -> Result<Option<MeetingRecord>, String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        let file = File::open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file: {}", e))?;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);

        // Find the record with matching token
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;

            // For finding a meeting, we check if it's either reserved or not cancelled
            let is_cancelled =
                record.get(7) == Some("Cancelled") || record.get(7) == Some("已取消");
            if record.get(0) == Some(entry_token) && !is_cancelled {
                // Convert to MeetingRecord
                return Ok(Some(self.string_record_to_meeting_record(&record)?));
            }
        }

        // No matching record found
        Ok(None)
    }

    // Find a meeting by entry token and specific status
    pub fn find_meeting_by_token_and_status(
        &self,
        entry_token: &str,
        status: &str,
    ) -> Result<Option<MeetingRecord>, String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        // If file doesn't exist, return None early
        if !Path::new(&self.csv_path).exists() {
            return Ok(None);
        }

        // Open file with better error handling
        let file = File::open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file: {}", e))?;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);

        // Find the record with matching token and status
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;

            // Use Option combinators to check if we have a match
            let token_matches = record.get(0).map_or(false, |val| val == entry_token);
            let status_matches = record.get(7).map_or(false, |val| val == status);

            if token_matches && status_matches {
                // Convert to MeetingRecord and return early
                return self.string_record_to_meeting_record(&record).map(Some);
            }
        }

        // No matching record found
        Ok(None)
    }

    /// Find all meetings with a specific token
    ///
    /// Unlike find_meeting_by_token which returns only one meeting,
    /// this function returns all meetings that match the given token.
    /// This is useful for operations that need to process multiple
    /// meetings associated with the same form submission, such as
    /// deduplication checks and batch cancellations.
    pub fn find_all_meetings_by_token(
        &self,
        entry_token: &str,
    ) -> Result<Vec<MeetingRecord>, String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        // If file doesn't exist yet, return empty vector
        if !Path::new(&self.csv_path).exists() {
            return Ok(Vec::new());
        }

        let file = match File::open(&self.csv_path) {
            Ok(file) => file,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(Vec::new());
                }
                return Err(format!("Failed to open database file: {}", e));
            }
        };

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);
        let mut meetings = Vec::new();

        // Find all records with matching token
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;

            if record.get(0) == Some(entry_token) {
                // Convert to MeetingRecord and add to results
                meetings.push(self.string_record_to_meeting_record(&record)?);
            }
        }

        Ok(meetings)
    }

    // Convert StringRecord to MeetingRecord
    fn string_record_to_meeting_record(
        &self,
        record: &StringRecord,
    ) -> Result<MeetingRecord, String> {
        // Helper function to get field with better error context
        let get_field = |idx: usize, name: &str| -> Result<String, String> {
            Ok(record.get(idx).map(|s| s.to_string()).unwrap_or_else(|| {
                // For logging only - not a fatal error
                if idx < record.len() {
                    warn!("Field {} at index {} is empty", name, idx);
                }
                String::new()
            }))
        };

        // Ensure record has at least the required fields
        if record.len() < 14 {
            return Err(format!(
                "Invalid record length: {}. Expected at least 14 fields.",
                record.len()
            ));
        }

        // Create record with operator fields
        Ok(MeetingRecord {
            entry_token: get_field(0, "entry_token")?,
            form_id: get_field(1, "form_id")?,
            form_name: get_field(2, "form_name")?,
            subject: get_field(3, "subject")?,
            room_name: get_field(4, "room_name")?,
            scheduled_at: get_field(5, "scheduled_at")?,
            scheduled_label: get_field(6, "scheduled_label")?,
            status: get_field(7, "status")?,
            meeting_id: get_field(8, "meeting_id")?,
            room_id: get_field(9, "room_id")?,
            created_at: get_field(10, "created_at")?,
            cancelled_at: get_field(11, "cancelled_at")?,
            operator_name: get_field(12, "operator_name")?,
            operator_id: get_field(13, "operator_id")?,
        })
    }

    // Helper to write a record to the CSV
    fn write_record(&self, record: &MeetingRecord) -> Result<(), String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        let file = OpenOptions::new()
            .append(true)
            .open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file: {}", e))?;

        let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);

        writer
            .serialize(record)
            .map_err(|e| format!("Failed to serialize record: {}", e))?;

        writer
            .flush()
            .map_err(|e| format!("Failed to flush writer: {}", e))?;

        info!(
            "Stored meeting record for token {} with ID {}",
            record.entry_token, record.meeting_id
        );

        Ok(())
    }
}

// Create a singleton database service
pub fn create_database_service() -> Arc<DatabaseService> {
    // Default path with environment variable override
    let default_path = "/app/data/meetings.csv";
    let csv_path =
        std::env::var("MEETING_DATABASE_PATH").unwrap_or_else(|_| default_path.to_string());

    // Create the data directory if it doesn't exist and we're using the default path
    if csv_path == default_path {
        let dir = std::path::Path::new(default_path).parent().unwrap();
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::error!("Failed to create data directory: {}", e);
            panic!("Failed to create data directory: {}", e);
        }
    }

    Arc::new(DatabaseService::new(&csv_path))
}
