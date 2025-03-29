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
pub mod client;
pub mod auth;

// Web API modules
pub mod handlers;
pub mod models;
pub mod services;
pub mod routes;

// Re-export the main API types for ease of use
pub use client::{
    TencentMeetingClient, 
    MeetingRoomItem, 
    MeetingRoomsResponse,
    CreateMeetingRequest,
    CreateMeetingResponse,
    CancelMeetingRequest,
    User,
    MeetingSettings,
    MeetingInfo,
};
pub use auth::TencentAuth;
pub use handlers::api::AppState;
pub use models::common::PaginationParams;
pub use models::form::FormSubmission;
pub use models::meeting::WebhookResponse;
pub use routes::create_router;