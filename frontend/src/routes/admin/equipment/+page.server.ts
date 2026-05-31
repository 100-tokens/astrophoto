import type { PageServerLoad } from './$types';
import { fetchEquipment } from '$lib/api/adminClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const kind = url.searchParams.get('kind') ?? '';
  const q = url.searchParams.get('q') ?? '';
  const page = Number(url.searchParams.get('page') ?? '0') || 0;
  try {
    const opts: { kind?: string; q?: string; page?: number } = { page };
    if (kind) opts.kind = kind;
    if (q) opts.q = q;
    const data = await fetchEquipment(fetch, opts);
    return { ...data, kind, q };
  } catch (_e) {
    throw error(500, 'Failed to load equipment catalog');
  }
};
