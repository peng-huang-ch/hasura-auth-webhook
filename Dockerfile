# Build Stage
FROM rust:1.79 AS builder

WORKDIR /usr/src

RUN apt-get update \
	&& apt-get install -y openssl ca-certificates tzdata libcurl4 libpq-dev ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

# 1. Create a new empty shell project
RUN USER=root cargo new --bin hasura-auth-webhook

# 2. Copy our manifests
WORKDIR /usr/src/hasura-auth-webhook
COPY Cargo.toml Cargo.lock ./

# 3. Build only the dependencies to cache them
RUN cargo build --release

# 4. Now that the dependency is built, copy your source code
# Copy actual source and rebuild
COPY src src/

# 5. Build for release.
RUN rm ./target/release/deps/hasura_auth_webhook*
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
	&& apt-get install -y openssl ca-certificates tzdata libcurl4 libpq-dev ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

EXPOSE 3050

COPY --from=builder /usr/src/hasura-auth-webhook/target/release/hasura-auth-webhook /usr/bin

CMD ["/usr/bin/hasura-auth-webhook"]