# syntax=docker/dockerfile:1

#############################
# Build stage
#############################
FROM rust:1.87 AS builder

# Install build dependencies (protoc is required by build.rs)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create app directory inside the image
WORKDIR /workspace

# Copy source code
COPY . .

# Compile only the relayer_init binary in release mode
RUN cargo build --release --bin relayer-init

#############################
# Runtime stage
#############################
FROM debian:bookworm-slim AS runtime

# Install CA certificates so HTTPS works
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create an unprivileged user to run the binary
RUN useradd -m relayer

# Set work directory to user home; this is where the JSON will be written
WORKDIR /home/relayer

# Copy the built binary from the builder stage
COPY --from=builder /workspace/target/release/relayer-init /usr/local/bin/relayer-init

# Switch to the non-root user
USER relayer

# The program expects several environment variables.
# You can provide them at `docker run` time.
# Example:
#   docker run -e ZKOS_SERVER_URL=https://zkos-rpc.twilight.rest ... relayer-init

ENTRYPOINT ["relayer-init"] 