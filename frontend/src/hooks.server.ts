import { api, ApiError } from '$lib/api/client';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  const cookie = event.request.headers.get('cookie') ?? '';
  if (cookie.includes('__Host-session=')) {
    try {
      const user = await api.me({ fetch: event.fetch, cookie });
      event.locals.user = { id: user.id, displayName: user.display_name };
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        event.locals.user = null;
      } else {
        // Don't break the page on transient backend errors.
        event.locals.user = null;
      }
    }
  } else {
    event.locals.user = null;
  }
  return resolve(event);
};
