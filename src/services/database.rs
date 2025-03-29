use chrono::Utc;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

use crate::models::form::FormSubmission;

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
    pub created_at: String,   // ISO format
    pub cancelled_at: String, // ISO format (empty if not cancelled)
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
                "created_at",
                "cancelled_at",
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

    // Store a new meeting record in the database
    pub fn store_meeting(
        &self,
        form: &FormSubmission,
        meeting_id: &str,
        room_name: &str,
    ) -> Result<(), String> {
        // Get current time in UTC
        let now = Utc::now();

        // Create a new record
        let record = MeetingRecord {
            entry_token: form.entry.token.clone(),
            form_id: form.form.clone(),
            form_name: form.form_name.clone(),
            subject: form.entry.field_8.clone(),
            room_name: room_name.to_string(),
            scheduled_at: form
                .entry
                .field_1
                .first()
                .map(|item| item.scheduled_at.clone())
                .unwrap_or_else(|| now.to_rfc3339()),
            scheduled_label: form
                .entry
                .field_1
                .first()
                .map(|item| item.scheduled_label.clone())
                .unwrap_or_default(),
            status: form.entry.reservation_status_fsf_field.clone(),
            meeting_id: meeting_id.to_string(),
            created_at: now.to_rfc3339(),
            cancelled_at: "".to_string(),
        };

        self.write_record(&record)
    }

    // Update meeting status to cancelled
    pub fn cancel_meeting(&self, entry_token: &str) -> Result<Option<String>, String> {
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
        let mut meeting_id = None;

        // Find the record with matching token and copy all records
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;

            // Check if this is the record to update
            if record.get(0) == Some(entry_token) && record.get(7) == Some("Reserved") {
                // Add to records with updated status
                let mut updated = record.clone();

                // Get meeting_id for cancellation
                meeting_id = record.get(8).map(String::from);

                // Update the record fields in-place
                if let Some(meeting_id_str) = &meeting_id {
                    let now = Utc::now().to_rfc3339();

                    // Build a new record with updated status and cancelled_at time
                    let mut updated_vec: Vec<String> = record.iter().map(String::from).collect();
                    updated_vec[7] = "Cancelled".to_string(); // Update status
                    updated_vec[10] = now.clone(); // Update cancelled_at

                    updated = StringRecord::from(updated_vec);

                    info!(
                        "Marked meeting {} as cancelled for token {}",
                        meeting_id_str, entry_token
                    );
                }

                records.push(updated);
            } else {
                records.push(record);
            }
        }

        // If no matching record found
        if meeting_id.is_none() {
            warn!("No active meeting found for token: {}", entry_token);
            return Ok(None);
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

        Ok(meeting_id)
    }

    // Find a meeting by entry token
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

            if record.get(0) == Some(entry_token) && record.get(7) == Some("Reserved") {
                // Convert to MeetingRecord
                return Ok(Some(self.string_record_to_meeting_record(&record)?));
            }
        }

        // No matching record found
        Ok(None)
    }

    // List all meetings (optional for future use)
    pub fn list_meetings(&self) -> Result<Vec<MeetingRecord>, String> {
        let _lock = self
            .file_mutex
            .lock()
            .map_err(|e| format!("Failed to acquire mutex: {}", e))?;

        let file = File::open(&self.csv_path)
            .map_err(|e| format!("Failed to open database file: {}", e))?;

        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);

        let mut records = Vec::new();

        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read record: {}", e))?;
            records.push(self.string_record_to_meeting_record(&record)?);
        }

        Ok(records)
    }

    // Convert StringRecord to MeetingRecord
    fn string_record_to_meeting_record(
        &self,
        record: &StringRecord,
    ) -> Result<MeetingRecord, String> {
        if record.len() < 11 {
            return Err(format!("Invalid record length: {}", record.len()));
        }

        Ok(MeetingRecord {
            entry_token: record.get(0).unwrap_or_default().to_string(),
            form_id: record.get(1).unwrap_or_default().to_string(),
            form_name: record.get(2).unwrap_or_default().to_string(),
            subject: record.get(3).unwrap_or_default().to_string(),
            room_name: record.get(4).unwrap_or_default().to_string(),
            scheduled_at: record.get(5).unwrap_or_default().to_string(),
            scheduled_label: record.get(6).unwrap_or_default().to_string(),
            status: record.get(7).unwrap_or_default().to_string(),
            meeting_id: record.get(8).unwrap_or_default().to_string(),
            created_at: record.get(9).unwrap_or_default().to_string(),
            cancelled_at: record.get(10).unwrap_or_default().to_string(),
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
    let csv_path =
        std::env::var("MEETING_DATABASE_PATH").unwrap_or_else(|_| "meetings.csv".to_string());

    Arc::new(DatabaseService::new(&csv_path))
}
