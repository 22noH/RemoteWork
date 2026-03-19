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

ENV LISTEN_ADDR=0.0.0.0:8080
ENV RUST_LOG=signaling_server=info

CMD ["./signaling-server"]
