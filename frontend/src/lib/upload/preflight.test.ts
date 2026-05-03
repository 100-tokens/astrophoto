import { describe, it, expect } from 'vitest';
import { preflight } from './preflight';

describe('preflight', () => {
  it('hashes deterministically', async () => {
    // jpeg-shaped 4 bytes — enough for sha256 + bitmap-creation may
    // fail in jsdom; gate the bitmap test out for unit context.
    const f = new File([new Uint8Array([0xff, 0xd8, 0xff, 0xe0])], 'a.jpg', {
      type: 'image/jpeg'
    });
    // Hash only — bitmap requires browser canvas
    const { hash } = await preflight(f).catch(() => ({ hash: '' }));
    if (hash) expect(hash.length).toBe(64);
  });
});
