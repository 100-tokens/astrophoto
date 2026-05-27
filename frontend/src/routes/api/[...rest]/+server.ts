// SvelteKit catch-all reverse proxy for /api/*.
//
// Why: in staging/prod the frontend (e.g. *-web-*.koyeb.app) and backend
// (e.g. *-008151d0.koyeb.app) live on different sibling subdomains with no
// shared parent. The auth flow stamps the session cookie on the frontend
// domain (SvelteKit signin/signup form actions read the backend's
// Set-Cookie and replay it via cookies.set, which scopes to the SvelteKit
// origin). The backend's own origin holds no cookie. So a browser-side
// `fetch('https://backend.../api/...')` sends an empty cookie jar to
// the backend → 401.
//
// Routing all browser→API traffic through this same-origin proxy
// preserves the same-cookie story the SSR loaders already rely on:
// SvelteKit reads its own cookies, forwards them as a `Cookie:` header
// to the backend, and pipes the response back. No CORS, no SameSite=None
// gymnastics required for the API path.
//
// What stays direct:
//   - presigned PUT to S3 (different host, no auth header needed; the
//     URL is itself the credential)
//   - CloudFront image fetches (PUBLIC_CDN_BASE_URL points at CDN)

import type { RequestHandler } from './$types';
import { env } from '$env/dynamic/private';

// Server-only env: full backend origin. Set on the Koyeb frontend service.
// Falls back to localhost for `pnpm dev`. Distinct from VITE_API_BASE_URL,
// which is the *client-bundle* base — that one stays empty/relative so
// browser fetches land on this proxy instead of going cross-origin.
const API = env.BACKEND_URL ?? 'http://localhost:8080';

// Hop-by-hop headers per RFC 7230 §6.1 — must not be forwarded.
const HOP_BY_HOP = new Set([
  'connection',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailer',
  'transfer-encoding',
  'upgrade',
  'host',
  'content-length'
]);

function buildCookieHeader(cookies: import('@sveltejs/kit').Cookies): string {
  return cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
}

const proxy: RequestHandler = async ({ request, params, fetch, cookies, getClientAddress }) => {
  const path = params.rest ?? '';
  const search = new URL(request.url).search;
  const targetUrl = `${API}/api/${path}${search}`;

  // Copy headers, dropping hop-by-hop ones, then attach forwarded auth.
  const headers = new Headers();
  for (const [k, v] of request.headers.entries()) {
    if (HOP_BY_HOP.has(k.toLowerCase())) continue;
    if (k.toLowerCase() === 'cookie') continue;
    headers.set(k, v);
  }
  const cookieHeader = buildCookieHeader(cookies);
  if (cookieHeader) headers.set('cookie', cookieHeader);

  // Preserve client IP for backend rate-limiting / audit logs.
  const clientIp = getClientAddress();
  if (clientIp) {
    const existing = request.headers.get('x-forwarded-for');
    headers.set('x-forwarded-for', existing ? `${existing}, ${clientIp}` : clientIp);
  }

  // `duplex` is required by undici to send a streaming request body but is
  // not yet in the DOM lib's RequestInit — widen the type locally instead
  // of suppressing with a directive (which would itself error if the lib
  // ever adds it).
  const init: RequestInit & { duplex?: 'half' } = {
    method: request.method,
    headers,
    redirect: 'manual'
  };
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    // Stream the body straight through rather than buffering it with
    // arrayBuffer(): the plate-solve re-solve and photo-replace flows POST
    // multi-MB files, and buffering would spike proxy memory to ~2× the
    // body, resident, on an unbounded concurrency layer. Streaming keeps it
    // flat. NOTE: this is only half the fix — adapter-node's BODY_SIZE_LIMIT
    // (default 512 KB) is enforced on the incoming stream regardless of
    // streaming, so it must be raised on the frontend service in tandem.
    // See CLAUDE.md "Gotchas".
    init.body = request.body;
    init.duplex = 'half';
  }

  const upstream = await fetch(targetUrl, init);

  // Strip hop-by-hop on the way back; forward Set-Cookie verbatim so the
  // browser stores it (it'll be scoped to this SvelteKit origin, which is
  // exactly what the rest of the app expects).
  const respHeaders = new Headers();
  for (const [k, v] of upstream.headers.entries()) {
    if (HOP_BY_HOP.has(k.toLowerCase())) continue;
    respHeaders.append(k, v);
  }

  return new Response(upstream.body, {
    status: upstream.status,
    statusText: upstream.statusText,
    headers: respHeaders
  });
};

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const PATCH = proxy;
export const DELETE = proxy;
export const HEAD = proxy;
export const OPTIONS = proxy;
