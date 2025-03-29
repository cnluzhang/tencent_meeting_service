use serde::Deserialize;

// Define pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

pub fn default_page() -> usize {
    1
}

pub fn default_page_size() -> usize {
    20
}
