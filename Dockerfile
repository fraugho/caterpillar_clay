# Stage 1: Generate dependency recipe
FROM rust:1.88-bookworm AS planner
RUN cargo install cargo-chef
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (CACHED LAYER)
FROM rust:1.88-bookworm AS cacher
RUN cargo install cargo-chef
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build application (uses cached deps)
FROM rust:1.88-bookworm AS builder
WORKDIR /app
# Copy cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
# Copy source
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

# Stage 4: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/caterpillar-clay .
COPY static ./static
COPY admin_static ./admin_static
COPY migrations ./migrations

ENV PORT=8080
EXPOSE 8080
CMD ["./caterpillar-clay"]
