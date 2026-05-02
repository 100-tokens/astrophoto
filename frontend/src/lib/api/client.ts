import type { Health, User } from './types';

// ---------------------------------------------------------------------------
// Photo DTOs — mirror PhotoDetail in backend/src/photos/get.rs
// ---------------------------------------------------------------------------

export interface PhotoSummary {
  id: string;
  owner_id: string;
  status: string;
  width: number | null;
  height: number | null;
  target: string | null;
  caption: string | null;
  taken_at: string | null;
  created_at: string;
  camera: string | null;
  iso: number | null;
  exposure_s: number | null;
}

interface PhotoListResponse {
  photos: PhotoSummary[];
}

const BASE = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

interface ApiCall {
  fetch?: typeof fetch;
  cookie?: string; // server-side: forward request cookies
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown
  ) {
    super(`API ${status}`);
  }
}

async function request<T>(
  method: 'GET' | 'POST' | 'DELETE',
  path: string,
  body?: unknown,
  opts: ApiCall = {}
): Promise<T> {
  const f = opts.fetch ?? fetch;
  const headers: Record<string, string> = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (opts.cookie) headers['Cookie'] = opts.cookie;

  const res = await f(`${BASE}${path}`, {
    method,
    credentials: 'include',
    headers,
    ...(body !== undefined ? { body: JSON.stringify(body) } : {})
  });
  if (!res.ok) {
    let errBody: unknown;
    try {
      errBody = await res.json();
    } catch {
      errBody = await res.text();
    }
    throw new ApiError(res.status, errBody);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  health: (opts?: ApiCall) => request<Health>('GET', '/healthz', undefined, opts),
  signup: (body: { email: string; password: string; display_name: string }, opts?: ApiCall) =>
    request<User>('POST', '/api/auth/signup', body, opts),
  login: (body: { email: string; password: string }, opts?: ApiCall) =>
    request<User>('POST', '/api/auth/login', body, opts),
  logout: (opts?: ApiCall) => request<void>('POST', '/api/auth/logout', undefined, opts),
  me: (opts?: ApiCall) => request<User>('GET', '/api/auth/me', undefined, opts),
  photos: {
    list: (ownerId?: string, opts?: ApiCall) => {
      const qs = ownerId ? `?owner_id=${encodeURIComponent(ownerId)}` : '';
      return request<PhotoListResponse>('GET', `/api/photos${qs}`, undefined, opts);
    },
    get: (id: string, opts?: ApiCall) =>
      request<PhotoSummary>('GET', `/api/photos/${id}`, undefined, opts)
    // upload uses multipart/form-data — callers use raw fetch directly
  }
};
