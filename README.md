# Astrophoto

Web app for amateur astrophotographers to upload, tag, and share images.

- **Backend**: Rust (axum + sqlx + PostgreSQL)
- **Frontend**: SvelteKit (Svelte 5 runes, SSR)
- **Storage**: AWS S3 in prod/staging, MinIO in dev; CloudFront + Lambda@Edge for image transforms

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

## Routes

| route                    | description                              |
|--------------------------|------------------------------------------|
| `/`                      | Home / explore feed                      |
| `/signup`, `/signin`     | Auth                                     |
| `/upload`                | Upload flow (presigned PUT → finalize → publish) |
| `/u/<handle>`            | Photographer profile page                |
| `/u/<handle>/p/<short-id>` | Single photo permalink               |

**Upload flow:** `/upload` → backend issues a presigned S3 PUT URL →
browser uploads directly to S3 → frontend calls `/api/photos/finalize`
→ backend generates display master in `spawn_blocking` → photo
published.

## Repository

- `backend/` — Rust API.
- `frontend/` — SvelteKit app.
- `docs/superpowers/specs/` — design documents.
- `docs/superpowers/plans/` — implementation plans.
- `docs/operations/` — infra runbooks.
- `CLAUDE.md` — instructions for AI coding assistants.

## License

MIT.
