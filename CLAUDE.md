# CLAUDE.md - Guidelines for Tencent Meeting Service

## Project Commands

### Quick Test Environment
- Start test server: `docker compose -f docker-compose.test.yml up -d`
- Access test endpoint: `http://localhost:3001/test`
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

## API Endpoints
- `GET /health` - Health check endpoint
- `GET /test` - Test endpoint with mock data
- `GET /test-meetings` - Test endpoint with sample meeting creation/cancellation requests
- `GET /meeting-rooms?page=1&page_size=20` - Get meeting rooms with pagination
- `POST /meetings` - Create a new meeting
- `POST /meetings/{meeting_id}/cancel` - Cancel an existing meeting

## Required Environment Variables
```
# Required for Tencent Meeting API authentication
TENCENT_MEETING_APP_ID=your_app_id
TENCENT_MEETING_SECRET_ID=your_secret_id
TENCENT_MEETING_SECRET_KEY=your_secret_key
TENCENT_MEETING_SDK_ID=your_sdk_id
TENCENT_MEETING_OPERATOR_ID=your_operator_id
```

## Code Style Guidelines
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

## Tencent Meeting API Authentication
- **Authentication Module**: `auth.rs` contains the `TencentAuth` struct with authentication utilities
- **Authentication Method**: AKSK (AppId, SecretId, SecretKey) with HMAC-SHA256 signatures
- **TencentAuth Functions**:
  - `generate_signature`: Creates HMAC-SHA256 signatures for API requests
  - `generate_nonce`: Generates random 8-digit nonces for request uniqueness
  - `get_timestamp`: Provides current Unix timestamps
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