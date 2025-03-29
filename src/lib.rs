//! Tencent Meeting API Service
//! 
//! This library provides a Rust client for the Tencent Meeting API
//! and a web service for accessing meeting room information.
//! It can be used to bridge between form services and Tencent Meeting.

pub mod client;

// Re-export the main API types for ease of use
pub use client::{TencentMeetingClient, MeetingRoomItem, MeetingRoomsResponse};
