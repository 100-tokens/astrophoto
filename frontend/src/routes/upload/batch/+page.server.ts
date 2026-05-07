import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import { getPhoto, batchApply } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals, url, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');

  const idsParam = url.searchParams.get('ids');
  if (!idsParam) error(400, 'missing ids');
  const ids = idsParam.split(',').filter(Boolean);
  if (ids.length === 0) error(400, 'no ids');
  if (ids.length === 1) redirect(303, `/upload/${ids[0]}/verify`);

  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');

  const photos = await Promise.all(
    ids.map(async (id) => {
      const r = await getPhoto(id, { fetch, cookie });
      return r;
    })
  );

  for (const p of photos) {
    if (!p.is_draft) error(400, `photo ${p.id} is already published`);
    if (p.owner_id !== locals.user.id) error(403, 'not owner');
  }

  return { ids, photos };
};

export const actions: Actions = {
  default: async ({ request, fetch, locals, cookies }) => {
    if (!locals.user) return fail(401);
    const data = await request.formData();
    const ids = (data.get('ids') as string).split(',').filter(Boolean);
    const targetRaw = data.get('target') as string | null;
    const target = targetRaw?.trim() || undefined;

    const parseTags = (): string[] | undefined => {
      try {
        const raw = data.get('tags');
        const parsed: unknown = JSON.parse(typeof raw === 'string' ? raw : '[]');
        if (!Array.isArray(parsed)) return undefined;
        const tags = parsed.filter((t): t is string => typeof t === 'string');
        return tags.length > 0 ? tags : undefined;
      } catch {
        return undefined;
      }
    };
    const tags = parseTags();

    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');

    try {
      await batchApply({
        fetch,
        cookie,
        ids,
        ...(target !== undefined ? { target } : {}),
        ...(tags !== undefined ? { tags } : {})
      });
    } catch (e: unknown) {
      const status = (e as { status?: number }).status ?? 500;
      const body = (e as { body?: unknown }).body;
      return fail(status, {
        error: typeof body === 'string' ? body : JSON.stringify(body)
      });
    }

    redirect(303, `/upload/batch/edit?ids=${ids.join(',')}`);
  }
};
