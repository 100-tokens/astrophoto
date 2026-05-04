import { describe, it, expect } from 'vitest';
import { env } from '$env/dynamic/public';
import { cdn, srcset } from './cdn';

// The base is resolved at module load from PUBLIC_CDN_BASE_URL with a
// fallback of '/cdn'. The CI run picks this up from `frontend/.env.local`
// or process env, so the assertions key off the actual resolved base
// rather than the hard-coded fallback.
const BASE = env.PUBLIC_CDN_BASE_URL ?? '/cdn';

describe('cdn URL builder', () => {
  it('returns base path + id with no params', () => {
    expect(cdn('abc')).toBe(`${BASE}/img/abc`);
  });
  it('adds width', () => {
    expect(cdn('abc', { w: 800 })).toBe(`${BASE}/img/abc?w=800`);
  });
  it('builds srcset', () => {
    const s = srcset('abc', [400, 800, 1200]);
    expect(s).toContain('400w');
    expect(s).toContain('1200w');
    expect(s).toContain(`${BASE}/img/abc`);
  });
});
