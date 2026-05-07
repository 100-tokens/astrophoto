import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { api } from '$lib/api/client';
import type { DraftListItem } from '$lib/api/DraftListItem';

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

  return { tier: locals.user.tier, recentDrafts };
};
