import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals, url, fetch, cookies }) => {
  if (!locals.user) redirect(303, `/signin?next=${encodeURIComponent(url.pathname + url.search)}`);
  const filter = (url.searchParams.get('filter') ?? 'all') as 'all' | 'published' | 'drafts';
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'oldest';
  const view = (url.searchParams.get('view') ?? 'list') as 'list' | 'grid';
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');

  // Surface backend failures as a real error page instead of letting an
  // AppError JSON flow into Number(...) (NaN stats) or a non-JSON gateway
  // body reject r.json() into an opaque 500.
  const fetchJson = async (path: string): Promise<Record<string, unknown>> => {
    const r = await fetch(`${API}${path}`, { headers: { Cookie: cookie } });
    if (!r.ok) error(502, 'Backend error');
    return (await r.json()) as Record<string, unknown>;
  };

  const [stats, published, drafts] = await Promise.all([
    fetchJson('/api/me/stats'),
    fetchJson(`/api/photos?owner_id=${locals.user.id}`),
    fetchJson('/api/photos?drafts=true')
  ]);

  // Cast bigint-typed fields to number before serializing to the page.
  // JSON.parse produces plain numbers anyway; this is defensive typing.
  const statsTyped = {
    published_count: Number(stats.published_count),
    draft_count: Number(stats.draft_count),
    integration_secs: Number(stats.integration_secs),
    appreciations_received: Number(stats.appreciations_received)
  };

  // Cast appreciation_count on each photo row (PhotoDetail has bigint appreciation_count).
  type PhotoRow = {
    id: string;
    target?: string | null;
    original_name: string;
    taken_at?: string | null;
    exposure_s?: number | null;
    is_draft: boolean;
    status: string;
    appreciation_count: number;
    created_at: string;
  };

  function normalisePhoto(p: unknown): PhotoRow {
    // Keep the spread: rows carry fields beyond PhotoRow at runtime that
    // downstream components rely on (e.g. last_step for DraftCard).
    const rec = p as Record<string, unknown>;
    return { ...rec, appreciation_count: Number(rec.appreciation_count ?? 0) } as PhotoRow;
  }

  const photosOf = (resp: Record<string, unknown>): unknown[] =>
    Array.isArray(resp.photos) ? resp.photos : [];

  const publishedPhotos: PhotoRow[] = photosOf(published).map(normalisePhoto);
  const draftsPhotos: PhotoRow[] = photosOf(drafts).map(normalisePhoto);

  let rows: PhotoRow[] =
    filter === 'drafts'
      ? draftsPhotos
      : filter === 'published'
        ? publishedPhotos
        : [...draftsPhotos, ...publishedPhotos];

  // Sort by created_at so oldest/newest is coherent across the merged list.
  // (A plain reverse() only flips the concatenation order, not the timestamp order.)
  const dir = sort === 'newest' ? -1 : 1;
  rows = [...rows].sort((a, b) => dir * a.created_at.localeCompare(b.created_at));

  return {
    stats: statsTyped,
    filter,
    sort,
    view,
    rows,
    drafts: draftsPhotos,
    counts: {
      all: publishedPhotos.length + draftsPhotos.length,
      published: publishedPhotos.length,
      drafts: draftsPhotos.length
    }
  };
};
