# Astrophoto

Web app for amateur astrophotographers to upload, tag, and share images.

- **Backend**: Rust (axum + sqlx + PostgreSQL)
- **Frontend**: SvelteKit (Svelte 5 runes, SSR)
- **Storage**: S3-compatible (Cloudflare R2 in prod, MinIO in dev)

## Quick start

```bash
# Prereqs: Docker, just, rustup, pnpm
cp .env.example .env
just dev
```

Open <http://localhost:5173>.

## Common commands

```bash
just              # list all commands
just check        # quality gates (fmt, clippy, types, lints)
just test         # run all tests (Docker required)
just db-reset     # drop + recreate dev db
```

## Repository

- `backend/` — Rust API.
- `frontend/` — SvelteKit app.
- `docs/superpowers/specs/` — design documents.
- `docs/superpowers/plans/` — implementation plans.
- `CLAUDE.md` — instructions for AI coding assistants.

## License

MIT.
