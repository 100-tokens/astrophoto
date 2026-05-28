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
    mkdir -p frontend/src/lib/api
    cd backend && cargo run --bin gen-types
    cd frontend && pnpm prettier --write 'src/lib/api/**/*.ts' 2>/dev/null || true

# Seed the dev database with demo content. Idempotent.
seed:
    cd backend && cargo run --bin seed

# Seed/refresh the targets table from the pinned OpenNGC CSVs.
# Idempotent. Run after migrations.
seed-targets:
    cd backend && cargo run --release --bin seed-targets

# Seed the targets table with PGC galaxies from HyperLEDA (idempotent).
# Requires backend/data/pgc/pgc.csv (or .csv.gz) — see backend/data/pgc/README.md.
seed-pgc:
    cd backend && cargo run --release --bin seed_pgc

# Resolve photos.target text against the catalog, insert manual photo_targets rows.
# Default is dry-run. Pass --apply to write. Idempotent.
backfill-photo-targets *args:
    cd backend && cargo run --release --bin backfill-photo-targets -- {{args}}

# Parse XISF headers and fill photos.processing_json for existing photos.
# Default is dry-run. Pass --apply to write. Idempotent.
backfill-processing *args:
    cd backend && cargo run --release --bin backfill-processing -- {{args}}
