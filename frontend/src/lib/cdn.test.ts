import { describe, it, expect } from 'vitest';
import { cdn, srcset } from './cdn';

describe('cdn URL builder', () => {
  it('returns base path + id with no params', () => {
    expect(cdn('abc')).toBe('/cdn/img/abc');
  });
  it('adds width', () => {
    expect(cdn('abc', { w: 800 })).toBe('/cdn/img/abc?w=800');
  });
  it('builds srcset', () => {
    const s = srcset('abc', [400, 800, 1200]);
    expect(s).toContain('400w');
    expect(s).toContain('1200w');
  });
});
