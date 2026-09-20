# ============================================================
# Backend — Multi-Stage Rust Build
# ============================================================
# Stage 1: Build the Rust release binary
# Stage 2: Minimal runtime image (non-root)
# ============================================================

# ── Stage 1: Build ────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ── Dependency caching layer ──────────────────────────────────
# Copy only the manifest files first so that dependencies are
# compiled and cached independently of application source code.
COPY Cargo.toml Cargo.toml

# Create a dummy main.rs to allow `cargo build` to compile dependencies
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs

# Build dependencies only (cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src

# ── Application source ────────────────────────────────────────
COPY src ./src
COPY migrations ./migrations

# Touch main.rs to force recompilation of our code (not deps)
RUN touch src/main.rs

# Build the final release binary
RUN cargo build --release

# ── Stage 2: Runtime ──────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install only what the binary needs at runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd --gid 1001 dhp && \
    useradd --uid 1001 --gid dhp --shell /bin/sh --create-home dhp

WORKDIR /app

# Copy the compiled binary from the build stage
COPY --from=builder /app/target/release/backend /app/backend

# Copy migrations (SQLx reads these at runtime)
COPY --from=builder /app/migrations /app/migrations

# Set ownership
RUN chown -R dhp:dhp /app

# Switch to non-root user
USER dhp

# Expose the application port
EXPOSE 8080

# Health check using the /health endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the backend binary
ENTRYPOINT ["/app/backend"]
