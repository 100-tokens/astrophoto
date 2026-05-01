set dotenv-load := true

# Default — list all commands.
default:
    @just --list

# Start full dev stack (postgres, minio, backend, frontend).
dev:
    docker compose up -d postgres minio
    sleep 2
    just _migrate
    (cd backend && cargo run) & \
    (cd frontend && pnpm dev) & \
    wait

# Run all quality gates. MUST pass before claiming a task done.
check:
    cd backend && cargo fmt --check
    cd backend && cargo clippy --all-targets -- -D warnings
    cd frontend && pnpm check
    cd frontend && pnpm lint

# Run all tests. Docker must be running (testcontainers).
test:
    cd backend && cargo test --all-targets
    cd frontend && pnpm test

# Format Rust + frontend.
fmt:
    cd backend && cargo fmt
    cd frontend && pnpm format

# Drop and recreate the dev database, apply migrations.
db-reset:
    docker compose exec -T postgres psql -U astrophoto -d postgres \
        -c "drop database if exists astrophoto;" \
        -c "create database astrophoto;"
    just _migrate

# Create a new timestamped migration file: just db-migrate add_users
db-migrate name:
    cd backend && sqlx migrate add {{name}}

# Apply pending migrations (internal).
_migrate:
    cd backend && sqlx migrate run

# Regenerate TypeScript types from Rust source.
types:
    cd backend && cargo run --bin gen-types > ../frontend/src/lib/api/types.ts
    cd frontend && pnpm prettier --write src/lib/api/types.ts
