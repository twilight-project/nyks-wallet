# syntax=docker/dockerfile:1

#############################
# Build stage
#############################
FROM rust:1.89 AS builder

# Install build dependencies (protoc is required by build.rs)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create app directory inside the image
WORKDIR /workspace

# Copy source code
COPY . .

# Compile the relayer-init and relayer-cli binaries in release mode
RUN cargo build --release --bin relayer-cli

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

# Copy the built binaries from the builder stage
COPY --from=builder /workspace/target/release/relayer-cli /usr/local/bin/relayer-cli

# Switch to the non-root user
USER relayer
ENV RUST_LOG=info

# The program expects several environment variables.
# You can provide them at `docker run` time.
# Example:
#   docker run -e ZKOS_SERVER_URL=https://zkos-rpc.twilight.rest ... relayer-init

ENTRYPOINT ["relayer-cli"] 