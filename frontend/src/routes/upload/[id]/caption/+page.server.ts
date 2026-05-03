import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
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
  return { photo };
};

export const actions: Actions = {
  publish: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    let r = await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null, last_step: 'caption' })
    });
    if (!r.ok) return fail(r.status, { error: await r.text() });
    r = await fetch(`${API}/api/photos/${params.id}/publish`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/photo/${params.id}`);
  },

  save_draft: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null, last_step: 'caption' })
    });
    redirect(303, '/account/frames');
  },

  save_changes: async ({ request, params, fetch, cookies }) => {
    // Already-published variant: caption-only PUT, no publish call.
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null })
    });
    redirect(303, `/photo/${params.id}`);
  }
};
