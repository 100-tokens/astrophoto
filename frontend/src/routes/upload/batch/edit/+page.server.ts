import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { getPhoto, batchPublish } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals, url, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');

  const idsParam = url.searchParams.get('ids');
  if (!idsParam) error(400, 'missing ids');
  const ids = idsParam.split(',').filter(Boolean);
  if (ids.length === 0) error(400, 'no ids');

  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');

  const photos = await Promise.all(ids.map((id) => getPhoto(id, { fetch, cookie })));

  for (const p of photos) {
    if (p.owner_id !== locals.user.id) error(403, 'not owner');
  }

  const selectedParam = url.searchParams.get('selected') ?? ids[0]!;
  const selected = ids.includes(selectedParam) ? selectedParam : ids[0]!;

  return { ids, photos, selected };
};

export const actions: Actions = {
  publish_all: async ({ fetch, locals, url, cookies }) => {
    if (!locals.user) return fail(401);

    const ids = (url.searchParams.get('ids') ?? '').split(',').filter(Boolean);
    if (ids.length === 0) return fail(400, { error: 'no ids' });

    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');

    try {
      const result = await batchPublish({ fetch, cookie, ids });
      redirect(
        303,
        `/account/frames?published=${result.published.length}&skipped=${result.skipped.length}`
      );
    } catch (e: unknown) {
      const status = (e as { status?: number }).status ?? 500;
      const body = (e as { body?: unknown }).body;
      return fail(status, {
        error: typeof body === 'string' ? body : JSON.stringify(body)
      });
    }
  }
};
