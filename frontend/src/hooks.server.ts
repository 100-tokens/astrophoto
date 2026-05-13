import { redirect } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';
import type { Handle } from '@sveltejs/kit';

// The backend issues `__Host-session=` over HTTPS (prod) and the unprefixed
// `session=` over plain HTTP (dev); browsers reject the `__Host-` prefix
// without Secure. Match either to decide whether to call /api/auth/me.
const SESSION_COOKIE_RE = /(?:^|;\s*)(?:__Host-session|session)=/;

function parseCookie(header: string, name: string): string | null {
  const re = new RegExp('(?:^|;\\s*)' + name + '=([^;]+)');
  const m = header.match(re);
  return m ? decodeURIComponent(m[1] ?? '') : null;
}

export const handle: Handle = async ({ event, resolve }) => {
  // Redirect removed /caption route to /verify so old bookmarks keep working.
  const captionMatch = event.url.pathname.match(/^\/upload\/([0-9a-f-]{36})\/caption\/?$/);
  if (captionMatch) {
    redirect(301, `/upload/${captionMatch[1]}/verify`);
  }

  const cookie = event.request.headers.get('cookie') ?? '';

  // Short-circuit auth lookup on /api/* requests. Those are handled by the
  // SvelteKit reverse proxy at routes/api/[...rest]/+server.ts which calls
  // the backend directly with the original cookie — re-running api.me here
  // would mean handle() → api.me → event.fetch('/api/auth/me') → handle()
  // again (subrequests re-enter the hook), an unbounded recursion that
  // hangs every request.
  const isApiPath = event.url.pathname.startsWith('/api/');

  if (!isApiPath && SESSION_COOKIE_RE.test(cookie)) {
    try {
      const user = await api.me({ fetch: event.fetch, cookie });
      event.locals.user = {
        id: user.id,
        email: user.email,
        displayName: user.display_name,
        handle: user.handle,
        following_ids: user.following_ids ?? [],
        pending_deletion_at: user.pending_deletion_at ?? null,
        tier: user.tier ?? 'free'
      };
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

  const themeCookie = parseCookie(cookie, 'theme');
  const densityCookie = parseCookie(cookie, 'density');
  const theme = (themeCookie === 'light' ? 'light' : 'dark') as 'dark' | 'light';
  const density = (densityCookie === 'data' ? 'data' : 'work') as 'work' | 'data';
  event.locals.preferences = { theme, density };

  return resolve(event, {
    // Global replace + done = true filter so the placeholder is robust to
    // chunk boundaries (the SvelteKit `transformPageChunk` is called per
    // chunk; a non-global `replace` would silently miss multi-occurrence).
    transformPageChunk: ({ html }) => html.replace(/%theme%/g, theme).replace(/%density%/g, density)
  });
};
