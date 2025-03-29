# Build stage
FROM rust:1.81-slim-bookworm as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy only Cargo.toml first to cache dependencies
COPY Cargo.toml .

# Create dummy source files to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub mod client;" > src/lib.rs && \
    touch src/client.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/tencent_meeting_service /app/tencent_meeting_service

# Copy .env file if it exists
COPY .env* /app/

# Expose the port the app will run on
EXPOSE 3000

# Command to run the application
CMD ["./tencent_meeting_service"]