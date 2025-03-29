# Tencent Meeting Service

A web service that provides a bridge between form services and Tencent Meeting API. It authenticates with the Tencent Meeting enterprise API using AKSK (AppId, SecretId, SecretKey) authentication to provide meeting room information and scheduling functionality.

## Features

- RESTful API for meeting room data
- Secure authentication with Tencent Meeting API
- Docker-ready for easy deployment
- Production and development environments
- Mock testing endpoints
- Configurable via environment variables

## Project Structure

```
tencent_meeting_service/
├── Cargo.toml           # Project dependencies
├── .env                 # Environment configuration
├── Dockerfile           # Multi-stage Docker configuration
├── docker-compose.yml   # Docker Compose setup
├── CLAUDE.md            # Development guidelines
└── src/
    ├── main.rs          # Web server and API endpoints
    ├── client.rs        # Tencent Meeting API client
    ├── auth.rs          # Authentication utilities for Tencent Meeting API
    └── lib.rs           # Library exports
```

## API Endpoints

- `GET /health` - Health check endpoint
- `GET /test` - Test endpoint that returns mock meeting room data
- `GET /test-meetings` - Test endpoint with sample meeting creation/cancellation requests
- `GET /test-form-submission` - Test endpoint with sample form webhook payload
- `GET /meeting-rooms?page=1&page_size=20` - Get meeting rooms with pagination
- `POST /meetings` - Create a new meeting with Tencent Meeting API
- `POST /meetings/{meeting_id}/cancel` - Cancel an existing meeting
- `POST /webhook/form-submission` - Webhook endpoint for form submissions to create meetings

## Setup

1. Edit the `.env` file to add your Tencent Meeting API credentials:

```
# Required credentials
TENCENT_MEETING_APP_ID=your_app_id
TENCENT_MEETING_SECRET_ID=your_secret_id
TENCENT_MEETING_SECRET_KEY=your_secret_key
TENCENT_MEETING_SDK_ID=your_sdk_id
TENCENT_MEETING_OPERATOR_ID=your_operator_id

# Form field mappings (required)
FORM_USER_FIELD_NAME=user_field_name
FORM_DEPT_FIELD_NAME=department_field_name

# Optional settings
TENCENT_MEETING_API_ENDPOINT=https://api.meeting.qq.com
RUST_LOG=info
```

## Quick Test

The simplest way to test the service is using the provided test Docker configuration:

```bash
docker compose -f docker-compose.test.yml up -d
```

The test server will be available at `http://localhost:3001`. You can access the mock endpoint at:
- `http://localhost:3001/test` - Returns mock meeting room data
- `http://localhost:3001/health` - Health check

## Development with Docker

Start the development environment:

```bash
docker compose up -d dev
```

In development mode, the application automatically reloads when you make changes to the code.

## Production Deployment

Start the production environment:

```bash
docker compose up -d app
```

The service will be available at `http://localhost:3000`.

## Authentication Method

This service implements the AKSK (AppId, SecretId, SecretKey) authentication method for Tencent Meeting API. The authentication logic is encapsulated in the `auth.rs` module, which provides utilities for generating signatures, timestamps, and nonces for API requests following Tencent's specifications.

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

## Form Service Integration

The service is designed to be integrated with form services, allowing users to:

1. View available meeting rooms
2. Schedule meetings in these rooms through form submissions
3. Check meeting room availability

### Form Webhook Integration

The service includes a webhook endpoint (`/webhook/form-submission`) that accepts form submissions and automatically creates meetings in Tencent Meeting. The webhook expects the following JSON structure:

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
- The operator_id from environment variables is used as the meeting creator/host
- Department and room name are used for the meeting location
- Meeting instance ID is set to 32 (as required by the API)

When multiple time slots are submitted in a single form:
1. The service attempts to find all mergeable groups of time slots
2. For each mergeable group:
   - If the group has multiple time slots that are contiguous and in the same room, they are merged into a single meeting
   - If the group has only one time slot, a single meeting is created for it
3. The response includes details for all created meetings, indicating:
   - Which time slots were merged
   - Which room was used for each meeting
   - Success/failure status for each meeting
   - Meeting IDs for successfully created meetings

You can test this integration using the `/test-form-submission` endpoint that provides a sample payload.

## Error Handling

The service includes:
- Proper error handling for API requests
- Timeout handling for long-running requests
- Structured logging
- CORS support for frontend integration

## Contributing

Please see [CLAUDE.md](./CLAUDE.md) for development guidelines and conventions.

## License

Copyright (c) 2025. All rights reserved.