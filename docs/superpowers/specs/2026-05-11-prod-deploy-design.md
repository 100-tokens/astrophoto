# Production deploy stream

**Status:** approved, ready for implementation plan
**Date:** 2026-05-11
**Owner:** Pascal Le Clech

## Goal

Stand up the production environment for Astrophoto on the existing staging
provider stack (Koyeb for compute and database, AWS for image storage +
CDN), with `astrophoto.pics` resolving via Vercel DNS. Tag-based manual
promotion gates what ships. Production must launch with the same CDN
architecture used in the design (CloudFront + Lambda@Edge), not the
backend `/cdn/img/` fallback.

## Non-goals

- Migrating staging or any other environment.
- Standing up email (MX) records at Vercel.
- Adding a CI workflow that auto-creates release tags (a possible
  follow-up, explicitly deferred).
- Multi-region failover or hot-standby — single region, single
  instance is sufficient for launch.
- Replacing the existing AWS-S3 + CloudFront design from
  `docs/operations/aws-s3-cloudfront.md`.

## Topology

```
        astrophoto.pics                        cdn.astrophoto.pics
        www.astrophoto.pics                          │
              │                                      │
              │ A / CNAME (Vercel DNS)               │ CNAME (Vercel DNS)
              ▼                                      ▼
   ┌────────────────────────┐               ┌──────────────────┐
   │  Koyeb: web (SvelteKit)│               │  CloudFront      │
   │  astrophoto-prod-web   │               │  + Lambda@Edge   │
   └──────────┬─────────────┘               │  (sharp resize)  │
              │ HTTPS                       └────────┬─────────┘
              ▼                                      │
   ┌────────────────────────┐                        ▼
   │ Koyeb: api (Rust/axum) │ ──── presigned ───►  S3: astrophoto-images-prod
   │ api.astrophoto.pics    │      PUT
   └──────────┬─────────────┘
              │ TLS (internal)
              ▼
   ┌────────────────────────┐
   │ Koyeb Managed Postgres │
   │  astrophoto-prod-db    │
   └────────────────────────┘
```

The backend gets its own subdomain (`api.astrophoto.pics`) so the
SameSite=None session-cookie behaviour stays bounded to a single CORS
origin (`APP_CORS_ORIGIN=https://astrophoto.pics`). Vercel never sits in
the request path — only authoritative DNS for the zone.

## DNS at Vercel

Vercel hosts the `astrophoto.pics` zone. Records (exact targets resolved
at provisioning time from Koyeb / CloudFront / ACM):

| record                 | type  | target                                          | purpose                              |
|------------------------|-------|-------------------------------------------------|--------------------------------------|
| `astrophoto.pics`      | A     | Koyeb anycast IPs (from Koyeb docs)             | Apex → web service                   |
| `www.astrophoto.pics`  | CNAME | `astrophoto.pics`                               | Redirect to apex (handled by Koyeb)  |
| `api.astrophoto.pics`  | CNAME | `astrophoto-prod-<org>.koyeb.app`               | Backend API                          |
| `cdn.astrophoto.pics`  | CNAME | `<dist-id>.cloudfront.net`                      | Image CDN                            |
| `_acme-challenge.*`    | CNAME | (per ACM and Koyeb domain-verification flows)   | Cert + domain validation             |

Rationale:

- Apex uses A records (not Vercel's CNAME-flattening) so the design is
  not coupled to Vercel-specific behaviour. If we ever move DNS, the
  records port cleanly.
- The ACM cert for `cdn.astrophoto.pics` lives in `us-east-1` (CloudFront
  requirement) and is DNS-validated against the Vercel zone.
- Koyeb provisions Let's Encrypt certificates automatically for
  `astrophoto.pics` and `api.astrophoto.pics` once domain ownership is
  verified.

## Koyeb production services

Three Koyeb resources under a `prod` environment, naming-parallel to
staging:

### `astrophoto-prod-web` (SvelteKit, SSR)

- Source: GitHub repo, deploy filter = git tags matching `v*`.
- Buildpack: Node 22. Build: `pnpm install --frozen-lockfile && pnpm build`.
  Run: `node build/index.js`.
- Public domains: `astrophoto.pics`, `www.astrophoto.pics` (redirect).
- Instance: `nano` (1 vCPU, 512 MB) to start; one instance.
- Health: HTTP `GET /` returns 200.

### `astrophoto-prod` (Rust backend)

- Source: same repo, same tag filter.
- Build: Dockerfile (same image as staging — keep identical).
- Public domain: `api.astrophoto.pics`.
- Instance: `small` (1 vCPU, 1 GB) — image decode runs in
  `spawn_blocking` and is CPU-bound.
- Health: HTTP `GET /healthz` returns 200.

### `astrophoto-prod-db` (Koyeb managed Postgres)

- Smallest paid Koyeb Postgres plan (free tier lacks PITR and capacity).
- PostgreSQL 16 (same major as staging).
- Connection string injected into the backend via
  `${{ astrophoto-prod-db.DATABASE_URL }}` reference.
- Backups: daily automated, 7-day retention (Koyeb default on paid plans).

Both compute services run in the same Koyeb region. Region choice
(Frankfurt vs Washington) follows the staging region — to be confirmed
during implementation.

## AWS production infrastructure

Follows `docs/operations/aws-s3-cloudfront.md` with `prod` substituted
for `staging`:

- **S3 bucket** `astrophoto-images-prod` in `us-east-1`. Public access
  blocked. CORS allows `https://astrophoto.pics` and
  `https://www.astrophoto.pics`. Lifecycle: `originals/` → Glacier after
  90 days; `display/` stays in Standard.
- **IAM user** `astrophoto-prod-uploader` with the minimal-privilege
  policy used in staging (PUT/HEAD on `originals/*`, GET on `display/*`).
- **ACM certificate** for `cdn.astrophoto.pics` (+ optional SAN
  `*.astrophoto.pics`) in `us-east-1`, DNS-01 validated against the
  Vercel zone.
- **Lambda@Edge** (origin-request trigger) — sharp-based transform,
  identical to staging. Deployed to `us-east-1` and associated with the
  prod distribution.
- **CloudFront distribution** with origin = the S3 bucket, behaviour =
  forward `width`, `height`, `format` query strings to the Lambda@Edge
  function, viewer protocol policy = redirect-to-HTTPS, alias =
  `cdn.astrophoto.pics`.

The runbook is authoritative for exact CLI commands. This spec only
names the resources.

## Promotion flow

Tag-based, deliberately low-tech:

```
   developer ──► git push main ──► Koyeb staging auto-deploy
                       │
                       │ smoke-tested on staging
                       ▼
   developer ──► git tag v2026.05.11 && git push --tags
                       │
                       ▼
        Koyeb prod (web + api) detects new tag matching `v*`
                       │
                       ▼
           builds, runs migrations on boot, swaps in new revision
```

- Both prod services share the same `v*` tag filter so they ship as a pair.
- Koyeb runs the new revision in parallel; traffic switches only after
  health checks pass; a failing check reverts to the previous revision.
- Tagging is a human action. No CI gate is added here. If we later want
  to gate tagging on green CI, that's a small follow-up (a GitHub
  Actions workflow that creates the tag only after `just check` and
  the test suite pass).

## Secrets and environment

Each prod Koyeb service has its own env. Secrets are created once via
`koyeb secret create` and referenced by name. Every prod secret is
**freshly generated** — nothing copied from staging.

| secret name                          | service | source                                            |
|--------------------------------------|---------|---------------------------------------------------|
| `prod_database_url`                  | api     | Koyeb DB service reference                        |
| `prod_aws_access_key_id`             | api     | `astrophoto-prod-uploader` IAM credentials        |
| `prod_aws_secret_access_key`         | api     | same                                              |
| `prod_session_signing_key`           | api     | freshly generated 32-byte key                     |
| `prod_google_oauth_client_secret`    | api     | new Google OAuth client for `https://astrophoto.pics` |
| `prod_postmark_token`                | api     | new transactional-email token                     |

Plain (non-secret) env vars on the backend:

- `APP_ENV=prod`
- `APP_BASE_URL=https://astrophoto.pics`
- `APP_CORS_ORIGIN=https://astrophoto.pics`
- `APP_S3_BUCKET=astrophoto-images-prod`
- `APP_S3_REGION=us-east-1`
- `APP_S3_PATH_STYLE=false`
- `APP_S3_ENDPOINT` — **unset** (use AWS default endpoints)
- `APP_CDN_BASE_URL=https://cdn.astrophoto.pics`
- `APP_CDN_LOCAL_FALLBACK` — **unset** (CloudFront is live; no backend fallback)

Plain env vars on the frontend:

- `PUBLIC_API_BASE_URL=https://api.astrophoto.pics`
- `PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics`

The session signing key in particular must differ from staging: if a
staging cookie ever leaked, it must not be forgeable in prod.

## First-deploy bootstrap

Done once, in this order:

1. Provision AWS resources (S3 bucket, IAM user, ACM certificate,
   Lambda@Edge function, CloudFront distribution) per the runbook.
2. Create Koyeb prod Postgres instance; capture connection string into
   `prod_database_url`.
3. Create a prod Google OAuth client with
   `https://api.astrophoto.pics/auth/google/callback` as the redirect URI.
4. Create the two Koyeb prod services pointing at the `v*` tag filter,
   with env + secrets wired in. Do **not** route public traffic yet —
   leave domains unverified.
5. At Vercel, add the DNS records from the DNS section.
6. Verify each Koyeb domain (Koyeb walks through the verification
   record). Koyeb provisions Let's Encrypt automatically.
7. Tag a release: `git tag v2026.05.11 && git push --tags`. Watch the
   Koyeb build.
8. On first boot, `sqlx::migrate!()` runs against the empty prod
   database; no manual migration step required.
9. Smoke test in this order, before announcing the URL:
   signup → email confirm → Google OAuth → upload (presigned PUT to
   prod bucket) → finalize → photo viewer (confirms the CloudFront
   transform path) → delete.

## Rollback

Two layers:

- **Automatic.** Koyeb runs the new revision in parallel and only routes
  traffic once health checks pass. A failing `/healthz` reverts to the
  prior revision with no operator action.
- **Manual.** "Redeploy" against the previous tag in the Koyeb dashboard,
  or `koyeb service redeploy astrophoto-prod --deployment <id>`.
  Migrations are append-only and forward-only — rollback reverts code,
  never schema. This matches the standard sqlx contract used elsewhere
  in the codebase.

For a *bad migration* specifically: roll forward with a compensating
migration tagged `v*-hotfix`. Never edit a merged migration.

## Open questions

None blocking. The two judgement calls to lock in during implementation:

- Koyeb region for prod compute (Frankfurt vs Washington) — to mirror
  staging unless we have a clearer audience signal.
- Whether to keep `www.astrophoto.pics` as an apex redirect or drop it
  entirely. Default: keep, with Koyeb handling the 301.

## References

- `docs/operations/aws-s3-cloudfront.md` — the authoritative runbook for
  S3 + CloudFront + Lambda@Edge provisioning. This spec defers to it
  for exact CLI invocations.
- `CLAUDE.md` "Gotchas" section — the session-cookie SameSite rules,
  presigned-PUT signature behaviour, and CDN fallback flag semantics
  are all load-bearing for the prod env-var choices above.
