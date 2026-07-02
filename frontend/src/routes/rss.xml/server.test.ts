import { describe, it, expect } from 'vitest';
import { env } from '$env/dynamic/public';
import { GET } from './+server';

// The enclosure/media URLs must be the same CDN URLs the site's <img>
// tags use — the old fallback pointed at /api/photos/<id>/thumb/1200,
// which 404s for photos without stored thumbnails (every XISF upload).
// Mirrors cdn.test.ts: assert against the resolved base rather than a
// hard-coded value so the test passes with or without .env.local.
const BASE = env.PUBLIC_CDN_BASE_URL ?? '/cdn';

const PHOTO = {
  id: '390f0d5f-3f85-4317-b813-379801e02e59',
  short_id: 'PVEhvjtj',
  target: 'NGC 5982',
  width: 2927,
  height: 2926,
  published_at: '2026-06-28T17:55:33.176713Z',
  author_handle: 'pascalleclech',
  author_display_name: 'Pascal Le Clech'
};

function mockEvent(photos: unknown[]) {
  const fetchImpl = (async () =>
    new Response(JSON.stringify({ photos }), {
      headers: { 'content-type': 'application/json' }
    })) as typeof fetch;
  return {
    url: new URL('https://www.astrophoto.pics/rss.xml'),
    fetch: fetchImpl
  } as unknown as Parameters<typeof GET>[0];
}

describe('rss.xml', () => {
  it('uses the CDN image URL, never the thumb API route', async () => {
    const res = await GET(mockEvent([PHOTO]));
    const xml = await res.text();

    expect(xml).not.toContain('/api/photos/');
    const expected = BASE.startsWith('http')
      ? `${BASE}/img/${PHOTO.id}?w=1200`
      : `https://www.astrophoto.pics${BASE}/img/${PHOTO.id}?w=1200`;
    expect(xml).toContain(`<enclosure url="${expected.replace(/&/g, '&amp;')}"`);
    expect(xml).toContain(`<media:content url="${expected.replace(/&/g, '&amp;')}"`);
  });

  it('emits a valid channel with the photo item', async () => {
    const res = await GET(mockEvent([PHOTO]));
    const xml = await res.text();
    expect(res.headers.get('content-type')).toContain('application/rss+xml');
    expect(xml).toContain('<title>NGC 5982</title>');
    expect(xml).toContain(
      '<guid isPermaLink="true">https://www.astrophoto.pics/u/pascalleclech/p/PVEhvjtj</guid>'
    );
  });

  it('fails soft to an empty feed when the API is down', async () => {
    const failingFetch = (async () => {
      throw new Error('backend down');
    }) as unknown as typeof fetch;
    const event = {
      url: new URL('https://www.astrophoto.pics/rss.xml'),
      fetch: failingFetch
    } as unknown as Parameters<typeof GET>[0];
    const res = await GET(event);
    expect(res.status).toBe(200);
    const xml = await res.text();
    expect(xml).toContain('<channel>');
    expect(xml).not.toContain('<item>');
  });
});
