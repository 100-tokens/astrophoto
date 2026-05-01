import type { Health } from './types';

const BASE = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

async function get<T>(path: string, fetchFn: typeof fetch = fetch): Promise<T> {
  const res = await fetchFn(`${BASE}${path}`, {
    credentials: 'include',
    headers: { Accept: 'application/json' }
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`GET ${path} ${res.status}: ${text}`);
  }
  return (await res.json()) as T;
}

export const api = {
  health: (f: typeof fetch = fetch) => get<Health>('/healthz', f)
};
