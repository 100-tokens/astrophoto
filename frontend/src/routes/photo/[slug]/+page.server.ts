import { error, fail } from '@sveltejs/kit';
import { PHOTOS, NGC7000 } from '$lib/data/photos';
import type { PageServerLoad, Actions } from './$types';
import type { PhotoDetail } from '$lib/data/photos';
import type { Comment } from '$lib/api/client';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

interface ExifRow {
  label: string;
  value: string;
  sublabel?: string;
  sublabelAccent?: boolean;
}

interface RealPhoto {
  id: string;
  owner_id: string;
  status: string;
  original_name: string;
  bytes: number;
  mime: string;
  width: number | null;
  height: number | null;
  taken_at: string | null;
  camera: string | null;
  lens: string | null;
  iso: number | null;
  exposure_s: number | null;
  focal_mm: number | null;
  target: string | null;
  caption: string | null;
  created_at: string;
  appreciation_count: number;
  comment_count: number;
  is_draft: boolean;
  last_step: string | null;
  replaced_at: string | null;
  original_uploaded_at: string;
}

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function formatExposure(s: number): string {
  if (s >= 1) return `${s} s`;
  if (s <= 0) return `${s} s`;
  return `1/${Math.round(1 / s)} s`;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toISOString().slice(0, 10);
}

function buildExifRows(p: RealPhoto): ExifRow[] {
  const rows: ExifRow[] = [];
  rows.push({ label: 'Original file', value: p.original_name });
  rows.push({ label: 'Size', value: formatBytes(p.bytes) });
  if (p.width != null && p.height != null) {
    rows.push({ label: 'Dimensions', value: `${p.width} × ${p.height}` });
  }
  if (p.target) rows.push({ label: 'Target', value: p.target });
  if (p.taken_at) rows.push({ label: 'Captured', value: formatDate(p.taken_at) });
  if (p.camera) rows.push({ label: 'Camera', value: p.camera });
  if (p.lens) rows.push({ label: 'Lens', value: p.lens });
  if (p.iso != null) rows.push({ label: 'ISO', value: String(p.iso) });
  if (p.exposure_s != null) {
    rows.push({ label: 'Exposure', value: formatExposure(p.exposure_s) });
  }
  if (p.focal_mm != null) {
    rows.push({ label: 'Focal', value: `${p.focal_mm} mm` });
  }
  return rows;
}

/** Minimal photo detail shape for gallery photos that lack rich EXIF data. */
function minimalDetail(
  target: string,
  integration: string,
  photographerName: string,
  slug: string,
  ratio: number
): PhotoDetail {
  return {
    slug,
    target,
    targetSubtitle: '',
    captured: '',
    camera: '',
    cameraSub: '',
    telescope: '',
    telescopeSub: '',
    mount: '',
    filters: '',
    exposure: '',
    exposureTotal: '',
    gain: '',
    ra: '',
    dec: '',
    field: '',
    pixelScale: '',
    publishedDate: '',
    photographer: {
      name: photographerName,
      initial: photographerName.charAt(0).toUpperCase(),
      frames: 0,
      bortle: 0,
      location: '',
      caption: ''
    },
    appreciations: 0,
    comments: 0,
    ratio,
    integration
  };
}

export const load: PageServerLoad = async ({ params, fetch, locals, request }) => {
  const { slug } = params;

  // Canonical NGC 7000 placeholder
  if (slug === 'ngc-7000-north-america-nebula') {
    return { photo: NGC7000, isRich: true, thumbSrc1200: undefined };
  }

  // Real photo by UUID
  if (UUID_RE.test(slug)) {
    const res = await fetch(`${API}/api/photos/${slug}`);
    if (!res.ok) {
      if (res.status === 404) {
        error(404, 'Photo not found');
      }
      error(500, 'Failed to load photo');
    }
    const photo = (await res.json()) as RealPhoto;

    let isAppreciated = false;
    if (locals.user) {
      try {
        const cookie = request.headers.get('cookie') ?? '';
        const stateRes = await fetch(`${API}/api/photos/${params.slug}/appreciation-state`, {
          headers: { Cookie: cookie }
        });
        if (stateRes.ok) {
          const state = (await stateRes.json()) as { appreciated: boolean };
          isAppreciated = state.appreciated;
        }
      } catch {
        // ignore
      }
    }

    let comments: Comment[] = [];
    try {
      const res = await fetch(`${API}/api/photos/${params.slug}/comments`);
      if (res.ok) {
        const body = (await res.json()) as { comments: Comment[] };
        comments = body.comments;
      }
    } catch {
      // ignore — backend down, render with empty comments
    }

    // Look up the owner's public profile so the photographer card shows
    // the actual display name + avatar initial + a working /u/{uuid} link
    // instead of the placeholder "User" / "/u/user".
    let ownerName = 'User';
    let ownerInitial = 'U';
    try {
      const r = await fetch(`${API}/api/users/${photo.owner_id}`);
      if (r.ok) {
        const u = (await r.json()) as { display_name: string };
        ownerName = u.display_name;
        ownerInitial = u.display_name.charAt(0).toUpperCase();
      }
    } catch {
      // ignore — fall back to placeholder
    }
    // If the viewer is the owner, prefer locals.user for fresh display name.
    if (locals.user?.id === photo.owner_id) {
      ownerName = locals.user.displayName;
      ownerInitial = ownerName.charAt(0).toUpperCase();
    }

    const detail: PhotoDetail = {
      slug: photo.id,
      id: photo.id,
      owner_id: photo.owner_id,
      target: photo.target ?? 'Untitled',
      targetSubtitle: '',
      captured: photo.taken_at ?? '',
      camera: photo.camera ?? '',
      cameraSub: '',
      telescope: '',
      telescopeSub: '',
      mount: '',
      filters: '',
      exposure: photo.exposure_s != null ? formatExposure(photo.exposure_s) : '',
      exposureTotal: '',
      gain: photo.iso != null ? String(photo.iso) : '',
      ra: '',
      dec: '',
      field: '',
      pixelScale: '',
      publishedDate: '',
      photographer: {
        id: photo.owner_id,
        name: ownerName,
        initial: ownerInitial,
        frames: 0,
        bortle: 0,
        location: '',
        caption: photo.caption ?? '',
        captionShort: photo.caption ?? ''
      },
      appreciations: photo.appreciation_count,
      comments: photo.comment_count,
      ratio: photo.width && photo.height ? photo.width / photo.height : 1.5,
      integration: '',
      is_draft: photo.is_draft,
      last_step: photo.last_step,
      replaced_at: photo.replaced_at,
      original_uploaded_at: photo.original_uploaded_at
    };

    return {
      photo: detail,
      isRich: false,
      thumbSrc1200: `${API}/api/photos/${photo.id}/thumb/1200`,
      exifRows: buildExifRows(photo),
      isAppreciated,
      comments,
      ownerId: photo.owner_id,
      current_user_id: locals.user?.id ?? null
    };
  }

  // Placeholder gallery photo by slug
  const match = PHOTOS.find((p) => p.slug === slug);
  if (!match) {
    error(404, 'Photo not found');
  }

  return {
    photo: minimalDetail(
      match.target,
      match.integration,
      match.photographer,
      match.slug,
      match.ratio
    ),
    isRich: false,
    thumbSrc1200: undefined
  };
};

export const actions: Actions = {
  comment: async ({ request, params, fetch, cookies }) => {
    const data = await request.formData();
    const body = String(data.get('body') ?? '').trim();
    if (body.length === 0 || body.length > 2000) {
      return fail(422, { message: 'Comment must be 1-2000 chars.' });
    }
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const res = await fetch(`${API}/api/photos/${params.slug}/comments`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ body })
    });
    if (!res.ok) {
      return fail(res.status, { message: `Failed: ${await res.text()}` });
    }
    return { ok: true };
  },

  deleteComment: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(400, { message: 'Missing comment id.' });
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');
    const res = await fetch(`${API}/api/comments/${id}`, {
      method: 'DELETE',
      credentials: 'include',
      headers: { Cookie: cookie }
    });
    if (!res.ok) {
      return fail(res.status, { message: `Failed: ${await res.text()}` });
    }
    return { ok: true };
  }
};
