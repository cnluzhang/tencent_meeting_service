use hyper::Response;
use mockall::mock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::client::{
    BookRoomsRequest, CancelMeetingRequest, CreateMeetingRequest, CreateMeetingResponse,
    MeetingInfo, MeetingRoomItem, MeetingRoomsResponse, ReleaseRoomsRequest,
};

// Define a mock client for the Tencent Meeting API
mock! {
    pub TencentMeetingClient {
        // Mock the getter for operator_id
        pub fn get_operator_id(&self) -> &str;

        // Mock all the API methods
        pub async fn list_rooms(
            &self,
            page: usize,
            page_size: usize,
        ) -> Result<MeetingRoomsResponse, reqwest::Error>;

        pub async fn create_meeting(
            &self,
            request: &CreateMeetingRequest,
        ) -> Result<CreateMeetingResponse, reqwest::Error>;

        pub async fn cancel_meeting(
            &self,
            meeting_id: &str,
            request: &CancelMeetingRequest,
        ) -> Result<(), reqwest::Error>;

        pub async fn book_rooms(
            &self,
            meeting_id: &str,
            request: &BookRoomsRequest,
        ) -> Result<(), reqwest::Error>;

        pub async fn release_rooms(
            &self,
            meeting_id: &str,
            request: &ReleaseRoomsRequest,
        ) -> Result<(), reqwest::Error>;
    }
}

// A simple in-memory store for our mock client
pub struct MockDataStore {
    meetings: Mutex<HashMap<String, MeetingInfo>>,
    rooms: Mutex<HashMap<String, MeetingRoomItem>>,
    booked_rooms: Mutex<HashMap<String, Vec<String>>>, // meeting_id -> room_ids
}

impl MockDataStore {
    pub fn new() -> Self {
        let mut rooms = HashMap::new();

        // Add some sample meeting rooms
        rooms.insert(
            "room1".to_string(),
            MeetingRoomItem {
                meeting_room_id: "room1".to_string(),
                meeting_room_name: "Conference Room A".to_string(),
                meeting_room_location: "Floor 1".to_string(),
                account_new_type: 1,
                account_type: 1,
                active_code: "RC001".to_string(),
                participant_number: 20,
                meeting_room_status: 1,
                scheduled_status: 0,
                is_allow_call: true,
            },
        );

        rooms.insert(
            "room2".to_string(),
            MeetingRoomItem {
                meeting_room_id: "room2".to_string(),
                meeting_room_name: "Conference Room B".to_string(),
                meeting_room_location: "Floor 2".to_string(),
                account_new_type: 1,
                account_type: 1,
                active_code: "RC002".to_string(),
                participant_number: 10,
                meeting_room_status: 1,
                scheduled_status: 0,
                is_allow_call: true,
            },
        );

        Self {
            meetings: Mutex::new(HashMap::new()),
            rooms: Mutex::new(rooms),
            booked_rooms: Mutex::new(HashMap::new()),
        }
    }

    // Helper methods for the mock client
    pub fn store_meeting(&self, meeting_id: String, meeting_info: MeetingInfo) {
        let mut meetings = self.meetings.lock().unwrap();
        meetings.insert(meeting_id, meeting_info);
    }

    pub fn get_meeting(&self, meeting_id: &str) -> Option<MeetingInfo> {
        let meetings = self.meetings.lock().unwrap();
        meetings.get(meeting_id).cloned()
    }

    pub fn cancel_meeting(&self, meeting_id: &str) -> bool {
        // Just remove the meeting from our store to simulate cancellation
        let mut meetings = self.meetings.lock().unwrap();
        meetings.remove(meeting_id).is_some()
    }

    pub fn book_room(&self, meeting_id: &str, room_ids: &[String]) -> bool {
        let mut booked_rooms = self.booked_rooms.lock().unwrap();
        booked_rooms.insert(meeting_id.to_string(), room_ids.to_vec());
        true
    }

    pub fn release_room(&self, meeting_id: &str) -> bool {
        let mut booked_rooms = self.booked_rooms.lock().unwrap();
        booked_rooms.remove(meeting_id).is_some()
    }

    pub fn list_rooms(&self, page: usize, page_size: usize) -> (Vec<MeetingRoomItem>, usize) {
        let rooms = self.rooms.lock().unwrap();
        let mut all_rooms = Vec::new();

        for room in rooms.values() {
            all_rooms.push(room.clone());
        }

        let total_count = all_rooms.len();

        let start = (page - 1) * page_size;
        let end = std::cmp::min(start + page_size, total_count);

        let paged_rooms = if start < total_count {
            all_rooms[start..end].to_vec()
        } else {
            Vec::new()
        };

        (paged_rooms, total_count)
    }
}

// Helper function to set up a mock client with predefined behavior
pub fn setup_mock_client() -> (MockTencentMeetingClient, Arc<MockDataStore>) {
    let data_store = Arc::new(MockDataStore::new());
    let data_store_clone = Arc::clone(&data_store);

    let mut mock_client = MockTencentMeetingClient::default();

    // Mock operator_id getter
    mock_client
        .expect_get_operator_id()
        .return_const("test_operator".to_string());

    // Mock list_rooms
    let store_ref1 = Arc::clone(&data_store);
    mock_client
        .expect_list_rooms()
        .returning(move |page, page_size| {
            let (rooms, total_count) = store_ref1.list_rooms(page, page_size);

            // Calculate pagination
            let total_page = (total_count as f64 / page_size as f64).ceil() as i32;

            Ok(MeetingRoomsResponse {
                total_count: total_count as i32,
                current_size: rooms.len() as i32,
                current_page: page as i32,
                total_page,
                meeting_room_list: rooms,
            })
        });

    // Mock create_meeting
    let store_ref2 = Arc::clone(&data_store);
    mock_client
        .expect_create_meeting()
        .returning(move |request| {
            // Generate a meeting ID
            let meeting_id = format!("meeting_{}", rand::random::<u32>());
            let meeting_code = format!("MC{}", rand::random::<u16>());

            // Create a meeting info object
            let meeting_info = MeetingInfo {
                subject: request.subject.clone(),
                meeting_id: meeting_id.clone(),
                meeting_code,
                password: request.password.clone(),
                hosts: request.hosts.clone(),
                participants: request.invitees.clone(),
                user_non_registered: None,
                start_time: request.start_time.clone(),
                end_time: request.end_time.clone(),
                join_url: Some(format!("https://example.com/join/{}", meeting_id)),
                settings: request.settings.clone(),
                enable_live: request.enable_live,
                live_config: request.live_config.clone(),
                host_key: request.host_key.clone(),
            };

            // Store the meeting
            store_ref2.store_meeting(meeting_id.clone(), meeting_info.clone());

            Ok(CreateMeetingResponse {
                meeting_number: 1,
                meeting_info_list: vec![meeting_info],
            })
        });

    // Mock cancel_meeting
    let store_ref3 = Arc::clone(&data_store);
    mock_client
        .expect_cancel_meeting()
        .returning(move |meeting_id, _request| {
            // Cancel the meeting in the store
            let _cancelled = store_ref3.cancel_meeting(meeting_id);
            // For testing purposes, we'll always succeed
            // This is a simplification to avoid creating reqwest::Error objects
            Ok(())
        });

    // Mock book_rooms
    let store_ref4 = Arc::clone(&data_store);
    mock_client
        .expect_book_rooms()
        .returning(move |meeting_id, request| {
            // Book the rooms in the store
            let _booked = store_ref4.book_room(meeting_id, &request.meeting_room_id_list);
            // For testing purposes, we'll always succeed
            // This is a simplification to avoid creating reqwest::Error objects
            Ok(())
        });

    // Mock release_rooms
    let store_ref5 = Arc::clone(&data_store);
    mock_client
        .expect_release_rooms()
        .returning(move |meeting_id, _request| {
            // Release the rooms in the store
            let _released = store_ref5.release_room(meeting_id);
            // For testing purposes, we'll always succeed
            // This is a simplification to avoid creating reqwest::Error objects
            Ok(())
        });

    (mock_client, data_store_clone)
}
