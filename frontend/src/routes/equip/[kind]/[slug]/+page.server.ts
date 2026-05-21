import type { PageServerLoad } from './$types';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import { error } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

/**
 * Photo tile we render in the detail page. Sourced primarily from the
 * discovery handler (which already returns `author_handle` so we can
 * link to `/u/<handle>/p/<short_id>` without an extra round-trip).
 */
interface PhotoTile {
  id: string;
  short_id: string;
  target: string | null;
  author_handle: string;
}

/**
 * Detail-page server load. Strategy:
 *   1. Resolve `kind + slug` → meta + photos via the existing
 *      `discovery::equipment::get` handler. It already does the
 *      `lower(scope|camera|mount|filters|guiding) = slug` photo
 *      lookup AND returns each photo's `author_handle`, which is the
 *      only thing we need beyond the typed catalog item.
 *   2. Hydrate the full `EquipmentItemDetail` (specs, brand/model,
 *      submitter handle, setup_count) via `/api/equipment/items/:id`,
 *      using the id returned by step 1.
 *
 * Filter kinds: the discovery handler matches against the legacy
 * `photos.filters` text cache (exact-string), so a multi-filter
 * photo with `filters="L,R,G,B"` won't show up under the
 * single-filter slug `"L"`. The backend now also exposes the
 * junction-backed `?filter_item_id=` query on `/api/photos`, but
 * that handler doesn't return author handles — leveraging it
 * requires a small follow-up to either extend `/api/photos` with
 * handles or add a dedicated discovery flavor. Captured in the
 * implementation report as a known limitation.
 */
export const load: PageServerLoad = async ({ fetch, params, parent }) => {
  // The "Edit specs" affordance is shown to any signed-in user
  // (Phase 1 — no moderation queue per spec). Delete is gated
  // separately on backend side and intentionally not surfaced yet
  // (no DELETE endpoint exists).
  const layout = await parent();
  const user = layout.user as { id: string } | null | undefined;

  const discoveryR = await fetch(
    `${API}/api/equipment/${encodeURIComponent(params.kind)}/${encodeURIComponent(params.slug)}?limit=24`
  );
  if (discoveryR.status === 404) throw error(404, 'Equipment item not found');
  if (!discoveryR.ok) throw error(500, `Failed to load catalog item`);
  const discovery = (await discoveryR.json()) as {
    equipment: { id: string };
    page: {
      photos: Array<{
        id: string;
        short_id: string;
        target: string | null;
        author_handle: string;
      }>;
    };
  };

  const itemR = await fetch(`${API}/api/equipment/items/${discovery.equipment.id}`);
  if (!itemR.ok) throw error(500, 'Failed to load catalog item detail');
  const item = (await itemR.json()) as EquipmentItemDetail;

  const photos: PhotoTile[] = discovery.page.photos.map((p) => ({
    id: p.id,
    short_id: p.short_id,
    target: p.target,
    author_handle: p.author_handle
  }));

  return {
    item,
    photos,
    canSeeEditAffordance: !!user
  };
};
