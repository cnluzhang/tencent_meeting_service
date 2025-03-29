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
    └── lib.rs           # Library exports
```

## API Endpoints

- `GET /health` - Health check endpoint
- `GET /test` - Test endpoint that returns mock meeting room data
- `GET /meeting-rooms?page=1&page_size=20` - Get meeting rooms with pagination

## Setup

1. Edit the `.env` file to add your Tencent Meeting API credentials:

```
# Required credentials
TENCENT_MEETING_APP_ID=your_app_id
TENCENT_MEETING_SECRET_ID=your_secret_id
TENCENT_MEETING_SECRET_KEY=your_secret_key
TENCENT_MEETING_SDK_ID=your_sdk_id
TENCENT_MEETING_OPERATOR_ID=your_operator_id

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

This service implements the AKSK (AppId, SecretId, SecretKey) authentication method for Tencent Meeting API. It generates the required signatures for API requests following Tencent's specifications:

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
2. Schedule meetings in these rooms
3. Check meeting room availability

Future releases will include more endpoints for scheduling meetings, managing attendees, and other meeting-related operations.

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