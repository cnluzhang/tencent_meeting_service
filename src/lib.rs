//! Tencent Meeting API Service
//!
//! This library provides a Rust client for the Tencent Meeting API
//! and a web service for accessing meeting room information.
//! It can be used to bridge between form services and Tencent Meeting.
//!
//! # Modules
//!
//! - `client`: TencentMeetingClient for API operations
//! - `auth`: Authentication utilities for Tencent Meeting API
//! - `handlers`: API endpoint handlers
//! - `models`: Data structures and types
//! - `services`: Business logic services
//! - `routes`: API routes configuration

// Core modules
pub mod auth;
pub mod client;

// Web API modules
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

// Test module
#[cfg(test)]
mod tests;

// Re-export the main API types for ease of use
pub use auth::TencentAuth;
pub use client::{
    CancelMeetingRequest, CreateMeetingRequest, CreateMeetingResponse, MeetingInfo,
    MeetingRoomItem, MeetingRoomsResponse, MeetingSettings, TencentMeetingClient, User,
};
pub use handlers::api::AppState;
pub use models::common::PaginationParams;
pub use models::form::FormSubmission;
pub use models::meeting::WebhookResponse;
pub use routes::create_router;
