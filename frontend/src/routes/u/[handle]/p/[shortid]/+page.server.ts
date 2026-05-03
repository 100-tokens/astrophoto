import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const { handle, shortid } = params;

  // Resolve short_id + handle → photo UUID via the by-permalink endpoint.
  const r = await fetch(`${API}/api/photos/by-permalink/${handle}/${shortid}`);
  if (!r.ok) throw error(404, 'Photo not found');
  const { id } = (await r.json()) as { id: string };

  // Fetch full photo detail by UUID.
  const photoR = await fetch(`${API}/api/photos/${id}`);
  if (!photoR.ok) throw error(404, 'Photo not found');

  return { photo: await photoR.json() };
};
