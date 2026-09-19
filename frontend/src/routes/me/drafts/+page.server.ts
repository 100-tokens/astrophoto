import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { api } from '$lib/api/client';
import { signinUrl } from '$lib/auth/return-to';

export const load: PageServerLoad = async ({ locals, fetch, request, url }) => {
  if (!locals.user) redirect(303, signinUrl(url.pathname + url.search));
  const cookie = request.headers.get('cookie') ?? '';
  const cursor = url.searchParams.get('cursor') ?? undefined;
  const list = await api.drafts({ fetch, cookie, ...(cursor ? { cursor } : {}) });
  return { drafts: list };
};
