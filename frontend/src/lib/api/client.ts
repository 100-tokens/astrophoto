import type { Health, User } from './types';

// ---------------------------------------------------------------------------
// Comment DTO
// ---------------------------------------------------------------------------

export interface Comment {
  id: string;
  photo_id: string;
  author_id: string | null;  // null = author account was deleted (pseudonymised)
  author_display_name: string;
  body: string;
  created_at: string;
}

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
    list: (
      opts: { ownerId?: string; limit?: number; following?: boolean } = {},
      apiOpts?: ApiCall
    ) => {
      const qs = new URLSearchParams();
      if (opts.ownerId) qs.set('owner_id', opts.ownerId);
      if (opts.limit != null) qs.set('limit', String(opts.limit));
      if (opts.following) qs.set('following', 'true');
      const path = qs.toString() ? `/api/photos?${qs}` : '/api/photos';
      return request<PhotoListResponse>('GET', path, undefined, apiOpts);
    },
    get: (id: string, opts?: ApiCall) =>
      request<PhotoSummary>('GET', `/api/photos/${id}`, undefined, opts)
    // upload uses multipart/form-data — callers use raw fetch directly
  },
  appreciations: {
    count: (photoId: string, opts?: ApiCall) =>
      request<{ count: number }>(
        'GET',
        `/api/photos/${photoId}/appreciations/count`,
        undefined,
        opts
      ),
    state: (photoId: string, opts?: ApiCall) =>
      request<{ appreciated: boolean }>(
        'GET',
        `/api/photos/${photoId}/appreciation-state`,
        undefined,
        opts
      ),
    appreciate: (photoId: string, opts?: ApiCall) =>
      request<void>('POST', `/api/photos/${photoId}/appreciate`, undefined, opts),
    unappreciate: (photoId: string, opts?: ApiCall) =>
      request<void>('DELETE', `/api/photos/${photoId}/appreciate`, undefined, opts)
  },
  follows: {
    follow: (userId: string, opts?: ApiCall) =>
      request<void>('POST', `/api/users/${userId}/follow`, undefined, opts),
    unfollow: (userId: string, opts?: ApiCall) =>
      request<void>('DELETE', `/api/users/${userId}/follow`, undefined, opts),
    followersCount: (userId: string, opts?: ApiCall) =>
      request<{ count: number }>('GET', `/api/users/${userId}/followers/count`, undefined, opts),
    followingCount: (userId: string, opts?: ApiCall) =>
      request<{ count: number }>('GET', `/api/users/${userId}/following/count`, undefined, opts)
  },
  comments: {
    list: (photoId: string, opts?: ApiCall) =>
      request<{ comments: Comment[] }>('GET', `/api/photos/${photoId}/comments`, undefined, opts),
    create: (photoId: string, body: string, opts?: ApiCall) =>
      request<Comment>('POST', `/api/photos/${photoId}/comments`, { body }, opts),
    delete: (commentId: string, opts?: ApiCall) =>
      request<void>('DELETE', `/api/comments/${commentId}`, undefined, opts)
  }
};
