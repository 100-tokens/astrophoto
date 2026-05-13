import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { getPhoto, batchPublish, ApiError } from '$lib/api/client';

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

  // Backend returns 404 for photos not visible to the caller (owner-gated).
  // Re-emit as 404 so the user sees "Not found" rather than "Unexpected error".
  // Any non-ApiError or unexpected status falls through to a 500.
  let photos;
  try {
    photos = await Promise.all(ids.map((id) => getPhoto(id, { fetch, cookie })));
  } catch (e) {
    if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
      error(404, 'Not found');
    }
    throw e;
  }

  for (const p of photos) {
    if (p.owner_id !== locals.user.id) error(404, 'Not found');
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

    let result;
    try {
      result = await batchPublish({ fetch, cookie, ids });
    } catch (e: unknown) {
      const status = (e as { status?: number }).status ?? 500;
      const body = (e as { body?: unknown }).body;
      return fail(status, {
        error: typeof body === 'string' ? body : JSON.stringify(body)
      });
    }
    redirect(
      303,
      `/account/frames?published=${result.published.length}&skipped=${result.skipped.length}`
    );
  }
};
