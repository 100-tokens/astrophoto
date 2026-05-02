# Phase 7 — Engagement Layer Design

**Date:** 2026-05-02
**Status:** Approved (sections 1–3) — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Add the social engagement layer designated as "Phase 2" in the original
design brief: appreciations (single-tap ♥, count, no public list),
comments (1-level threading, photo owner moderates), follows (asymmetric),
and a following feed on the authenticated home. The home for a logged-in
user with no follows degrades gracefully to the public gallery (the
designed behavior — never present an empty page).

Deferred to a later phase: notifications, comment edit, likes-on-comments,
direct messages, block/mute.

## Decisions

| # | Topic                              | Choice                                                              |
|---|------------------------------------|---------------------------------------------------------------------|
| 1 | Scope                              | Trio + feed: appreciations, comments, follows, following feed       |
| 2 | Empty following feed               | Fallback to public gallery (no empty state)                         |
| 3 | Comment threading                  | 1-level only (flat, no `parent_comment_id` column)                  |
| 4 | Comment edit                       | Out of scope (delete + repost works for now)                        |
| 5 | Comment moderation                 | Author can delete own; photo owner can delete any comment           |
| 6 | Appreciation public list           | None — count only, like the design brief specifies                  |
| 7 | Self-follow                        | Forbidden at DB level (`check (follower_id <> followed_id)`)        |
| 8 | Follow on own profile              | Hide the Follow button entirely on self-profile                     |
| 9 | Counts: dénormalised vs query      | Query at read time with FK indices; dénormalise only if slow later  |
| 10| `auth/me` extension                | Include `following_ids: Vec<Uuid>` (cap 500), drives client buttons |
| 11| Photo detail extension             | Add `appreciation_count`, `comment_count` to the JSON response       |
| 12| `is_appreciated` for current user  | Separate auth-required endpoint `GET /api/photos/:id/appreciation-state` |
| 13| Module layout                       | `backend/src/engagement/{appreciations,comments,follows}.rs`        |
| 14| Per-feature integration tests      | 3 tests in `backend/tests/engagement.rs`                            |

## Migration `0002_engagement.sql`

```sql
-- Phase 7: appreciations, comments, follows.

create table appreciations (
    user_id  uuid not null references users(id) on delete cascade,
    photo_id uuid not null references photos(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (user_id, photo_id)
);
create index appreciations_photo_id_idx on appreciations (photo_id);

create table comments (
    id          uuid primary key default gen_random_uuid(),
    photo_id    uuid not null references photos(id) on delete cascade,
    author_id   uuid not null references users(id) on delete cascade,
    body        text not null check (length(body) between 1 and 2000),
    created_at  timestamptz not null default now()
);
create index comments_photo_created_idx on comments (photo_id, created_at);

create table follows (
    follower_id uuid not null references users(id) on delete cascade,
    followed_id uuid not null references users(id) on delete cascade,
    created_at  timestamptz not null default now(),
    primary key (follower_id, followed_id),
    check (follower_id <> followed_id)
);
create index follows_followed_idx on follows (followed_id);
```

Notes:
- No dénormalised count columns. Counts come from `count(*) where ...`
  on the FK index. If a profile load exceeds ~50ms in production, add
  `users.follower_count` and update via triggers.
- `comments` is flat — no `parent_comment_id`. The 1-level threading
  invariant is enforced by UI, not schema.
- `appreciations` and `follows` share the toggle-via-PK-conflict pattern:
  `INSERT ... ON CONFLICT DO NOTHING` (turn ON), `DELETE` (turn OFF).

## Endpoints

```
POST   /api/photos/:id/appreciate                       (auth, idempotent)  204
DELETE /api/photos/:id/appreciate                       (auth, idempotent)  204
GET    /api/photos/:id/appreciations/count              (public)            200 { count: i64 }
GET    /api/photos/:id/appreciation-state               (auth required)     200 { appreciated: bool } / 401

GET    /api/photos/:id/comments                         (public)            200 { comments: [...] }
POST   /api/photos/:id/comments                         (auth)              201 + Comment
DELETE /api/comments/:id                                (auth, see below)   204 / 403

POST   /api/users/:id/follow                            (auth, idempotent)  204
DELETE /api/users/:id/follow                            (auth, idempotent)  204
GET    /api/users/:id/followers/count                   (public)            200 { count: i64 }
GET    /api/users/:id/following/count                   (public)            200 { count: i64 }

# Existing endpoints extended:
GET    /api/auth/me                                     (auth)              200 + following_ids: [uuid]
GET    /api/photos/:id                                  (public)            200 + appreciation_count, comment_count
```

`DELETE /api/comments/:id` rules:
- 204 if requester is `comment.author_id` OR is `photos.owner_id` for
  the photo this comment belongs to.
- 403 otherwise.
- 401 if no session.

`/api/photos?following_for=<user_id>` — implemented as a new query
branch in the existing list handler, using the session's user id (no
`?following_for` query param needed; the handler checks auth + `following`
flag instead). Final shape:

```
GET /api/photos?following=true                          (auth required)   200 { photos: [...] }
```

When `following=true` and the user follows nobody → backend returns an
empty list. The frontend then falls back to a second call without the
flag. (See "Following feed" below.)

## Following feed

Logic in `frontend/src/routes/+page.server.ts`:

```ts
export const load: PageServerLoad = async ({ fetch, locals }) => {
  const isAuth = !!locals.user;
  let realPhotos: PhotoSummary[] = [];

  if (isAuth) {
    // Try the personalised feed first.
    const r1 = await api.photos.list({ following: true });
    realPhotos = r1.photos;
  }

  if (realPhotos.length === 0) {
    // Either anonymous, or auth user follows nobody, or follows
    // photographers who haven't posted: show the public feed.
    const r2 = await api.photos.list({ limit: 24 });
    realPhotos = r2.photos;
  }

  // ... rest unchanged
};
```

Eyebrow on the gallery section reflects which feed is shown. For now we
keep the single eyebrow "FRAME OF THE WEEK"; enhancing this to "FROM
THE PHOTOGRAPHERS YOU FOLLOW · 12" can come in Phase 8.

## UI changes

### Photo detail (`/photo/[slug]`)

**AppreciateButton component** replaces the existing inline `♡ N
appreciations` button. Props: `photoId`, `appreciated`, `count`. State
derived from auth (clicking when not auth → redirect to /signin?return=...).
Optimistic updates with rollback on backend error.

**CommentsSection component** (new) below the action row in the aside.
Renders a list of comments and, when authenticated, a textarea + Post
button. Comment body is plain text, max 2000 chars. Each comment:
24px avatar circle + display name + relative timestamp ("2h ago",
"3d ago", "Mar 17") + body text. Delete affordance visible only when
the request user is the comment author OR the photo owner.

The `+page.server.ts` for the photo detail route gains form actions:
- `appreciate` / `unappreciate` (for non-JS clients)
- `comment` (post a new comment)
- `deleteComment` (delete by id)

These are server-side actions that POST to the backend with the user's
cookie forwarded.

### User profile (`/u/[uuid]`)

**FollowButton component** replaces the placeholder `Follow` button on
the hero. Props: `userId`, `isFollowing`. Hidden entirely when viewing
your own profile. Click → POST/DELETE the follow endpoint, optimistic
update.

**Stats row** updates: `followers` count is fetched via the new endpoint
on load. `frames` already wired (Phase 6). `integration` stays "—" for
now (calculation deferred). `collections` stays "0" hardcoded
(collections feature is Phase 8+).

### Authenticated home (`/`)

Falls into the following-feed branch when `locals.user` is present.
Hero + grid layout unchanged; only the data source switches. Empty
result → fallback to public gallery (item 2 in Decisions).

## Backend module layout

```
backend/src/engagement/
├── mod.rs              ← pub mod appreciations; pub mod comments; pub mod follows;
├── appreciations.rs    ← handlers + inline queries (toggle, count, state)
├── comments.rs         ← handlers + inline queries (list, create, delete)
└── follows.rs          ← handlers + inline queries (toggle, counts)
```

Each file aimed at <200 lines. Patterns shared by `appreciations` and
`follows` (idempotent toggle via PK conflict) repeated rather than
abstracted — two callers don't justify a helper trait.

`/api/auth/me` extension lives in `auth/me.rs` (existing); the SQL to
fetch `following_ids` is added there directly.

`/api/photos/:id` extension lives in `photos/get.rs`; `appreciation_count`
and `comment_count` added to `PhotoDetail` DTO and 2 extra queries.

## Testing

3 integration tests in `backend/tests/engagement.rs`:

```rust
// 1. appreciation_toggle
//    signup → POST appreciate (204) → POST again (204) → count=1 → DELETE (204)
//    → count=0 → DELETE again (204).

// 2. comment_create_list_delete_authorization
//    userA signup → upload photo
//    userB signup → POST comment (201)
//    GET list (1 comment, contains userB's text)
//    userC signup
//    userC DELETE userB's comment (403)
//    userA (photo owner) DELETE userB's comment (204)
//    GET list (0 comments)

// 3. follow_toggle_and_counts
//    userA signup, userB signup
//    userA POST follow userB (204)
//    GET userB followers/count (1)
//    GET userA /me — following_ids contains userB's id
//    userA DELETE follow userB (204)
//    counts back to 0
//    userA POST follow userA (422 — self-follow forbidden)
```

No new unit tests — handlers are thin SQL wrappers, exercised by the
integration tests.

No new frontend tests. Final smoke test in the merge phase exercises
the full flow via Chrome DevTools.

## Implementation order

5 batches, ~6h total:

| # | Contenu | Effort |
|---|---------|--------|
| 1 | Migration `0002_engagement.sql` + `appreciations` (handlers + integration test) | 1h |
| 2 | `comments` (handlers + integration test) | 1.5h |
| 3 | `follows` (handlers + integration test) + extend `/me` + extend `/api/photos/:id` | 1.5h |
| 4 | Frontend: `AppreciateButton`, `FollowButton` + form actions | 1h |
| 5 | Frontend: `CommentsSection` + following feed branch on `/` + browser smoke test | 1h |

Final task: merge to main with `--no-ff`, tag `v0.6.0-engagement`.

## Out of scope (deferred)

- **Notifications** (Phase 8): in-app badge + dropdown + email digest.
- **Comment edit** (Phase 8): requires `edited_at` column.
- **Comment likes** — never. Single appreciation surface per photo.
- **Block / mute** (Phase 9+).
- **Rate limiting on spam** (Phase 9): for now Argon2 signup cost
  naturally limits abuse rate.
- **Direct messages** (Vision per design brief).
- **Following feed eyebrow personalisation**: currently keeps "FRAME OF
  THE WEEK"; switching label to "FROM THE PHOTOGRAPHERS YOU FOLLOW · N"
  is a Phase 8 polish.
- **Integration time calculation** on profile: needs `sum(exposure_s *
  count)` per photo, deferred.
- **Collections**: Phase 8+.

## References

- Phase 6 plan (just merged): `docs/superpowers/plans/2026-05-02-phase-6-polish.md`
- Original brief: `docs/design/2026-05-01-design-brief.md` § Phase 2 features
- Schema source: `backend/migrations/0001_init.sql`
- Existing patterns to extend: `backend/src/auth/middleware.rs::CurrentUser`,
  `backend/src/photos/queries.rs::find_by_id`, `backend/src/users/queries.rs`.
