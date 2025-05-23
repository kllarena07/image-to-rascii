# Build stage
FROM rust:bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

# Final run stage
FROM debian:bookworm-slim AS runner

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/image-to-rascii /app/image-to-rascii
CMD ["/app/image-to-rascii"]
