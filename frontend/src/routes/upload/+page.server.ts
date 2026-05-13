import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { api } from '$lib/api/client';
import type { DraftListItem } from '$lib/api/DraftListItem';
import type { StorageSummary } from '$lib/api/StorageSummary';

export const load: PageServerLoad = async ({ locals, fetch, request }) => {
  if (!locals.user) redirect(303, '/signin');

  const cookie = request.headers.get('cookie') ?? '';

  let recentDrafts: DraftListItem[] = [];
  try {
    const draftsResp = await api.drafts({ fetch, cookie, limit: 24 });
    const since = Date.now() - 24 * 60 * 60 * 1000;
    recentDrafts = draftsResp.items.filter((d) => Date.parse(d.created_at) >= since);
  } catch {
    // Non-critical: dropzone still works without the resume banner.
  }

  // Fetched server-side so the footer storage line renders on first paint.
  // Failure is non-fatal — the line just hides.
  let storage: StorageSummary | null = null;
  try {
    const r = await fetch('/api/me/storage', { headers: { cookie } });
    if (r.ok) storage = (await r.json()) as StorageSummary;
  } catch {
    /* ignore */
  }

  return { tier: locals.user.tier, recentDrafts, storage };
};
