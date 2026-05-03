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

async function callPut(
  fetchFn: typeof globalThis.fetch,
  cookie: string,
  id: string,
  patch: Record<string, unknown>
) {
  return fetchFn(`${API}/api/photos/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json', Cookie: cookie },
    body: JSON.stringify(patch)
  });
}

function collectPatch(fd: FormData, last_step: 'verify' | 'caption') {
  const numOrNull = (k: string): number | null => {
    const v = fd.get(k);
    if (typeof v !== 'string' || v.trim() === '') return null;
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  };
  const strOrNull = (k: string): string | null => {
    const v = fd.get(k);
    return typeof v === 'string' && v.trim() !== '' ? v.trim() : null;
  };
  const parseTags = (): string[] => {
    try {
      const raw = fd.get('tags');
      const parsed: unknown = JSON.parse(typeof raw === 'string' ? raw : '[]');
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((t): t is string => typeof t === 'string');
    } catch {
      return [];
    }
  };
  return {
    target: strOrNull('target'),
    camera: strOrNull('camera'),
    lens: strOrNull('lens'),
    iso: numOrNull('iso'),
    exposure_s: numOrNull('exposure_s'),
    focal_mm: numOrNull('focal_mm'),
    category: strOrNull('category'),
    scope: strOrNull('scope'),
    mount: strOrNull('mount'),
    filters: strOrNull('filters'),
    guiding: strOrNull('guiding'),
    tags: parseTags(),
    last_step
  };
}

export const actions: Actions = {
  save_continue: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const r = await callPut(fetch, cookie, params.id!, patch);
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/upload/${params.id}/caption`);
  },

  save_draft: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    await callPut(fetch, cookie, params.id!, patch);
    redirect(303, '/account/frames');
  },

  save_changes_published: async ({ request, params, fetch, cookies }) => {
    // Edit-metadata terminus: published photo, save and go to /photo/[slug].
    const fd = await request.formData();
    const patch = collectPatch(fd, 'caption');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    await callPut(fetch, cookie, params.id!, patch);
    redirect(303, `/photo/${params.id}`);
  }
};
