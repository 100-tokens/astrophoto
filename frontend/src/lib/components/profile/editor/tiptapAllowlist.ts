/**
 * Allowed HTML tags for the bio editor. MUST match
 * `backend/data/bio-allowed-tags.json` exactly — drift is verified by
 * `tiptapAllowlist.test.ts`.
 */
export const ALLOWED_HTML_TAGS = [
  'a',
  'blockquote',
  'br',
  'code',
  'em',
  'h2',
  'h3',
  'h4',
  'li',
  'ol',
  'p',
  'strong',
  'u',
  'ul'
] as const;

export type AllowedTag = (typeof ALLOWED_HTML_TAGS)[number];
