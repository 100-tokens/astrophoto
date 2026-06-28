# Browser e2e is the production-readiness bar for critical user journeys

A *critical user journey* is considered production-ready only when a browser-driven
end-to-end test exercises it through the real stack (SvelteKit frontend → axum
backend → Postgres/S3/mail). The ~500 backend integration tests count as **API
coverage** but do **not** by themselves mark a critical journey ready, because they
can't catch SSR / `/api` reverse-proxy / cookie / form-action integration breaks
that only surface in the browser. Coverage and gaps are tracked in
`docs/operations/production-readiness-e2e.md`; the functional + mobile + a11y +
security e2e suite is wired as a per-PR CI gate (perf/load + cross-browser run
on-demand). Journeys gated by externals that can't run locally (Google OAuth,
plate-solving) are covered by backend tests + a manual staging/prod smoke
checklist rather than brittle browser mocks.

## Considered Options

- **API-only coverage** (rely on the backend integration suite) — rejected: misses
  UI↔backend integration regressions a go-live cares about.
- **Staging-only e2e** (run against the deployed env on a schedule) — rejected as the
  primary gate: slower feedback and pollutes staging data; kept as the home for
  perf/cross-browser on-demand runs.
