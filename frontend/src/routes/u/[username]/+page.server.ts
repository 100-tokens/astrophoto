import { error } from '@sveltejs/kit';
import { PHOTOS, MARIE } from '$lib/data/photos';
import type { PageServerLoad } from './$types';
import type { User } from '$lib/data/photos';

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

export const load: PageServerLoad = async ({ params }) => {
	const { username } = params;

	if (username === 'marie-dubois') {
		return {
			user: MARIE,
			photos: PHOTOS.slice(0, 8)
		};
	}

	// Try to find a photographer in the gallery photos
	const match = PHOTOS.find((p) => p.photographerSlug === username);
	if (!match) {
		throw error(404, 'User not found');
	}

	// Return a minimal user with photos attributed to them
	const userPhotos = PHOTOS.filter((p) => p.photographerSlug === username);
	return {
		user: minimalUser(username),
		photos: userPhotos.slice(0, 8)
	};
};
