# Multi-stage build for minimal runtime image
FROM rust:1.94-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /build

COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies (avoid rebuilding on code changes)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src

# Touch main.rs to force rebuild after dummy cache
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest

RUN apk add --no-cache ca-certificates

# Security: run as non-root user
RUN addgroup -S gateway && adduser -S -G gateway gateway

WORKDIR /app

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/sec-gateway .

RUN chown -R gateway:gateway /app

USER gateway

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

CMD ["./sec-gateway"]
