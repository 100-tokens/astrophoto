import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
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

// Report-only CSP: surfaces would-be violations in browser consoles without
// any breakage risk. Enforcing needs browser validation (SvelteKit's inline
// hydration bootstrap, the app.html font-loader's inline `onload` handler —
// both covered by 'unsafe-inline' below) and should graduate to kit.csp
// (mode 'auto', which nonces the bootstrap) once this policy stays quiet.
// img-src includes the image CDN origin when PUBLIC_CDN_BASE_URL is
// absolute (the '/cdn' dev fallback is same-origin, covered by 'self');
// connect-src allows *.amazonaws.com for the direct-to-S3 presigned PUT
// upload path, which bypasses the /api proxy by design.
const cdnOrigin = (() => {
  const base = env.PUBLIC_CDN_BASE_URL ?? '';
  if (!base.startsWith('http')) return '';
  try {
    return new URL(base).origin;
  } catch {
    return '';
  }
})();
const REPORT_ONLY_CSP = [
  `default-src 'self'`,
  `script-src 'self' 'unsafe-inline'`,
  `style-src 'self' 'unsafe-inline' https://fonts.googleapis.com`,
  `font-src 'self' https://fonts.gstatic.com`,
  `img-src 'self' data: blob:${cdnOrigin ? ` ${cdnOrigin}` : ''}`,
  `connect-src 'self' https://*.amazonaws.com${cdnOrigin ? ` ${cdnOrigin}` : ''}`,
  `frame-ancestors 'none'`
].join('; ');

export const handle: Handle = async ({ event, resolve }) => {
  // Apex → www redirect. The canonical host is www.astrophoto.pics; apex
  // visitors get a 301 so bookmarks and links collapse on one canonical host.
  // Only kicks in once the apex DNS resolves to this app — see infra runbook.
  if (event.url.host === 'astrophoto.pics') {
    redirect(301, `https://www.astrophoto.pics${event.url.pathname}${event.url.search}`);
  }

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
        tier: user.tier ?? 'free',
        avatarId: user.avatar_id ?? null,
        isAdmin: user.is_admin ?? false
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

  const response = await resolve(event, {
    // Global replace + done = true filter so the placeholder is robust to
    // chunk boundaries (the SvelteKit `transformPageChunk` is called per
    // chunk; a non-global `replace` would silently miss multi-occurrence).
    transformPageChunk: ({ html }) => html.replace(/%theme%/g, theme).replace(/%density%/g, density)
  });

  // Baseline security headers. Set each only if a downstream handler/route
  // hasn't already provided it, so route-specific overrides win.
  // NOTE: no enforcing Content-Security-Policy on purpose — enforcing one
  // risks breaking Google Fonts, the image CDN, and Svelte's inline styles.
  // The Report-Only variant (built above) is shipped instead to gather
  // violation data; graduating it to enforcing needs dedicated work
  // (nonces/hashes via kit.csp) and should land as its own change.
  const securityHeaders: Record<string, string> = {
    'Content-Security-Policy-Report-Only': REPORT_ONLY_CSP,
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'SAMEORIGIN',
    'Referrer-Policy': 'strict-origin-when-cross-origin',
    'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
    'Strict-Transport-Security': 'max-age=31536000; includeSubDomains'
  };
  for (const [name, value] of Object.entries(securityHeaders)) {
    if (!response.headers.has(name)) response.headers.set(name, value);
  }

  return response;
};
