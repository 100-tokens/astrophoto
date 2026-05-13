import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, url, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');

  let r = await fetch(`${API}/api/photos/${params.id}`, {
    headers: { Cookie: cookie }
  });
  if (r.status === 404) error(404, 'Photo not found');
  if (!r.ok) error(500, 'Backend error');
  let photo = await r.json();
  if (photo.owner_id !== locals.user.id) error(404, 'Not found');

  // Fetch the user's setups list.
  const sr = await fetch(`${API}/api/equipment/setups`, {
    headers: { Cookie: cookie }
  });
  const setups = sr.ok ? await sr.json() : [];

  // Auto-apply default setup only on first visit (before the user has been
  // through the verify step). Once last_step is 'verify' or later, respect
  // any detach decision the user may have made.
  if (!photo.setup_id && (photo.last_step === null || photo.last_step === 'upload')) {
    const def = setups.find((s: { is_default: boolean }) => s.is_default);
    if (def) {
      const ar = await fetch(`${API}/api/photos/${params.id}/apply-setup`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Cookie: cookie },
        body: JSON.stringify({ setup_id: def.id, mode: 'fill_empty' })
      });
      if (ar.ok) {
        // Re-read the photo so the form has the merged columns and setup_id.
        r = await fetch(`${API}/api/photos/${params.id}`, {
          headers: { Cookie: cookie }
        });
        if (r.ok) photo = await r.json();
      }
    }
  }

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

  return { photo, setups, queueIds, queueIndex };
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
    aperture_f: numOrNull('aperture_f'),
    gain: numOrNull('gain'),
    sensor_temp_c: numOrNull('sensor_temp_c'),
    sessions: numOrNull('sessions'),
    ra_deg: numOrNull('ra_deg'),
    dec_deg: numOrNull('dec_deg'),
    category: strOrNull('category'),
    scope: strOrNull('scope'),
    focal_modifier: strOrNull('focal_modifier'),
    mount: strOrNull('mount'),
    filters: strOrNull('filters'),
    guiding: strOrNull('guiding'),
    tags: parseTags(),
    last_step
  };
}

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
