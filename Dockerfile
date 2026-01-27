# Single stage build - simpler and avoids copy issues
FROM rust:1.88-bookworm

WORKDIR /app

# Copy everything
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY static ./static
COPY admin_static ./admin_static
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# Cloud Run uses PORT env var
ENV PORT=8080

EXPOSE 8080

# Run directly from target
CMD ["./target/release/caterpillar-clay"]
