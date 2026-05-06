import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * `/targets` is the user-facing URL form (matches the "TARGETS" nav label),
 * but the route lives at `/t/*` to match the existing `/t/[slug]` detail
 * pages. Permanent redirect to keep both forms resolvable.
 */
export const load: PageServerLoad = () => {
  redirect(308, '/t');
};
