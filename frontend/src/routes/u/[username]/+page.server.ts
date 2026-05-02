import { error } from '@sveltejs/kit';
import { PHOTOS, MARIE } from '$lib/data/photos';
import type { PageServerLoad } from './$types';
import type { User, Photo } from '$lib/data/photos';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

interface UserPublic {
  id: string;
  display_name: string;
  created_at: string;
  photo_count: number;
}

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { username } = params;

  // Real user — UUID slug
  if (UUID_RE.test(username)) {
    let displayName = 'User';
    let photoCount = 0;
    let memberSince = '2026';

    try {
      const res = await fetch(`${API}/api/users/${username}`);
      if (res.status === 404) {
        throw error(404, 'User not found');
      }
      if (res.ok) {
        const u = (await res.json()) as UserPublic;
        displayName = u.display_name;
        photoCount = u.photo_count;
        memberSince = new Date(u.created_at).getFullYear().toString();
      }
    } catch (e) {
      // Re-throw SvelteKit errors/redirects.
      if (e && typeof e === 'object' && 'status' in e) throw e;
      // Network error: fall through with defaults.
    }

    // If the viewer is themselves the owner, prefer locals.user displayName
    // (most up-to-date, avoids a stale cache).
    if (locals.user?.id === username) {
      displayName = locals.user.displayName;
    }

    let followerCount = 0;
    try {
      const res = await fetch(`${API}/api/users/${username}/followers/count`);
      if (res.ok) {
        const body = (await res.json()) as { count: number };
        followerCount = body.count;
      }
    } catch {
      // ignore — keep default 0
    }

    let photos: Photo[] = [];
    try {
      const res = await fetch(`${API}/api/photos?owner_id=${username}&limit=24`);
      if (res.ok) {
        const body = (await res.json()) as {
          photos: Array<{
            id: string;
            target: string | null;
            width: number | null;
            height: number | null;
          }>;
        };
        photos = body.photos.map((p) => ({
          slug: p.id,
          target: p.target ?? 'Untitled',
          ratio: p.width && p.height ? p.width / p.height : 1.5,
          integration: '',
          photographer: '',
          photographerSlug: '',
          camera: '',
          thumbSrc: `${API}/api/photos/${p.id}/thumb/400`
        }));
      }
    } catch {
      // backend down — return empty gallery
    }

    const parts = displayName.split(' ');
    const firstName = parts[0] ?? displayName;
    const surnameItalic = parts.slice(1).join(' ') || displayName;

    const user: User = {
      username,
      displayName,
      firstName,
      surnameItalic,
      initial: displayName[0]?.toUpperCase() ?? 'U',
      about: '',
      frames: photoCount,
      integrationTotal: '—',
      followers: followerCount,
      collections: 0,
      lat: '—',
      long: '—',
      bortle: 0,
      sqm: 0,
      equipment: { scope: '—', camera: '—', mount: '—', filters: '—' },
      memberSince
    };

    const isFollowing = locals.user?.following_ids?.includes(username) ?? false;
    const isSelf = locals.user?.id === username;

    // Field name 'profile' (not 'user') to avoid colliding with the layout's
    // `data.user` (the auth state). Layout-level `user` must keep flowing
    // through to AppHeader's $app/state read.
    return { profile: user, photos, isReal: true as const, isFollowing, isSelf };
  }

  // Placeholder canonical user
  if (username === 'marie-dubois') {
    return {
      profile: MARIE,
      photos: PHOTOS.slice(0, 8),
      isReal: false as const
    };
  }

  throw error(404, 'User not found');
};
