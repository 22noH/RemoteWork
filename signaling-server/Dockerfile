# Build context: workspace root (docker build -f signaling-server/Dockerfile .)
FROM rust:1.80-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy proto files and signaling-server source
COPY proto/ ./proto/
COPY signaling-server/ ./signaling-server/

RUN cd signaling-server && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /workspace/signaling-server/target/release/signaling-server .

EXPOSE 8080

# No hard-coded LISTEN_ADDR: the binary binds $PORT if set (Render/Railway/Fly),
# else 0.0.0.0:8080 (deploy/ compose reaches it by service name on 8080).
ENV RUST_LOG=signaling_server=info

# ENTRYPOINT so compose/`docker run` can append flags (e.g. --insecure when TLS
# is terminated by a reverse proxy in front of it).
ENTRYPOINT ["./signaling-server"]
