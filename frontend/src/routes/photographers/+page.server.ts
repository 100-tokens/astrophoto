import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { PhotographerIndexPage } from '$lib/api/PhotographerIndexPage';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const VALID_SORTS = new Set(['active', 'followers', 'recent']);

export const load: PageServerLoad = async ({ url, fetch }) => {
  const sort = VALID_SORTS.has(url.searchParams.get('sort') ?? '')
    ? (url.searchParams.get('sort') as 'active' | 'followers' | 'recent')
    : 'active';

  // Fail loud, not fake-empty: a backend outage used to render the
  // "No photographers yet" empty state with HTTP 200 (plus JSON-LD
  // numberOfItems:0 for crawlers), and a network-level failure escaped
  // as an unhandled rejection. Same convention as the explore loader.
  let initial: PhotographerIndexPage;
  try {
    const r = await fetch(`${API}/api/photographers?sort=${sort}&limit=24`);
    if (!r.ok) throw new Error(`photographers ${r.status}`);
    initial = (await r.json()) as PhotographerIndexPage;
  } catch (_e) {
    throw error(500, 'Failed to load photographers');
  }

  return { sort, initial };
};
