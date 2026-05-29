import type { RequestHandler } from './$types';

// Dynamic sitemap. Compose from three sources:
//   1. Static surfaces (home, /t, /explore, footer pages)
//   2. Recent published photos (top 200 by published_at, public ones only)
//   3. Top targets (by frame count)
//
// Kept under 5 MB / 50 K URLs per Google's limit. If we cross that, split
// into a sitemap index + multiple per-resource sitemaps.

const STATIC_PATHS = [
  { loc: '/', changefreq: 'hourly', priority: 1.0 },
  { loc: '/explore', changefreq: 'hourly', priority: 0.9 },
  { loc: '/t', changefreq: 'daily', priority: 0.7 },
  { loc: '/photographers', changefreq: 'daily', priority: 0.7 },
  { loc: '/about', changefreq: 'monthly', priority: 0.4 },
  { loc: '/terms', changefreq: 'yearly', priority: 0.3 },
  { loc: '/privacy', changefreq: 'yearly', priority: 0.3 },
  { loc: '/contact', changefreq: 'monthly', priority: 0.3 }
];

// Enabled categories mirror backend/src/discovery/category.rs CATEGORIES,
// minus the catch-all "other" which has no editorial value.
// URLs use hyphens; the backend normalises hyphen↔underscore.
const CATEGORY_SLUGS = ['dso', 'planetary', 'lunar', 'solar', 'wide-field', 'nightscape'];

function escape(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

interface ExplorePhoto {
  short_id: string;
  owner_handle: string;
  published_at?: string | null;
  created_at?: string;
}
interface TargetItem {
  slug: string;
}

export const GET: RequestHandler = async ({ url, fetch }) => {
  const origin = `${url.protocol}//${url.host}`;
  const urls: { loc: string; lastmod?: string; changefreq?: string; priority?: number }[] = [];

  for (const s of STATIC_PATHS) {
    urls.push({ loc: `${origin}${s.loc}`, changefreq: s.changefreq, priority: s.priority });
  }

  for (const slug of CATEGORY_SLUGS) {
    urls.push({
      loc: `${origin}/c/${slug}`,
      changefreq: 'daily',
      priority: 0.6
    });
  }

  // Recent photos — re-use the public /api/explore endpoint. Fail soft so
  // a transient backend hiccup doesn't break the whole sitemap.
  try {
    const r = await fetch('/api/explore?limit=200');
    if (r.ok) {
      const data = (await r.json()) as { photos?: ExplorePhoto[] };
      for (const p of data.photos ?? []) {
        if (!p.short_id || !p.owner_handle) continue;
        const lastmod = p.published_at ?? p.created_at;
        urls.push({
          loc: `${origin}/u/${encodeURIComponent(p.owner_handle)}/p/${p.short_id}`,
          ...(lastmod ? { lastmod } : {}),
          changefreq: 'weekly',
          priority: 0.8
        });
        // Photographer profile page (deduped via Set below)
        urls.push({
          loc: `${origin}/u/${encodeURIComponent(p.owner_handle)}`,
          changefreq: 'weekly',
          priority: 0.6
        });
      }
    }
  } catch {
    /* fail soft */
  }

  // Top targets — only those with published photos. After the OpenNGC seed
  // the catalog is ~12k objects, most photo-less; without this filter the
  // sitemap would hand crawlers thousands of empty stub pages.
  try {
    const r = await fetch('/api/targets?limit=200&has_photos=true');
    if (r.ok) {
      const data = (await r.json()) as { targets?: TargetItem[]; items?: TargetItem[] };
      for (const t of data.targets ?? data.items ?? []) {
        if (!t.slug) continue;
        urls.push({
          loc: `${origin}/t/${encodeURIComponent(t.slug)}`,
          changefreq: 'weekly',
          priority: 0.7
        });
      }
    }
  } catch {
    /* fail soft */
  }

  // Dedupe by URL
  const seen = new Set<string>();
  const deduped = urls.filter((u) => {
    if (seen.has(u.loc)) return false;
    seen.add(u.loc);
    return true;
  });

  const body =
    `<?xml version="1.0" encoding="UTF-8"?>\n` +
    `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n` +
    deduped
      .map((u) => {
        const parts = [`  <url>`, `    <loc>${escape(u.loc)}</loc>`];
        if (u.lastmod) parts.push(`    <lastmod>${escape(u.lastmod)}</lastmod>`);
        if (u.changefreq) parts.push(`    <changefreq>${u.changefreq}</changefreq>`);
        if (u.priority != null) parts.push(`    <priority>${u.priority}</priority>`);
        parts.push(`  </url>`);
        return parts.join('\n');
      })
      .join('\n') +
    `\n</urlset>\n`;

  return new Response(body, {
    headers: {
      'content-type': 'application/xml; charset=utf-8',
      // Cache 1h at the CDN — sitemap content moves slowly enough.
      'cache-control': 'public, max-age=3600, stale-while-revalidate=86400'
    }
  });
};
