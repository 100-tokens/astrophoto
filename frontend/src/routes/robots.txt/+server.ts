import type { RequestHandler } from './$types';

// robots.txt is served from a route (not frontend/static/) so the Sitemap
// line tracks the request origin. The previous static file hardcoded the
// staging host and shipped identically to every environment, so prod
// advertised the staging sitemap. Static assets shadow routes under
// adapter-node, so the static file must stay deleted for this to be hit.
//
// Crawl directives are preserved verbatim from the former static file.
const DIRECTIVES = `User-agent: *
Allow: /
Disallow: /signin
Disallow: /signup
Disallow: /upload
Disallow: /settings
Disallow: /account
Disallow: /reset
Disallow: /me
Disallow: /api/
Disallow: /design
Disallow: /drafts
Disallow: /search

# Generative-engine guidance — these are the LLM crawlers as of 2026.
# They get the same allow-list as everyone else, plus the llms.txt summary.
User-agent: GPTBot
User-agent: ChatGPT-User
User-agent: PerplexityBot
User-agent: ClaudeBot
User-agent: Google-Extended
Allow: /
Disallow: /signin
Disallow: /signup
Disallow: /upload
Disallow: /settings
Disallow: /account
Disallow: /reset
Disallow: /me
Disallow: /api/
Disallow: /drafts
Disallow: /search
`;

export const GET: RequestHandler = ({ url }) => {
  const origin = `${url.protocol}//${url.host}`;
  const body = `${DIRECTIVES}\nSitemap: ${origin}/sitemap.xml\n`;
  return new Response(body, {
    headers: {
      'content-type': 'text/plain; charset=utf-8',
      'cache-control': 'public, max-age=3600'
    }
  });
};
