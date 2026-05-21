import type { PageServerLoad } from './$types';
import type { EquipmentCatalogResponse } from '$lib/api/EquipmentCatalogResponse';
import { error } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

/** Valid kind values — mirrors `equipment::VALID_KINDS` on the backend. */
const VALID_KINDS = new Set([
  'telescope',
  'camera',
  'mount',
  'filter',
  'focal_modifier',
  'guiding'
]);

const ALLOWED_SORTS = new Set(['most_used', 'brand_asc', 'aperture_desc', 'recent']);

/**
 * Browse page server-load. Reads the URL query, validates a small
 * subset of params client-side (kind + sort), and proxies the rest to
 * the new `/api/equipment/catalog` endpoint via fetch — same posture
 * as the existing `/equip/[kind]/[slug]` route. We do basic validation
 * here because the SvelteKit error page renders nicer than a backend
 * 422.
 */
export const load: PageServerLoad = async ({ fetch, url, params }) => {
  if (!VALID_KINDS.has(params.kind)) {
    throw error(404, 'Unknown equipment kind');
  }

  const q = url.searchParams.get('q')?.trim() ?? '';
  const brandRaw = url.searchParams.get('brand') ?? '';
  const minApRaw = url.searchParams.get('min_aperture');
  const maxApRaw = url.searchParams.get('max_aperture');
  const sortRaw = url.searchParams.get('sort') ?? 'most_used';
  const sort = ALLOWED_SORTS.has(sortRaw) ? sortRaw : 'most_used';
  const page = Number(url.searchParams.get('page') ?? '0');
  const safePage = Number.isFinite(page) && page >= 0 ? Math.floor(page) : 0;

  const params2 = new URLSearchParams();
  params2.set('kind', params.kind);
  params2.set('limit', '24');
  params2.set('page', String(safePage));
  if (q) params2.set('q', q);
  if (brandRaw) params2.set('brand', brandRaw);
  if (minApRaw) params2.set('min_aperture', minApRaw);
  if (maxApRaw) params2.set('max_aperture', maxApRaw);
  params2.set('sort', sort);

  try {
    const r = await fetch(`${API}/api/equipment/catalog?${params2.toString()}`);
    if (!r.ok) throw error(r.status, `catalog fetch failed: ${r.status}`);
    const data = (await r.json()) as EquipmentCatalogResponse;
    return {
      kind: params.kind,
      q,
      brand: brandRaw,
      minAperture: minApRaw,
      maxAperture: maxApRaw,
      sort,
      page: safePage,
      response: data
    };
  } catch (e) {
    if (e && typeof e === 'object' && 'status' in e) throw e;
    throw error(500, 'Failed to load catalog');
  }
};
