import type { PageServerLoad } from './$types';
import type { PhotographerIndexPage } from '$lib/api/PhotographerIndexPage';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const VALID_SORTS = new Set(['active', 'followers', 'recent']);

export const load: PageServerLoad = async ({ url, fetch }) => {
  const sort = VALID_SORTS.has(url.searchParams.get('sort') ?? '')
    ? (url.searchParams.get('sort') as 'active' | 'followers' | 'recent')
    : 'active';

  const r = await fetch(`${API}/api/photographers?sort=${sort}&limit=24`);
  let initial: PhotographerIndexPage = { items: [], next_cursor: null };
  if (r.ok) initial = (await r.json()) as PhotographerIndexPage;

  return { sort, initial };
};
