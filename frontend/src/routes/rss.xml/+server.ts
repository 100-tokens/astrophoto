import type { RequestHandler } from './$types';

// Site-wide RSS 2.0 feed of recently published photos. Linked from the
// footer; readers can subscribe in Feedly / NetNewsWire / Inoreach.
//
// 50 most-recent published photos. Cached 30 min at the edge so a busy
// reader poll doesn't hammer /api/explore.

interface ExplorePhoto {
  id: string;
  short_id: string;
  target: string | null;
  width?: number | null;
  height?: number | null;
  published_at?: string | null;
  author_handle: string;
  author_display_name: string;
}

const CDN_BASE = (import.meta.env.VITE_CDN_BASE_URL as string | undefined) ?? '';

function escape(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function rfc822(iso: string | null | undefined): string {
  // RSS 2.0 expects RFC 822 dates. JS toUTCString returns the right format.
  const d = iso ? new Date(iso) : new Date();
  return d.toUTCString();
}

export const GET: RequestHandler = async ({ url, fetch }) => {
  const origin = `${url.protocol}//${url.host}`;

  let photos: ExplorePhoto[] = [];
  try {
    const r = await fetch('/api/explore?limit=50');
    if (r.ok) {
      const data = (await r.json()) as { photos?: ExplorePhoto[] };
      photos = data.photos ?? [];
    }
  } catch {
    /* fail soft — emit an empty feed rather than 500 */
  }

  const lastBuildDate = rfc822(photos[0]?.published_at);

  const items = photos
    .filter((p) => p.short_id && p.author_handle)
    .map((p) => {
      const link = `${origin}/u/${encodeURIComponent(p.author_handle)}/p/${p.short_id}`;
      const title = p.target ?? `Untitled by @${p.author_handle}`;
      const pubDate = rfc822(p.published_at);
      const imgUrl = CDN_BASE
        ? `${CDN_BASE}/img/${p.id}?w=1200`
        : `${origin}/api/photos/${p.id}/thumb/1200`;
      return [
        '    <item>',
        `      <title>${escape(title)}</title>`,
        `      <link>${escape(link)}</link>`,
        `      <guid isPermaLink="true">${escape(link)}</guid>`,
        `      <pubDate>${pubDate}</pubDate>`,
        `      <dc:creator>${escape(p.author_display_name || p.author_handle)}</dc:creator>`,
        `      <description>${escape(`${title} — captured by ${p.author_display_name || '@' + p.author_handle} on Astrophoto.`)}</description>`,
        // RSS 2.0 requires all three enclosure attributes; length="0" is the
        // RSS Best Practices Profile's sanctioned value for unknown sizes
        // (we never fetch the image bytes here).
        `      <enclosure url="${escape(imgUrl)}" length="0" type="image/jpeg" />`,
        `      <media:content url="${escape(imgUrl)}" medium="image" />`,
        '    </item>'
      ].join('\n');
    })
    .join('\n');

  const body =
    `<?xml version="1.0" encoding="UTF-8"?>\n` +
    `<rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:media="http://search.yahoo.com/mrss/" xmlns:atom="http://www.w3.org/2005/Atom">\n` +
    `  <channel>\n` +
    `    <title>Astrophoto — recent frames</title>\n` +
    `    <link>${escape(origin)}</link>\n` +
    `    <atom:link href="${escape(origin)}/rss.xml" rel="self" type="application/rss+xml" />\n` +
    `    <description>The most recent published frames from amateur astrophotographers on Astrophoto.</description>\n` +
    `    <language>en</language>\n` +
    `    <lastBuildDate>${lastBuildDate}</lastBuildDate>\n` +
    items +
    `\n  </channel>\n` +
    `</rss>\n`;

  return new Response(body, {
    headers: {
      'content-type': 'application/rss+xml; charset=utf-8',
      'cache-control': 'public, max-age=1800, stale-while-revalidate=3600'
    }
  });
};
