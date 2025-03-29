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
//!
//! # Authentication
//!
//! The library uses AKSK (AppId, SecretId, SecretKey) authentication with HMAC-SHA256
//! signatures as required by Tencent Meeting API. The authentication logic is
//! encapsulated in the `auth` module.

pub mod client;
pub mod auth;

// Re-export the main API types for ease of use
pub use client::{TencentMeetingClient, MeetingRoomItem, MeetingRoomsResponse};
pub use auth::TencentAuth;
