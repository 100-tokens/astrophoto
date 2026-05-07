import type { Health, User } from './types';
import type { PhotoDetail } from './PhotoDetail';
import type { DraftListResponse } from './DraftListResponse';
import type { BatchApplyResponse } from './BatchApplyResponse';
import type { BatchPublishResponse } from './BatchPublishResponse';

// ---------------------------------------------------------------------------
// Comment DTO
// ---------------------------------------------------------------------------

export interface Comment {
  id: string;
  photo_id: string;
  author_id: string | null; // null = author account was deleted (pseudonymised)
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
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
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
      request<PhotoSummary>('GET', `/api/photos/${id}`, undefined, opts),
    getDetail: (id: string, opts?: ApiCall) =>
      request<PhotoDetail>('GET', `/api/photos/${id}`, undefined, opts),
    putMetadata: (id: string, patch: Record<string, unknown>, opts?: ApiCall) =>
      request<void>('PUT', `/api/photos/${id}`, patch, opts),
    publish: (id: string, opts?: ApiCall) =>
      request<void>('POST', `/api/photos/${id}/publish`, undefined, opts),
    delete: (id: string, opts?: ApiCall) =>
      request<void>('DELETE', `/api/photos/${id}`, undefined, opts)
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
  },

  passwordResetRequest: (email: string, opts?: ApiCall) =>
    request<void>('POST', '/api/auth/password-reset/request', { email }, opts),

  passwordResetConfirm: (token: string, new_password: string, opts?: ApiCall) =>
    request<void>('POST', '/api/auth/password-reset/confirm', { token, new_password }, opts),

  emailChangeConfirm: (token: string, opts?: ApiCall) =>
    request<{ status: 'success' | 'expired' | 'taken' }>(
      'POST',
      '/api/auth/email-change/confirm',
      { token },
      opts
    ),

  getProfile: (opts?: ApiCall) =>
    request<{ display_name: string }>('GET', '/api/me/profile', undefined, opts),

  putProfile: (body: { display_name?: string }, opts?: ApiCall) =>
    request<void>('PUT', '/api/me/profile', body, opts),

  getPreferences: (opts?: ApiCall) =>
    request<{ theme: string; density: string }>('GET', '/api/me/preferences', undefined, opts),

  putPreferences: (body: { theme?: string; density?: string }, opts?: ApiCall) =>
    request<void>('PUT', '/api/me/preferences', body, opts),

  listSessions: (opts?: ApiCall) =>
    request<import('./SessionRow').SessionRow[]>('GET', '/api/me/sessions', undefined, opts),

  revokeSession: (id: string, opts?: ApiCall) =>
    request<void>('DELETE', `/api/me/sessions/${encodeURIComponent(id)}`, undefined, opts),

  signOutOthers: (opts?: ApiCall) =>
    request<void>('POST', '/api/me/sessions/sign-out-others', undefined, opts),

  requestEmailChange: (new_email: string, current_password: string, opts?: ApiCall) =>
    request<void>('POST', '/api/me/email-change/request', { new_email, current_password }, opts),

  changePassword: (body: { current_password?: string; new_password: string }, opts?: ApiCall) =>
    request<void>('POST', '/api/me/password-change', body, opts),

  requestDeletion: (
    body: { current_password?: string; confirmation_phrase: string },
    opts?: ApiCall
  ) => request<void>('POST', '/api/me/delete-request', body, opts),

  cancelDeletion: (opts?: ApiCall) =>
    request<void>('POST', '/api/me/delete-cancel', undefined, opts),

  photosCount: (opts?: ApiCall) =>
    request<{ count: number }>('GET', '/api/me/photos/count', undefined, opts),

  drafts: (opts: ApiCall & { limit?: number; cursor?: string } = {}) => {
    const { limit, cursor, ...apiOpts } = opts;
    const qs = new URLSearchParams();
    if (limit != null) qs.set('limit', String(limit));
    if (cursor) qs.set('cursor', cursor);
    const path = qs.toString() ? `/api/photos/me/drafts?${qs}` : '/api/photos/me/drafts';
    return request<DraftListResponse>('GET', path, undefined, apiOpts);
  }
};

// ---------------------------------------------------------------------------
// Standalone helpers for the upload wizard (server-side load + actions)
// ---------------------------------------------------------------------------

export async function getPhoto(id: string, opts: ApiCall = {}): Promise<PhotoDetail> {
  return request<PhotoDetail>('GET', `/api/photos/${id}`, undefined, opts);
}

export async function batchApply(
  opts: ApiCall & { ids: string[]; target?: string; tags?: string[] }
): Promise<BatchApplyResponse> {
  const { ids, target, tags, ...apiOpts } = opts;
  return request<BatchApplyResponse>(
    'POST',
    '/api/photos/batch/apply',
    {
      ids,
      target: target ?? null,
      tags: tags ?? null
    },
    apiOpts
  );
}

export async function putPhotoMetadata(
  id: string,
  patch: Record<string, unknown>,
  opts: ApiCall = {}
): Promise<void> {
  await request<void>('PUT', `/api/photos/${id}`, patch, opts);
}

export async function publishPhoto(id: string, opts: ApiCall = {}): Promise<void> {
  await request<void>('POST', `/api/photos/${id}/publish`, undefined, opts);
}

export async function batchPublish(
  opts: ApiCall & { ids: string[] }
): Promise<BatchPublishResponse> {
  const { ids, ...apiOpts } = opts;
  return request<BatchPublishResponse>('POST', '/api/photos/batch/publish', { ids }, apiOpts);
}
