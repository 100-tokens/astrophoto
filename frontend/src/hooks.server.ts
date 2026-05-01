import type { Handle } from '@sveltejs/kit';

// Reads the session cookie and resolves event.locals.user.
// MVP: stub. Real session resolution arrives with the auth feature.
export const handle: Handle = async ({ event, resolve }) => {
  event.locals.user = null;
  return resolve(event);
};
