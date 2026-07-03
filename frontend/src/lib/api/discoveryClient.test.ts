import { describe, it, expect } from 'vitest';
import { fetchExplore } from './discoveryClient';

// Pin the query-string contract the /explore loader depends on: which
// params are emitted, which are dropped, and that non-ok responses
// throw (the loader's allowlist keeps user input from ever triggering
// that throw, but backend failures still must).
function mockFetch(capture: { url?: string }, status = 200) {
  return (async (input: RequestInfo | URL) => {
    capture.url = String(input);
    return new Response(JSON.stringify({ photos: [], next_cursor: null }), {
      status,
      headers: { 'content-type': 'application/json' }
    });
  }) as typeof fetch;
}

describe('fetchExplore', () => {
  it('emits set params and drops empty/false ones', async () => {
    const cap: { url?: string } = {};
    await fetchExplore(mockFetch(cap), {
      sort: 'most-appreciated',
      since: '7d',
      category: 'wide_field',
      following: false,
      limit: 24
    });
    const url = new URL(cap.url!, 'http://localhost');
    expect(url.pathname).toBe('/api/explore');
    expect(url.searchParams.get('sort')).toBe('most-appreciated');
    expect(url.searchParams.get('since')).toBe('7d');
    expect(url.searchParams.get('category')).toBe('wide_field');
    expect(url.searchParams.get('limit')).toBe('24');
    expect(url.searchParams.has('following')).toBe(false);
  });

  it('carries the cursor through untouched', async () => {
    const cap: { url?: string } = {};
    await fetchExplore(mockFetch(cap), { cursor: 'abc+def==', limit: 24 });
    const url = new URL(cap.url!, 'http://localhost');
    expect(url.searchParams.get('cursor')).toBe('abc+def==');
  });

  it('throws on non-ok responses', async () => {
    const cap: { url?: string } = {};
    await expect(fetchExplore(mockFetch(cap, 400), { limit: 24 })).rejects.toThrow(
      /fetchExplore 400/
    );
  });
});
