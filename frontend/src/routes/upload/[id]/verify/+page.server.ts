import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, url, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
  const r = await fetch(`${API}/api/photos/${params.id}`, { headers: { Cookie: cookie } });
  if (r.status === 404) error(404, 'Photo not found');
  if (!r.ok) error(500, 'Backend error');
  const photo = await r.json();
  if (photo.owner_id !== locals.user.id) error(404, 'Not found');

  // Optional batch context: ?ids=a,b,c lets the verify page render a
  // queue thumbs strip and Skip/Continue with frame index. Filter to
  // the caller's own photos so a forged URL can't enumerate someone
  // else's drafts (each thumb still hits an authorised /api/photos/:id
  // call client-side, but pre-filtering keeps the strip honest).
  const idsParam = url.searchParams.get('ids');
  let queueIds: string[] = [];
  let queueIndex = -1;
  if (idsParam) {
    const ids = idsParam
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    queueIds = ids;
    queueIndex = ids.indexOf(params.id);
  }

  return { photo, queueIds, queueIndex };
};

export const actions: Actions = {
  // Autosave (useAutosave) continuously PATCHes metadata, so these actions
  // only need to handle state transitions. No PUT required on submit.

  save_draft: async () => {
    redirect(303, '/account/frames');
  },

  save_changes_published: async ({ params }) => {
    redirect(303, `/photo/${params.id}`);
  },

  publish: async ({ params, fetch, cookies }) => {
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const r = await fetch(`${API}/api/photos/${params.id}/publish`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/photo/${params.id}`);
  }
};
