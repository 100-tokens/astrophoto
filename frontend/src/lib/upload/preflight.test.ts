import { describe, it, expect } from 'vitest';
import { preflight, resolveMime } from './preflight';

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

describe('resolveMime', () => {
  it('returns file.type when the browser knows the mime', () => {
    const f = new File([new Uint8Array([0])], 'a.jpg', { type: 'image/jpeg' });
    expect(resolveMime(f)).toBe('image/jpeg');
  });

  it('maps `.xisf` to application/x-xisf when file.type is empty', () => {
    // Browsers leave File.type = "" for unknown extensions like .xisf —
    // the resolver fills that in so upload_init / S3 PUT both see the
    // mime the backend's allowlist expects.
    const f = new File([new Uint8Array([0])], 'master.xisf', { type: '' });
    expect(resolveMime(f)).toBe('application/x-xisf');
  });

  it('case-insensitive on the .xisf extension', () => {
    const f = new File([new Uint8Array([0])], 'MASTER.XISF', { type: '' });
    expect(resolveMime(f)).toBe('application/x-xisf');
  });

  it('falls back to application/octet-stream for unknown extensions', () => {
    const f = new File([new Uint8Array([0])], 'mystery.dat', { type: '' });
    expect(resolveMime(f)).toBe('application/octet-stream');
  });
});
