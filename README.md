# Tencent Meeting Service

A web service that provides a bridge between form services and Tencent Meeting API. It authenticates with the Tencent Meeting enterprise API using AKSK (AppId, SecretId, SecretKey) authentication to provide meeting room information and scheduling functionality.

## Features

- RESTful API for meeting room data
- Secure authentication with Tencent Meeting API
- Webhook endpoint with optional token-based authentication
- Docker-ready for easy deployment
- Production and development environments
- Configurable via environment variables
- Health check endpoint for monitoring
- Graceful shutdown with proper signal handling
- Optional Sentry integration for error monitoring
- Comprehensive test suite with 30+ automated tests

## Project Structure

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
    ├── client_mock.rs   # Mock client for testing
    ├── client_test.rs   # Tests for the client implementation
    ├── routes.rs        # API routes configuration
    ├── tests.rs         # General test utilities
    ├── integration_tests.rs # End-to-end integration tests
    ├── handlers/        # API endpoint handlers
    │   ├── api.rs       # Main API endpoints
    │   ├── api_test.rs  # Tests for API endpoints
    │   ├── mod.rs       # Module exports
    │   └── test.rs      # Test endpoints
    ├── models/          # Data structures and types
    │   ├── common.rs    # Shared types
    │   ├── form.rs      # Form-related structures
    │   ├── meeting.rs   # Meeting-related structures
    │   └── mod.rs       # Module exports
    ├── routes/          # Route organization 
    └── services/        # Business logic
        ├── database.rs  # CSV database operations
        ├── database_test.rs # Tests for database operations
        ├── mod.rs       # Module exports
        ├── time_slots.rs # Time slot processing
        └── time_slots_test.rs # Tests for time slot operations
```

## API Endpoints

- `GET /health` - Health check endpoint
- `GET /meeting-rooms?page=1&page_size=20` - Get meeting rooms with pagination
- `POST /meetings` - Create a new meeting with Tencent Meeting API
- `POST /meetings/{meeting_id}/cancel` - Cancel an existing meeting
- `POST /meetings/{meeting_id}/book-rooms` - Book meeting rooms for an existing meeting
- `POST /meetings/{meeting_id}/release-rooms` - Release previously booked meeting rooms
- `POST /webhook/form-submission?auth=token` - Webhook endpoint for form submissions (with optional authentication)

## Setup

1. Edit the `.env` file to add your Tencent Meeting API credentials:

```
# Required credentials
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

# Optional settings
TENCENT_MEETING_API_ENDPOINT=https://api.meeting.qq.com
RUST_LOG=info

# Feature toggles (optional)
SKIP_MEETING_CREATION=false  # Set to true to only store in database without API calls
SKIP_ROOM_BOOKING=false      # Set to true to create meetings but skip room booking
ENVIRONMENT=development      # Set to "production" to restrict endpoints for production

# Error monitoring (optional)
SENTRY_DSN=https://your-sentry-dsn  # DSN from Sentry dashboard for error monitoring

# Database configuration (optional)
MEETING_DATABASE_PATH=/app/data/meetings.csv  # Path to CSV database file
```

## Feature Toggles

The service supports several environment variables to control its behavior:

1. **SKIP_MEETING_CREATION** - When set to `true`:
   - No API calls are made to Tencent Meeting for meeting creation/cancellation
   - Form submissions are only stored in the database
   - Simulated meeting IDs are generated
   - Useful for testing form processing without making actual API calls

2. **SKIP_ROOM_BOOKING** - When set to `true`:
   - Meetings are created normally in Tencent Meeting
   - No room booking API calls are made
   - Useful when room booking is handled separately

3. **ENVIRONMENT** - When set to `production`:
   - Only exposes the webhook endpoint (`/webhook/form-submission`) and health check (`/health`)
   - Restricts all management API endpoints for security
   - Reduces attack surface for production deployments
   - Recommended for any public-facing deployment
   
4. **SENTRY_DSN** - When set:
   - Enables error tracking and monitoring with Sentry (requires sentry-monitoring feature flag)
   - Automatically reports unhandled exceptions to your Sentry dashboard
   - Includes environment information and release version
   - Useful for production monitoring and debugging

## Data Storage

The service uses a simple CSV file-based database to track meeting reservations:

- Stored in a dedicated Docker volume for persistence
- Default path is `/app/data/meetings.csv`
- Can be customized via the `MEETING_DATABASE_PATH` environment variable
- Includes deduplication to prevent duplicate entries
- Stores meeting details, room IDs, and status information

## Quick Test

The simplest way to test the service is using the provided test Docker configuration:

```bash
docker compose -f docker-compose.test.yml up -d
```

The test server will be available at `http://localhost:3001`. You can access the health check at:
- `http://localhost:3001/health` - Health check endpoint

## Development with Docker

Start the development environment:

```bash
docker compose up -d dev
```

In development mode, the application automatically reloads when you make changes to the code.

### Running Tests

Run the comprehensive test suite:

```bash
docker compose exec dev cargo test
```

Run tests with output:

```bash
docker compose exec dev cargo test -- --nocapture
```

Run a specific test module:

```bash
docker compose exec dev cargo test database_tests
```

Run a specific group of tests:

```bash
docker compose exec dev cargo test client_tests  # Run all client tests
docker compose exec dev cargo test integration_tests  # Run integration tests
```

The test suite includes:
- Database operation tests
- Time slot processing tests
- Authentication tests
- Client API tests
- Handler tests
- Integration tests with simulated API calls
- Error handling tests

## Production Deployment

Start the production environment:

```bash
docker compose up -d app
```

The service will be available at `http://localhost:3000`.

## Authentication Method

This service implements the AKSK (AppId, SecretId, SecretKey) authentication method for Tencent Meeting API. The authentication logic is encapsulated in the `auth.rs` module, which provides utilities for generating signatures, timestamps, and nonces for API requests following Tencent's specifications.

### Multiple Operator Support

The service supports dynamic operator selection based on form submissions:

- Configure any number of operators using the `TENCENT_MEETING_OPERATOR_ID` environment variable (currently 42 operators are used)
- Format: `name1:id1,name2:id2,name3:id3,...`
- When a form is submitted, the system looks up the matching operator by name from the form field specified in `FORM_USER_FIELD_NAME`
- The matched operator's ID is used for all API calls related to that meeting
- If no match is found, the first operator is used as default
- Each meeting record stores both the operator name and ID for tracking purposes

The `TencentAuth` struct provides the following functionality:
- `generate_signature` - Creates HMAC-SHA256 signatures for API requests
- `generate_nonce` - Generates random nonces for request uniqueness
- `get_timestamp` - Provides current Unix timestamps

Required headers:
- `Content-Type` - application/json
- `X-TC-Key` - SecretId
- `X-TC-Timestamp` - Current Unix timestamp
- `X-TC-Nonce` - Random integer
- `X-TC-Signature` - HMAC-SHA256 signature
- `AppId` - Enterprise ID
- `SdkId` - User sub-account or application ID (if available)
- `X-TC-Registered` - Set to "1"

Required query parameters for meeting room endpoints:
- `operator_id` - User ID of the operator making the request
- `operator_id_type` - Type of operator ID (1 for userid)

## Codebase Organization

The project uses a modular architecture to improve maintainability and separation of concerns, with an extensive test suite covering core functionality:

1. **Models** (`src/models/`) - Data structures and types
   - Common types shared across the application
   - Form submission data structures
   - Meeting-related data structures

2. **Handlers** (`src/handlers/`) - API endpoint handlers
   - Production API endpoints
   - Test endpoints for development and debugging

3. **Services** (`src/services/`) - Business logic
   - Time slot processing logic
   - Meeting creation and merging logic
   - CSV database for persistent storage

4. **Routes** (`src/routes.rs`) - Centralized routing configuration
   - Manages all API endpoints in a single location
   - Separates routing concerns from business logic

5. **Client** (`src/client.rs`) - Tencent Meeting API client
   - Handles communication with the Tencent Meeting API
   - Encapsulates request/response handling
   - Supports multiple operators with dynamic selection based on form data
   - Comprehensive mocking support for testing

6. **Time Slot Processing** (`src/services/time_slots.rs`) - Meeting time management
   - Supports precise time parsing with minute granularity (e.g., "14:00-14:30")
   - Returns an error if both start and end times are in the past (cannot create completely past meetings)
   - Handles partially past time slots by adjusting start times while preserving end times
   - Automatically merges consecutive time slots for efficient meeting creation
   - Handles time zone conversions and format parsing

7. **Authentication** (`src/auth.rs`) - Authentication utilities
   - HMAC-SHA256 signature generation
   - Nonce and timestamp utilities
   - Tested signature validation
   
8. **Database** (`src/services/database.rs`) - Simple CSV-based storage
   - Stores meeting records in a persistent CSV file
   - Handles record creation, retrieval, and updates
   - Provides deduplication to prevent duplicate entries
   - Data is stored in a Docker volume for persistence

## Form Service Integration

The service is designed to be integrated with form services, allowing users to:

1. View available meeting rooms
2. Schedule meetings in these rooms through form submissions
3. Check meeting room availability

### Form Webhook Integration

The service includes a webhook endpoint (`/webhook/form-submission`) implemented in `src/handlers/api.rs` that accepts form submissions and automatically creates meetings in Tencent Meeting. 

**Webhook Security:**
- The endpoint can be secured with a token via the `WEBHOOK_AUTH_TOKEN` environment variable
- Clients must provide this token in the `auth` query parameter: `/webhook/form-submission?auth=your_token`
- If no token is configured, authentication is disabled
- Returns 401 Unauthorized when authentication fails

The webhook expects the following JSON structure:

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
    "reservation_status_fsf_field": "Reserved"
  }
}
```

The service processes this data as follows:
- Meeting subject is taken from field_8
- Meeting time is taken from scheduled_at (in UTC format)
- Meeting duration is calculated from the time range in scheduled_label (e.g., "09:00-10:00")
- The operator_id is determined by matching the user name from the form with the configured operator mappings
- Location is set based on form name ('西安-大会议室' for Xi'an forms, '成都-天府广场' for Chengdu forms)
- Meeting instance ID is set to 32 (as required by the API)
- After meeting creation, the appropriate meeting room based on form name is booked automatically

When multiple time slots are submitted in a single form:
1. The service attempts to find all mergeable groups of time slots
2. For each mergeable group:
   - If the group has multiple time slots that are contiguous and in the same room, they are merged into a single meeting
   - If the group has only one time slot, a single meeting is created for it
   - Consecutive time slots remain mergeable even when some are in the past (start times are adjusted while preserving end times)
3. For each created meeting, the service:
   - Books the appropriate meeting room based on form name (XA_MEETING_ROOM_ID for Xi'an forms, CD_MEETING_ROOM_ID for Chengdu forms)
   - Stores the meeting ID and room ID in the database for future reference
4. The response includes details for all created meetings, indicating:
   - Which time slots were merged
   - Which room was used for each meeting
   - Success/failure status for each meeting
   - Meeting IDs for successfully created meetings

For meeting cancellation:
1. When a form submission with status "已取消" (Cancelled) is received
2. The system looks up the meeting and room IDs from the database using the entry token
3. First, it releases the booked meeting room
4. Then it cancels the meeting
5. Finally, it updates the database with the cancellation status

You can test this integration by sending a properly formatted payload to the `/webhook/form-submission` endpoint.

## Error Handling

The service includes:
- Proper error handling for API requests
- Timeout handling for long-running requests
- Structured logging
- CORS support for frontend integration

## Graceful Shutdown

The service implements graceful shutdown to ensure clean termination:

- Captures standard termination signals:
  - SIGINT (Ctrl+C) on all platforms
  - SIGTERM on Unix/Linux platforms (for Docker/Kubernetes orchestration)
- Allows in-progress requests to complete
- Logs shutdown events for monitoring
- Prevents connection interruptions during deployments
- Enables safer container orchestration

## Contributing

Please see [CLAUDE.md](./CLAUDE.md) for development guidelines and conventions.

## License

Copyright (c) 2025. All rights reserved.