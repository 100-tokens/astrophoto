// API client wrappers for the P3 discovery endpoints.
// Pages: /explore, /t/<slug>, /tag/<slug>, /equip/<kind>/<slug>, /c/<cat>, /search.
// Plus pass-throughs to the existing P1 autocomplete endpoints.

import type { DiscoveryPage } from './DiscoveryPage';
import type { TargetPage } from './TargetPage';
import type { TagPage } from './TagPage';
import type { EquipmentPage } from './EquipmentPage';
import type { CategoryPage } from './CategoryPage';
import type { SearchResults } from './SearchResults';

const API_BASE: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';
type FetchFn = typeof fetch;

export interface FeedOpts {
  cursor?: string;
  sort?: 'newest' | 'most-appreciated';
  since?: '24h' | '7d' | '30d' | 'all';
  category?: string;
  following?: boolean;
  limit?: number;
}

function qs(opts: Record<string, string | number | boolean | undefined>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(opts)) {
    if (v === undefined || v === '' || v === false) continue;
    p.set(k, String(v));
  }
  const s = p.toString();
  return s ? `?${s}` : '';
}

export async function fetchExplore(f: FetchFn, opts: FeedOpts = {}): Promise<DiscoveryPage> {
  const r = await f(
    `${API_BASE}/api/explore${qs(opts as Record<string, string | number | boolean | undefined>)}`
  );
  if (!r.ok) throw new Error(`fetchExplore ${r.status}`);
  return (await r.json()) as DiscoveryPage;
}

export async function fetchTargetPage(
  f: FetchFn,
  slug: string,
  opts: FeedOpts = {}
): Promise<TargetPage> {
  const r = await f(
    `${API_BASE}/api/targets/${slug}${qs(opts as Record<string, string | number | boolean | undefined>)}`
  );
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchTargetPage ${r.status}`);
  return (await r.json()) as TargetPage;
}

export async function fetchTagPage(
  f: FetchFn,
  slug: string,
  opts: FeedOpts = {}
): Promise<TagPage> {
  const r = await f(
    `${API_BASE}/api/tags/${slug}${qs(opts as Record<string, string | number | boolean | undefined>)}`
  );
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchTagPage ${r.status}`);
  return (await r.json()) as TagPage;
}

export async function fetchEquipmentPage(
  f: FetchFn,
  kind: string,
  slug: string,
  opts: FeedOpts = {}
): Promise<EquipmentPage> {
  const r = await f(
    `${API_BASE}/api/equipment/${kind}/${slug}${qs(opts as Record<string, string | number | boolean | undefined>)}`
  );
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchEquipmentPage ${r.status}`);
  return (await r.json()) as EquipmentPage;
}

export async function fetchCategoryPage(
  f: FetchFn,
  cat: string,
  opts: FeedOpts = {}
): Promise<CategoryPage> {
  const r = await f(
    `${API_BASE}/api/categories/${cat}${qs(opts as Record<string, string | number | boolean | undefined>)}`
  );
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchCategoryPage ${r.status}`);
  return (await r.json()) as CategoryPage;
}

export async function fetchSearch(f: FetchFn, q: string): Promise<SearchResults> {
  const r = await f(`${API_BASE}/api/search?q=${encodeURIComponent(q)}`);
  if (!r.ok) throw new Error(`fetchSearch ${r.status}`);
  return (await r.json()) as SearchResults;
}

// ── Autocomplete pass-throughs (existing endpoints from P1) ─────────────

export async function autocompleteTargets(
  f: FetchFn,
  q: string
): Promise<Array<{ slug: string; canonical_name: string }>> {
  const r = await f(`${API_BASE}/api/targets/autocomplete?q=${encodeURIComponent(q)}`);
  if (!r.ok) throw new Error(`autocompleteTargets ${r.status}`);
  return (await r.json()) as Array<{ slug: string; canonical_name: string }>;
}

export async function autocompleteTags(
  f: FetchFn,
  q: string
): Promise<Array<{ slug: string; name: string }>> {
  const r = await f(`${API_BASE}/api/tags/autocomplete?q=${encodeURIComponent(q)}`);
  if (!r.ok) throw new Error(`autocompleteTags ${r.status}`);
  return (await r.json()) as Array<{ slug: string; name: string }>;
}

export async function autocompleteEquipment(
  f: FetchFn,
  kind: string,
  q: string
): Promise<Array<{ kind: string; canonical_name: string; display_name: string }>> {
  const params = new URLSearchParams({ kind, q });
  const r = await f(`${API_BASE}/api/equipment/autocomplete?${params.toString()}`);
  if (!r.ok) throw new Error(`autocompleteEquipment ${r.status}`);
  return (await r.json()) as Array<{ kind: string; canonical_name: string; display_name: string }>;
}
