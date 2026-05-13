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

Staging uses two separate Koyeb apps (one per service). Prod mirrors
that — three resources, `was` region (matching staging):

### App `astrophoto-prod-web`, service `frontend` (SvelteKit, SSR)

- Source: GitHub repo, deploy filter = git tags matching `v*`.
- Build: `Dockerfile.frontend`.
- Public domains: `astrophoto.pics`, `www.astrophoto.pics` (redirect).
- Instance: `nano` (1 vCPU, 512 MB).
- Port: 3000. Health: implicit port-open (matches staging — no explicit
  HTTP health check configured at Koyeb level).

### App `astrophoto-prod`, service `backend` (Rust API)

- Source: same repo, same tag filter.
- Build: `Dockerfile.backend`.
- Public domain: `api.astrophoto.pics`.
- Instance: `nano` (1 vCPU, 512 MB) — staging runs the same instance
  type and image decode under `spawn_blocking` has been fine. Bumpable
  if hot-path latency or OOMs appear.
- Port: 8080. Health: implicit port-open.

### Database `astrophoto-prod-pg` (Koyeb managed Postgres, under app `astrophoto-prod`)

- Smallest paid Koyeb Postgres plan (free tier lacks PITR and capacity).
  Plan picked: `small`.
- PostgreSQL 16 (same major as staging).
- Connection string captured into a Koyeb secret
  `astrophoto-prod-db-url` (matching the staging pattern of
  `astrophoto-staging-db-url`). The backend reads it via
  `APP_DATABASE_URL`.
- Backups: daily automated, 7-day retention (Koyeb default on paid plans).

## AWS production infrastructure

Follows `docs/operations/aws-s3-cloudfront.md` with `prod` substituted
for `staging`, plus an SES setup that staging never had:

- **S3 bucket** `astrophoto-images-prod` in `us-east-1`. Public access
  blocked. CORS allows `https://astrophoto.pics` and
  `https://www.astrophoto.pics`. Lifecycle: abort stale multipart
  uploads after 1 day. (The originals → Glacier lifecycle from the
  runbook is deferred; staging doesn't apply it either.)
- **IAM user** `astrophoto-prod-uploader` with the minimal-privilege
  policy used in staging (PUT/HEAD on `originals/*`, GET on `display/*`).
- **ACM certificate** for `cdn.astrophoto.pics` (+ SAN
  `*.astrophoto.pics`) in `us-east-1`, DNS-01 validated against the
  Vercel zone.
- **CAA exception** at the Vercel apex: `0 issue "amazon.com"` must be
  added alongside the existing Vercel-default CAA records (which only
  list `pki.goog`, `sectigo.com`, `letsencrypt.org`), or ACM cannot
  issue.
- **Lambda@Edge** `astrophoto-prod-image-transformer` (origin-request
  trigger) — sharp-based transform, identical to staging except the
  `BUCKET` constant. Deployed to `us-east-1`, executed via role
  `astrophoto-prod-lambda-exec` that trusts both `lambda.amazonaws.com`
  and `edgelambda.amazonaws.com`.
- **CloudFront distribution** with origin = the S3 bucket via OAC
  `astrophoto-prod-s3-oac`, cache policy `astrophoto-prod-img-cp`
  whitelisting `w/h/fit/q/fm` query strings, viewer protocol policy =
  redirect-to-HTTPS, alias = `cdn.astrophoto.pics`.
- **SES (us-east-1)** for transactional email: domain identity for
  `astrophoto.pics` with three DKIM CNAMEs at Vercel; dedicated IAM
  user `astrophoto-prod-ses-smtp` with inline policy granting
  `ses:SendRawEmail` + `ses:SendEmail`; SMTP credentials derived from
  the IAM access key via SigV4 HMAC and stored as Koyeb secrets
  `astrophoto-prod-smtp-username` and `astrophoto-prod-smtp-password`.
  The account is already out of SES sandbox (50k/day quota).

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

Source of truth: `backend/src/config.rs` (the `Config` struct that
parses `APP_*` env vars) and the staging Koyeb deployment definition.
No `APP_SESSION_SIGNING_KEY` exists — the backend uses opaque DB-backed
sessions, so there is no signing key to rotate per environment.

### Koyeb secrets (created via `koyeb secrets create`)

| secret name                                    | source                                                     |
|------------------------------------------------|------------------------------------------------------------|
| `astrophoto-prod-db-url`                       | Connection string from `astrophoto-prod-pg`                |
| `astrophoto-prod-s3-access-key`                | `astrophoto-prod-uploader` IAM access-key ID               |
| `astrophoto-prod-s3-secret-key`                | `astrophoto-prod-uploader` IAM secret access key           |
| `astrophoto-prod-oauth-google-client-id`       | New prod Google OAuth client (separate from staging)       |
| `astrophoto-prod-oauth-google-client-secret`   | Same client                                                |
| `astrophoto-prod-smtp-username`                | `astrophoto-prod-ses-smtp` IAM access-key ID               |
| `astrophoto-prod-smtp-password`                | SMTP password derived from the IAM secret via SigV4 HMAC   |

### Backend env (service `astrophoto-prod/backend`)

Plain values:

- `APP_BIND=0.0.0.0:8080`
- `APP_LOG=info`
- `APP_PUBLIC_BASE_URL=https://astrophoto.pics`
- `APP_CORS_ORIGIN=https://astrophoto.pics`
- `APP_CDN_BASE_URL=https://cdn.astrophoto.pics`
- `APP_CDN_LOCAL_FALLBACK=false`
- `APP_S3_BUCKET=astrophoto-images-prod`
- `APP_S3_REGION=us-east-1`
- `APP_S3_PATH_STYLE=false`
- `APP_SESSION_DOMAIN=""` (host-only cookie on `api.astrophoto.pics`; works because `SECURE=true` triggers `SameSite=None`)
- `APP_SESSION_SECURE=true`
- `APP_OAUTH_GOOGLE_REDIRECT_URL=https://api.astrophoto.pics/api/auth/oauth/google/callback`
- `APP_SMTP_HOST=email-smtp.us-east-1.amazonaws.com`
- `APP_SMTP_PORT=587`
- `APP_SMTP_TLS=true`
- `APP_MAIL_FROM=Astrophoto <noreply@astrophoto.pics>`

Secret references:

- `APP_DATABASE_URL` ← `astrophoto-prod-db-url`
- `APP_S3_ACCESS_KEY` ← `astrophoto-prod-s3-access-key`
- `APP_S3_SECRET_KEY` ← `astrophoto-prod-s3-secret-key`
- `APP_OAUTH_GOOGLE_CLIENT_ID` ← `astrophoto-prod-oauth-google-client-id`
- `APP_OAUTH_GOOGLE_CLIENT_SECRET` ← `astrophoto-prod-oauth-google-client-secret`
- `APP_SMTP_USER` ← `astrophoto-prod-smtp-username`
- `APP_SMTP_PASS` ← `astrophoto-prod-smtp-password`

### Frontend env (service `astrophoto-prod-web/frontend`)

No secrets — talks to the backend via public HTTPS only:

- `HOST=0.0.0.0`
- `PORT=3000`
- `NODE_ENV=production`
- `ORIGIN=https://astrophoto.pics`
- `BACKEND_URL=https://api.astrophoto.pics`
- `PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics`

## First-deploy bootstrap

Done once, in this order:

1. Provision AWS resources: S3 bucket, S3 uploader IAM user, prod
   Lambda execution role, prod Lambda function + version, ACM cert
   (gated on the CAA record at Vercel), CloudFront OAC + cache policy
   + distribution, bucket policy granting OAC read on `display/*`.
2. Provision SES: domain identity for `astrophoto.pics`, three DKIM
   CNAMEs at Vercel, dedicated IAM user `astrophoto-prod-ses-smtp`
   with inline `ses:SendRawEmail` policy, access key, SMTP password
   derived via SigV4 HMAC.
3. Provision Koyeb: two apps (`astrophoto-prod` + `astrophoto-prod-web`),
   one managed Postgres `astrophoto-prod-pg` under
   `astrophoto-prod` (region `was`, plan `small`, PostgreSQL 16).
4. Create a prod Google OAuth client with redirect URI
   `https://api.astrophoto.pics/api/auth/oauth/google/callback`.
5. Create the seven Koyeb secrets listed above.
6. Create the two Koyeb services (`backend`, `frontend`) with their
   env wired, deploy filter set to a non-existent placeholder tag
   (e.g. `v0.0.0-pending`) so they don't deploy until DNS is in place.
7. At Vercel, add the public DNS records (apex A, www CNAME, api
   CNAME, cdn CNAME, and Koyeb verification CNAMEs).
8. Verify each Koyeb custom domain. Koyeb auto-provisions Let's
   Encrypt certs once verified.
9. Switch the Koyeb tag filter from the placeholder to `v*`, then
   `git tag v2026.05.12 && git push --tags`. Watch the build.
10. On first boot, `sqlx::migrate!()` runs against the empty prod
    DB; no manual migration step.
11. Smoke test in order before announcing the URL:
    signup → email confirm (SES delivery) → Google OAuth → upload
    (presigned PUT to prod bucket) → finalize → photo viewer
    (confirms the CloudFront transform path) → delete.

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

Resolved during implementation:

- Koyeb region: `was` (Washington), matching staging.
- `www.astrophoto.pics`: kept as redirect to apex, Koyeb handles 301.
- Email provider: Amazon SES (us-east-1), already production-tier on
  this AWS account.

## References

- `docs/operations/aws-s3-cloudfront.md` — the authoritative runbook for
  S3 + CloudFront + Lambda@Edge provisioning. This spec defers to it
  for exact CLI invocations.
- `CLAUDE.md` "Gotchas" section — the session-cookie SameSite rules,
  presigned-PUT signature behaviour, and CDN fallback flag semantics
  are all load-bearing for the prod env-var choices above.
