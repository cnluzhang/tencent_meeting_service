use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FormField1Item {
    pub item_name: String,
    pub scheduled_label: String,
    pub number: i32,
    pub scheduled_at: String,
    pub api_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FormEntry {
    pub token: String,
    pub field_1: Vec<FormField1Item>,
    pub field_8: String,
    #[serde(flatten)]
    pub extra_fields: HashMap<String, Value>,
    pub reservation_status_fsf_field: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FormSubmission {
    pub form: String,
    pub form_name: String,
    pub entry: FormEntry,
}

// Test data structure for mock responses
#[derive(Debug, Serialize)]
pub struct TestFormSubmission {
    pub example: FormSubmission,
    pub description: String,
}
