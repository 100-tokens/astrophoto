import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { SetupDetail } from '$lib/api/SetupDetail';

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

  // Compute orphan tokens: legacy cache-string tokens that don't match any
  // structured filter_item. Shown as read-only "legacy" chips in the UI.
  const cacheTokens = ((photo.filters as string | null | undefined) ?? '')
    .split(',')
    .map((t: string) => t.trim())
    .filter(Boolean);
  const known = new Set<string>(
    ((photo.filter_items as Array<{ display_name: string }>) ?? []).map(
      (c: { display_name: string }) => c.display_name
    )
  );
  const orphans = cacheTokens.filter((t: string) => !known.has(t));

  // Seed the plate-solve panel so the first paint reflects whether a
  // solve is already in flight (e.g. user opened a second tab) or
  // previously completed. Treated as best-effort: failures land in
  // the UI as `null`, which the panel renders as the idle state.
  let platesolveStatus = null;
  try {
    const psr = await fetch(`${API}/api/photos/${params.id}/platesolve-status`, {
      headers: { Cookie: cookie }
    });
    if (psr.ok) platesolveStatus = await psr.json();
  } catch {
    // best-effort
  }

  // When a setup is applied, fetch its detail so the page can label each
  // equipment field's provenance (FROM SETUP vs FROM EXIF). Best-effort:
  // on failure `setupValues` stays null and the page omits FROM SETUP.
  let setupValues: {
    camera: string | null;
    scope: string | null;
    mount: string | null;
    focal_modifier: string | null;
    guiding: string | null;
  } | null = null;
  if (photo.setup_id) {
    try {
      const dr = await fetch(`${API}/api/equipment/setups/${photo.setup_id}`, {
        headers: { Cookie: cookie }
      });
      if (dr.ok) {
        const detail: SetupDetail = await dr.json();
        const items = detail.items ?? [];
        const byRole = (role: string): string | null =>
          items.find((i) => i.role === role)?.item.display_name ?? null;
        setupValues = {
          camera: byRole('main_camera'),
          scope: byRole('optical_tube'),
          mount: byRole('mount'),
          focal_modifier: byRole('focal_modifier'),
          guiding: detail.guiding ?? null
        };
      }
    } catch {
      // best-effort — provenance just falls back to FROM EXIF / none
    }
  }

  return { photo, setups, queueIds, queueIndex, orphans, platesolveStatus, setupValues };
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

// `last_step` only ever 'verify' now — the 'caption' step was removed
// (56acf4e) and verify is the single publish step.
function collectPatch(fd: FormData, last_step: 'verify') {
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
  const parseFilterItemIds = (): string[] | undefined => {
    const raw = fd.get('filter_item_ids');
    if (typeof raw !== 'string') return undefined;
    const ids = raw.split(',').filter(Boolean);
    return ids;
  };
  const parseFilterIntegrations = (): unknown[] => {
    try {
      const raw = fd.get('filter_integrations');
      const parsed: unknown = JSON.parse(typeof raw === 'string' ? raw : '[]');
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  };
  const filter_item_ids = parseFilterItemIds();
  return {
    target: strOrNull('target'),
    caption: strOrNull('caption'),
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
    filter_integrations: parseFilterIntegrations(),
    ...(filter_item_ids !== undefined ? { filter_item_ids } : {}),
    last_step
  };
}

export const actions: Actions = {
  save_draft: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const r = await callPut(fetch, cookie, params.id!, patch);
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, '/account/frames');
  },

  save_changes_published: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const r = await callPut(fetch, cookie, params.id!, patch);
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/photo/${params.id}`);
  },

  publish: async ({ request, params, fetch, cookies }) => {
    // Verify is the single publish step (the /caption page was removed in
    // 56acf4e). Persist the form — including the caption — then flip the
    // photo to published, so publishing never strands unsaved edits.
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const saved = await callPut(fetch, cookie, params.id!, patch);
    if (!saved.ok) return fail(saved.status, { error: await saved.text() });
    const published = await fetch(`${API}/api/photos/${params.id}/publish`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
    if (!published.ok) return fail(published.status, { error: await published.text() });
    redirect(303, `/photo/${params.id}`);
  }
};
