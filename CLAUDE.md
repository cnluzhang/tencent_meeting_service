# CLAUDE.md - Guidelines for Tencent Meeting Service

## Project Commands

### Quick Test Environment
- Start test server: `docker compose -f docker-compose.test.yml up -d`
- Access health check: `http://localhost:3001/health`
- View logs: `docker compose -f docker-compose.test.yml logs -f`
- Stop test server: `docker compose -f docker-compose.test.yml down`

### Docker Development Environment
- Start dev environment: `docker compose up -d dev`
- Start production environment: `docker compose up -d app`
- Access dev container: `docker compose exec dev bash`
- Build inside container: `docker compose exec dev cargo build`
- Run inside container: `docker compose exec dev cargo run`
- Test all: `docker compose exec dev cargo test`
- Test single: `docker compose exec dev cargo test test_name`
- Lint: `docker compose exec dev cargo clippy`
- Format: `docker compose exec dev cargo fmt`

### Local Development
- Install dependencies: `cargo build`
- Run tests: `cargo test`
- Run server: `cargo run`
- Check code: `cargo clippy`
- Format code: `cargo fmt`

## Test Suite

The project includes a comprehensive test suite covering core functionality, organized in a clear structure:

### Test Organization 

```
src/tests/
├── common/              # Shared test utilities
│   ├── fixtures.rs      # Test data generators
│   ├── mocks.rs         # Mock implementations
│   ├── test_utils.rs    # Shared test functions
│   └── mod.rs           # Module exports
├── services/            # Service-level tests
│   ├── database_test.rs # Database service tests
│   ├── time_slots_test.rs # Time slot service tests
│   └── mod.rs           # Module exports
├── handlers/            # API handler tests
│   ├── api_test.rs      # API endpoint handler tests
│   └── mod.rs           # Module exports
├── integration/         # End-to-end tests
│   ├── api_test.rs      # API endpoint integration tests
│   ├── webhook_test.rs  # Webhook functionality tests
│   ├── workflow_test.rs # Complete workflow tests
│   └── mod.rs           # Module exports
├── client_test.rs       # Client tests
└── mod.rs               # Root test module
```

### Database Tests
- `test_database_creation`: Tests the creation of the database file
- `test_store_meeting_with_time_slot`: Tests storing a single meeting with a specific time slot
- `test_store_merged_meeting`: Tests storing a merged meeting with multiple time slots
- `test_cancel_meeting`: Tests cancelling a meeting and updating its status
- `test_multiple_meetings_same_token`: Tests handling multiple meetings with the same token
- `test_deduplication`: Tests prevention of duplicate meeting entries

### Time Slot Tests
- `test_parse_time_slot`: Tests parsing time slots from form entries
- `test_find_mergeable_groups_empty`: Tests handling empty time slot collections
- `test_find_mergeable_groups_single`: Tests handling a single time slot
- `test_find_mergeable_groups_consecutive`: Tests identifying consecutive time slots
- `test_find_mergeable_groups_non_consecutive`: Tests handling non-consecutive time slots
- `test_find_mergeable_groups_different_rooms`: Tests handling slots in different rooms
- `test_find_mergeable_groups_complex`: Tests complex combinations of time slots

### Authentication Tests
- `test_generate_nonce`: Tests nonce generation for API authentication
- `test_get_timestamp`: Tests timestamp generation for API requests
- `test_generate_signature`: Tests HMAC-SHA256 signature generation

### Client Tests
- `test_list_rooms`: Tests retrieving meeting rooms
- `test_create_meeting`: Tests creating a meeting
- `test_cancel_meeting`: Tests cancelling a meeting
- `test_book_rooms`: Tests booking rooms for a meeting
- `test_release_rooms`: Tests releasing previously booked rooms
- `test_workflow_create_book_cancel_release`: Tests the complete meeting lifecycle

### API Handler Tests
- `test_webhook_form_submission`: Tests form submission handling
- `test_webhook_form_cancellation`: Tests cancellation form handling
- `test_health_endpoint`: Tests the service health check endpoint
- `test_meeting_rooms_endpoint`: Tests the meeting rooms listing endpoint
- `test_meeting_rooms_handler`: Tests the meeting rooms handler directly
- `test_multiple_time_slots`: Tests handling submissions with multiple time slots
- `test_simulation_mode`: Tests the simulation mode feature
- `test_invalid_form_submission`: Tests validation of incorrect form data
- `test_form_with_unknown_status`: Tests handling of forms with invalid status values

### Integration Tests
- `test_complete_reservation_workflow`: Tests the end-to-end reservation workflow
- `test_multi_slot_reservation_with_merging`: Tests merging of consecutive time slots
- `test_simulation_mode`: Tests the end-to-end simulation mode behavior
- `test_error_handling_invalid_form`: Tests system error handling
- `test_concurrent_requests`: Tests parallel processing of multiple requests
- `test_list_meeting_rooms`: Tests the meeting rooms listing endpoint in context

### Running Tests
- Run all tests: `docker compose exec dev cargo test`
- Run specific module: `docker compose exec dev cargo test tests::services::database_test`
- Run with output: `docker compose exec dev cargo test -- --nocapture`
- Run a specific test: `docker compose exec dev cargo test test_simulation_mode`
- Run integration tests only: `docker compose exec dev cargo test tests::integration`
- Run client tests only: `docker compose exec dev cargo test tests::client_test`

## API Endpoints
- `GET /health` - Health check endpoint
- `GET /meeting-rooms?page=1&page_size=20` - Get meeting rooms with pagination
- `POST /meetings` - Create a new meeting
- `POST /meetings/{meeting_id}/cancel` - Cancel an existing meeting
- `POST /meetings/{meeting_id}/book-rooms` - Book meeting rooms for an existing meeting
- `POST /meetings/{meeting_id}/release-rooms` - Release previously booked meeting rooms
- `POST /webhook/form-submission` - Webhook endpoint for form submissions to create meetings

## Required Environment Variables
```
# Required for Tencent Meeting API authentication
TENCENT_MEETING_APP_ID=your_app_id
TENCENT_MEETING_SECRET_ID=your_secret_id
TENCENT_MEETING_SECRET_KEY=your_secret_key
TENCENT_MEETING_SDK_ID=your_sdk_id
# Format: name1:id1,name2:id2,name3:id3 (supports all current operators)
TENCENT_MEETING_OPERATOR_ID=name1:id1,name2:id2,name3:id3

# Form field mappings (required)
FORM_USER_FIELD_NAME=user_field_name
FORM_DEPT_FIELD_NAME=department_field_name

# Room booking (required)
XA_MEETING_ROOM_ID=your_xian_room_id
CD_MEETING_ROOM_ID=your_chengdu_room_id

# Security (optional but recommended)
WEBHOOK_AUTH_TOKEN=your_secure_token  # Authentication token for webhook endpoints

# Feature toggles (optional)
SKIP_MEETING_CREATION=false  # Set to true to only store in database without API calls
SKIP_ROOM_BOOKING=false      # Set to true to create meetings but skip room booking
ENVIRONMENT=development      # Set to "production" to only expose webhook and health endpoints

# Error tracking (optional)
SENTRY_DSN=https://your-sentry-dsn  # DSN from Sentry for error monitoring

# Database configuration (optional)
MEETING_DATABASE_PATH=/app/data/meetings.csv  # Path to CSV database file
```

## Feature Toggles
- `SKIP_MEETING_CREATION=true` - Simulation mode: processes forms but doesn't make API calls
- `SKIP_ROOM_BOOKING=true` - Creates meetings but skips room booking API calls
- Both toggles can be used in combination for different testing scenarios
- `ENVIRONMENT=production` - Restricts exposed endpoints to only webhook and health endpoints

## Time Slot Handling
- Support for precise time parsing with minute granularity (e.g., "14:00-14:30")
- Consecutive time slots in the same room are automatically identified and merged
- Past time slots are handled as follows:
  - If both start and end times are in the past: Returns an error (no meeting can be created for entirely past time slots)
  - If only start time is in the past but end time is in the future: Adjusted start time to current time + 2 minutes while preserving the original end time
  - This preserves the continuity of consecutive time slots even when some are in the past
- Time slots are grouped by room and consecutive timing for optimal meeting creation

## Production Mode
- Set `ENVIRONMENT=production` to run the service in production mode
- Only exposes the webhook endpoint (`/webhook/form-submission`) and health check endpoint (`/health`)
- Restricts access to management API endpoints for security
- Recommended for production deployments to minimize attack surface
- Use together with webhook authentication for maximum security

## Error Monitoring with Sentry
- Optional Sentry integration for error tracking and monitoring
- Enable by setting the SENTRY_DSN environment variable and using the sentry-monitoring feature flag
- Automatically captures unhandled errors in the web service
- Environment-aware reporting (development vs production)
- Releases are tagged automatically using Cargo package version

## Graceful Shutdown
- Service includes graceful shutdown support for SIGTERM and SIGINT (Ctrl+C) signals
- Active connections are allowed to complete before server termination
- Shutdown process is logged for monitoring and debugging purposes

## Data Storage
- Meeting data is stored in a persistent CSV file in a Docker volume
- Default location: `/app/data/meetings.csv`
- Deduplication to prevent duplicate entries based on token and status
- Database automatically handles both English and Chinese status values
- Stores operator name and ID information for each meeting

## Code Organization

### Project File Structure
```
tencent_meeting_service/
├── Cargo.toml           # Project dependencies
├── Dockerfile           # Multi-stage Docker configuration
├── Dockerfile.test      # Test-specific Docker configuration
├── docker-compose.yml   # Docker Compose setup
├── docker-compose.test.yml # Test environment Docker Compose setup
├── README.md            # Project documentation
├── CLAUDE.md            # Development guidelines
├── data/
│   └── meetings.csv     # CSV database for meeting storage
└── src/
    ├── main.rs          # Application entry point
    ├── lib.rs           # Library exports
    ├── auth.rs          # Authentication utilities for Tencent Meeting API
    ├── client.rs        # Tencent Meeting API client
    ├── routes.rs        # API routes configuration
    ├── handlers/        # API endpoint handlers
    │   ├── api.rs       # Main API endpoints
    │   ├── mod.rs       # Module exports
    │   └── test.rs      # Test endpoints
    ├── models/          # Data structures and types
    │   ├── common.rs    # Shared types
    │   ├── form.rs      # Form-related structures
    │   ├── meeting.rs   # Meeting-related structures
    │   └── mod.rs       # Module exports
    ├── services/        # Business logic
    │   ├── database.rs  # CSV database operations
    │   ├── mod.rs       # Module exports
    │   └── time_slots.rs # Time slot processing
    ├── tests/           # Organized testing infrastructure
    │   ├── common/      # Shared test utilities
    │   │   ├── fixtures.rs # Test data generators
    │   │   ├── mocks.rs # Mock implementations (moved from client_mock.rs)
    │   │   ├── test_utils.rs # Shared test functions
    │   │   └── mod.rs   # Common module exports
    │   ├── services/    # Service tests
    │   │   ├── database_test.rs # Database service tests
    │   │   ├── time_slots_test.rs # Time slot tests
    │   │   └── mod.rs   # Services module exports
    │   ├── handlers/    # Handler tests
    │   │   ├── api_test.rs # API handler tests
    │   │   └── mod.rs   # Handler module exports
    │   ├── integration/ # Integration tests
    │   │   ├── api_test.rs # API endpoint tests
    │   │   ├── webhook_test.rs # Webhook tests
    │   │   ├── workflow_test.rs # End-to-end workflow tests
    │   │   └── mod.rs   # Integration module exports
    │   ├── client_test.rs # Client tests
    │   └── mod.rs       # Root test module
```

### Module Structure
- **handlers/**: API endpoint handlers
  - **api.rs**: Main API endpoints (meeting rooms, meeting creation/cancellation, form webhook)
  - **api_test.rs**: Tests for API endpoints
  - **test.rs**: Test and health check endpoints
  - **mod.rs**: Module exports
- **models/**: Data structures
  - **common.rs**: Shared types like PaginationParams
  - **form.rs**: Form submission data structures
  - **meeting.rs**: Meeting-related data structures
  - **mod.rs**: Module exports
- **services/**: Business logic
  - **database.rs**: CSV database operations for storing meetings
  - **database_test.rs**: Tests for database operations
  - **time_slots.rs**: Time slot processing, merging, and meeting creation
  - **time_slots_test.rs**: Tests for time slot operations
  - **mod.rs**: Module exports
- **routes.rs**: Centralizes all API routes
- **client.rs**: Tencent Meeting API client implementation
- **client_mock.rs**: Mock client for testing without real API calls
- **client_test.rs**: Tests for the client implementation
- **auth.rs**: Authentication utilities for Tencent Meeting API
- **tests.rs**: General test utilities
- **integration_tests.rs**: End-to-end integration tests

### Code Style Guidelines
- **Imports**: Group standard library, external crates, then local modules
- **Formatting**: Follow rustfmt conventions; run `cargo fmt` before commits
- **Types**: Use strong typing with Serde for JSON serialization/deserialization
- **Error Handling**: Use Result<T, E> with proper error propagation; avoid unwrap() in production code
- **Naming**: Use snake_case for variables/functions; CamelCase for types/traits
- **Comments**: Document public API with /// comments; explain complex code blocks
- **Environment**: All configuration via environment variables; define in .env file
- **Async**: Use .await with proper error propagation; prefer ? operator for Results
- **Security**: Never hardcode credentials; always use .env for sensitive values
- **Logging**: Use tracing crate with appropriate log levels (debug/info/warn/error)
- **Web Routes**: Keep handler functions small and focused on business logic
- **Modularity**: Keep files focused on a single responsibility; prefer many small files over few large ones

## Tencent Meeting API Authentication
- **Authentication Module**: `src/auth.rs` contains the `TencentAuth` struct with authentication utilities
- **Authentication Method**: AKSK (AppId, SecretId, SecretKey) with HMAC-SHA256 signatures
- **TencentAuth Functions**:
  - `generate_signature`: Creates HMAC-SHA256 signatures for API requests
  - `generate_nonce`: Generates random 8-digit nonces for request uniqueness
  - `get_timestamp`: Provides current Unix timestamps

## Operator Management
- **Multiple Operators**: Support for multiple operators configured in environment variables (currently 42 operators are used)
- **Operator Mapping Format**: `name1:id1,name2:id2,name3:id3,...` in TENCENT_MEETING_OPERATOR_ID
- **Operator Selection**: Automatic matching of form submission user name to operator ID
- **Default Fallback**: If no match found, uses first operator in the list as default
- **Database Storage**: Stores both operator name and ID with each meeting record
- **Helper Functions**: `get_operator_info` automatically extracts and maps operator information
- **Signature Format**: 
  ```
  httpMethod + "\n" + 
  headerString + "\n" + 
  uri + "\n" + 
  body
  ```
  where headerString is: "X-TC-Key=secretId&X-TC-Nonce=nonce&X-TC-Timestamp=timestamp"
- **Required Headers**:
  - Content-Type: application/json
  - X-TC-Key: SecretId
  - X-TC-Timestamp: Unix timestamp 
  - X-TC-Nonce: Random integer
  - X-TC-Signature: Base64 encoded hex string of HMAC-SHA256 signature
  - AppId: Enterprise ID
  - SdkId: User sub-account or application ID (if available)
  - X-TC-Registered: Set to "1"
- **Required Parameters**:
  - operator_id: User ID of the operator
  - operator_id_type: Type of operator ID (1 for userid)

## Integration With Form Services
When connecting to form services, make sure to:
1. Implement proper CORS headers
2. Use structured error handling
3. Add validation for incoming form data
4. Ensure proper timezone handling in scheduling APIs

### Form-Specific Customizations
- Form name `西安会议室预约` uses Xi'an meeting room ID and sets location to `西安-大会议室`
- Form name `成都会议室预约` uses Chengdu meeting room ID and sets location to `成都-天府广场`
- Other form names default to Xi'an meeting room ID with generic location formatting

## Form Webhook Structure
The webhook endpoint (`/webhook/form-submission?auth=your_token`) expects the following JSON structure:

> **Authentication:** The endpoint can be secured with a token via the `auth` query parameter that must match the `WEBHOOK_AUTH_TOKEN` environment variable if it's set.

For meeting creation:

```json
{
  "form": "form_id",
  "form_name": "Meeting Room Reservation",
  "entry": {
    "token": "token123",
    "field_1": [
      {
        "item_name": "Conference Room A",
        "scheduled_label": "2025-03-30 09:00-10:00",
        "number": 1,
        "scheduled_at": "2025-03-30T01:00:00.000Z",
        "api_code": "CODE1"
      }
    ],
    "field_8": "Meeting Subject",
    "user_field_name": "User Name",
    "department_field_name": "Department",
    "reservation_status_fsf_field": "已预约"
  }
}
```

For meeting cancellation (uses the same token to identify the meeting):

```json
{
  "form": "form_id",
  "form_name": "Meeting Room Reservation",
  "entry": {
    "token": "token123",
    "field_1": [
      {
        "item_name": "Conference Room A",
        "scheduled_label": "2025-03-30 09:00-10:00",
        "number": 1,
        "scheduled_at": "2025-03-30T01:00:00.000Z",
        "api_code": "CODE1"
      }
    ],
    "field_8": "Meeting Subject",
    "user_field_name": "User Name",
    "department_field_name": "Department",
    "reservation_status_fsf_field": "已取消"
  }
}
```

The webhook processes reservations ("已预约") as follows:
1. Creates a meeting in Tencent Meeting with:
   - Meeting subject from field_8
   - Meeting time from scheduled_at (in UTC)
   - Meeting duration calculated from the time range in scheduled_label (e.g., "09:00-10:00")
   - Meeting creator using the operator_id from environment variables
   - Meeting location as the item_name and department
   - Meeting instance ID set to 32
   - Meeting type set to 0 (scheduled meeting)
2. Books the meeting room specified in DEFAULT_MEETING_ROOM_ID environment variable
3. Stores the meeting details in the database, including:
   - Form entry token (used to identify the meeting later)
   - Meeting ID returned from Tencent Meeting API
   - Room ID used for booking
   - Current status ("Reserved")

The webhook processes cancellations ("已取消") as follows:
1. Looks up the meeting in the database using the form entry token
2. First releases the meeting room using the room ID
3. Then cancels the meeting using the meeting ID
4. Updates the meeting status in the database to "Cancelled"

For multiple time slots in a single form submission:
- Groups time slots into mergeable sets based on continuity and room
- Creates meetings for each mergeable group:
  - Merged meetings when slots are contiguous in the same room
  - Individual meetings for non-mergeable slots
- Processes all available time slots, not just the first group
- Returns a detailed response with information about all created meetings