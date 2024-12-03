# Build Stage
FROM rust:1.79 AS builder

WORKDIR /usr/src

RUN apt-get update \
	&& apt-get install -y openssl ca-certificates tzdata libcurl4 libpq-dev ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

# 1. Create a new empty shell project
RUN USER=root cargo new --bin auth-webhook

# 2. Copy our manifests
WORKDIR /usr/src/auth-webhook
COPY Cargo.toml Cargo.lock ./
COPY ./crates ./crates

# 3. Build for release
RUN cargo build --release --package auth-webhook

# Runtime Stage
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && \
	apt-get install -y openssl

EXPOSE 3050

COPY --from=builder /usr/src/auth-webhook/target/release/auth-webhook /usr/bin

CMD ["/usr/bin/auth-webhook"]