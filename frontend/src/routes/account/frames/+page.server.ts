import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals, url, fetch, cookies }) => {
  if (!locals.user) redirect(303, `/signin?next=${encodeURIComponent(url.pathname + url.search)}`);
  const filter = (url.searchParams.get('filter') ?? 'all') as 'all' | 'published' | 'drafts';
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'oldest';
  const view = (url.searchParams.get('view') ?? 'list') as 'list' | 'grid';
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');

  const [stats, published, drafts] = await Promise.all([
    fetch(`${API}/api/me/stats`, { headers: { Cookie: cookie } }).then((r) => r.json()),
    fetch(`${API}/api/photos?owner_id=${locals.user.id}`, { headers: { Cookie: cookie } }).then((r) => r.json()),
    fetch(`${API}/api/photos?drafts=true`, { headers: { Cookie: cookie } }).then((r) => r.json())
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
    id: string; target?: string | null; original_name: string;
    taken_at?: string | null; exposure_s?: number | null;
    is_draft: boolean; status: string; appreciation_count: number;
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function normalisePhoto(p: any): PhotoRow {
    return { ...p, appreciation_count: Number(p.appreciation_count ?? 0) } as PhotoRow;
  }

  const publishedPhotos: PhotoRow[] = (published.photos ?? []).map(normalisePhoto);
  const draftsPhotos: PhotoRow[] = (drafts.photos ?? []).map(normalisePhoto);

  let rows: PhotoRow[] =
    filter === 'drafts' ? draftsPhotos :
    filter === 'published' ? publishedPhotos :
    [...draftsPhotos, ...publishedPhotos];
  if (sort === 'oldest') rows = [...rows].reverse();

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
