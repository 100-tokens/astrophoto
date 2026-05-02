import { error } from '@sveltejs/kit';
import { PHOTOS, MARIE } from '$lib/data/photos';
import type { PageServerLoad } from './$types';
import type { User, Photo } from '$lib/data/photos';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Minimal user shape for unknown usernames. */
function minimalUser(username: string): User {
  const displayName = username
    .split('-')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
  return {
    username,
    displayName,
    firstName: displayName.split(' ')[0] ?? displayName,
    surnameItalic: displayName.split(' ').slice(1).join(' ') || displayName,
    initial: displayName.charAt(0).toUpperCase(),
    about: 'Amateur astrophotographer.',
    frames: 0,
    integrationTotal: '—',
    followers: 0,
    collections: 0,
    lat: '—',
    long: '—',
    bortle: 0,
    sqm: 0,
    equipment: { scope: '—', camera: '—', mount: '—', filters: '—' },
    memberSince: '2026'
  };
}

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { username } = params;

  // Real user — UUID slug
  if (UUID_RE.test(username)) {
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

    // If the logged-in user is viewing their own profile, use their display name
    const localUser = locals.user;
    let displayName = 'User';
    if (localUser && localUser.id === username) {
      displayName = localUser.displayName;
    }
    const firstName = displayName.split(' ')[0] ?? displayName;
    const surname = displayName.split(' ').slice(1).join(' ') || displayName;

    const user: User = {
      username,
      displayName,
      firstName,
      surnameItalic: surname,
      initial: displayName.charAt(0).toUpperCase(),
      about: '',
      frames: photos.length,
      integrationTotal: '—',
      followers: 0,
      collections: 0,
      lat: '—',
      long: '—',
      bortle: 0,
      sqm: 0,
      equipment: { scope: '—', camera: '—', mount: '—', filters: '—' },
      memberSince: '2026'
    };

    return { user, photos };
  }

  // Placeholder canonical user
  if (username === 'marie-dubois') {
    return {
      user: MARIE,
      photos: PHOTOS.slice(0, 8)
    };
  }

  // Try to find a photographer in the gallery photos
  const match = PHOTOS.find((p) => p.photographerSlug === username);
  if (!match) {
    error(404, 'User not found');
  }

  // Return a minimal user with photos attributed to them
  const userPhotos = PHOTOS.filter((p) => p.photographerSlug === username);
  return {
    user: minimalUser(username),
    photos: userPhotos.slice(0, 8)
  };
};
