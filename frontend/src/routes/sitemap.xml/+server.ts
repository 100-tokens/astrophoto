import type { RequestHandler } from './$types';

// Dynamic sitemap. Compose from three sources:
//   1. Static surfaces (home, /t, /explore, footer pages)
//   2. Published photos by published_at, public ones only — paginated so the
//      sitemap is the deep-crawl path for `/u/<handle>/p/<short-id>` permalinks
//      that the Load-more grid never links to as static <a> hrefs.
//   3. Top targets (by frame count)
//
// Kept under 5 MB / 50 K URLs per Google's limit. If we cross that, split
// into a sitemap index + multiple per-resource sitemaps.
//
// The backend clamps /api/explore and /api/targets `limit` to 60 (see
// backend/src/discovery/{explore,target_index}.rs), so a single big `limit=`
// is silently truncated — we must walk `next_cursor` to enumerate beyond 60.

// Max cursor pages to walk per source. At 60 rows/page this bounds photo
// enumeration to ~PHOTO_PAGE_CAP*60 URLs and the request to that many
// sequential API round-trips (intentional; this endpoint is CDN-cached 1h).
// TODO: when published photos exceed ~PHOTO_PAGE_CAP*60, the 60-row clamp
// makes one sitemap too slow/large — switch to a sitemap index + a bulk
// (uncapped) enumeration path on the backend rather than raising this cap.
const PHOTO_PAGE_CAP = 84; // ~5,040 photo permalinks, well under the 50k limit
const TARGET_PAGE_CAP = 20; // ~1,200 photographed targets

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
  // /api/explore (DiscoveryPhoto) returns `author_handle`; accept the legacy
  // `owner_handle` too in case any caller shape differs.
  author_handle?: string;
  owner_handle?: string;
  published_at?: string | null;
  created_at?: string;
}
interface ExplorePage {
  photos?: ExplorePhoto[];
  next_cursor?: string | null;
}
interface TargetItem {
  slug: string;
}
interface TargetPage {
  targets?: TargetItem[];
  items?: TargetItem[];
  next_cursor?: string | null;
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

  // Published photos — re-use the public /api/explore endpoint, walking
  // `next_cursor` so permalinks beyond the backend's 60-row page are crawlable.
  // Fail soft: a mid-walk hiccup breaks the loop but keeps what we collected
  // (and the static/category entries already pushed above).
  try {
    let cursor: string | null = null;
    for (let page = 0; page < PHOTO_PAGE_CAP; page++) {
      const qs = cursor ? `&cursor=${encodeURIComponent(cursor)}` : '';
      const r = await fetch(`/api/explore?limit=60${qs}`);
      if (!r.ok) break;
      const data = (await r.json()) as ExplorePage;
      const photos = data.photos ?? [];
      for (const p of photos) {
        const handle = p.author_handle ?? p.owner_handle;
        if (!p.short_id || !handle) continue;
        const lastmod = p.published_at ?? p.created_at;
        urls.push({
          loc: `${origin}/u/${encodeURIComponent(handle)}/p/${p.short_id}`,
          ...(lastmod ? { lastmod } : {}),
          changefreq: 'weekly',
          priority: 0.8
        });
        // Photographer profile page (deduped via Set below)
        urls.push({
          loc: `${origin}/u/${encodeURIComponent(handle)}`,
          changefreq: 'weekly',
          priority: 0.6
        });
      }
      // Stop on exhausted cursor or an empty page (defensive: avoids spinning).
      cursor = data.next_cursor ?? null;
      if (!cursor || photos.length === 0) break;
    }
  } catch {
    /* fail soft */
  }

  // Top targets — only those with published photos. After the OpenNGC seed
  // the catalog is ~12k objects, most photo-less; without this filter the
  // sitemap would hand crawlers thousands of empty stub pages. Same 60-row
  // clamp as explore, so walk `next_cursor` here too.
  try {
    let cursor: string | null = null;
    for (let page = 0; page < TARGET_PAGE_CAP; page++) {
      const qs = cursor ? `&cursor=${encodeURIComponent(cursor)}` : '';
      const r = await fetch(`/api/targets?limit=60&has_photos=true${qs}`);
      if (!r.ok) break;
      const data = (await r.json()) as TargetPage;
      const targets = data.targets ?? data.items ?? [];
      for (const t of targets) {
        if (!t.slug) continue;
        urls.push({
          loc: `${origin}/t/${encodeURIComponent(t.slug)}`,
          changefreq: 'weekly',
          priority: 0.7
        });
      }
      cursor = data.next_cursor ?? null;
      if (!cursor || targets.length === 0) break;
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
