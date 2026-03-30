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

# Compile the relayer-init and relayer-cli binaries in release mode
RUN cargo build --release --bin relayer-init --bin relayer-cli

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
COPY --from=builder /workspace/target/release/relayer-init /usr/local/bin/relayer-init
COPY --from=builder /workspace/target/release/relayer-cli /usr/local/bin/relayer-cli

# Copy the relayer program JSON and env example for reference
COPY --from=builder /workspace/relayerprogram.json /home/relayer/relayerprogram.json
COPY --from=builder /workspace/.env.example /home/relayer/.env.example

# Switch to the non-root user
USER relayer

# Default environment for testnet
ENV NYKS_LCD_BASE_URL=https://lcd.twilight.rest \
    NYKS_RPC_BASE_URL=https://rpc.twilight.rest \
    FAUCET_BASE_URL=https://faucet-rpc.twilight.rest \
    ZKOS_SERVER_URL=https://nykschain.twilight.rest/zkos \
    RELAYER_API_RPC_SERVER_URL=https://relayer.twilight.rest/api \
    RELAYER_PROGRAM_JSON_PATH=./relayerprogram.json \
    RUST_LOG=info

# Default to the CLI; override with `docker run <image> relayer-init` if needed
ENTRYPOINT ["relayer-cli"]