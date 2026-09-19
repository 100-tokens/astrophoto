import { describe, it, expect } from 'vitest';
import { humanizeUploadError } from './errors';

describe('humanizeUploadError', () => {
  it('never surfaces raw JSON for known error envelopes', () => {
    const conflict = humanizeUploadError(
      '{"error":"conflict","message":"conflict: file already uploaded"}'
    );
    expect(conflict).toContain('already uploaded');
    expect(conflict).not.toContain('{');

    const xisf = humanizeUploadError(
      '{"error":"unsupported-format","message":"unsupported format: application/x-xisf (plate-solve service not configured)"}'
    );
    expect(xisf).toContain('XISF uploads are temporarily unavailable');
    expect(xisf).not.toContain('{');
  });

  it('falls back to the envelope message for unmapped codes', () => {
    expect(humanizeUploadError('{"error":"bad-request","message":"photo not ready"}')).toBe(
      'photo not ready'
    );
  });

  it('never surfaces raw forbidden / origin-mismatch strings', () => {
    const envelope = humanizeUploadError('{"error":"forbidden","message":"forbidden"}');
    expect(envelope.toLowerCase()).not.toBe('forbidden');
    expect(envelope).toMatch(/page address is not allowed/i);
    expect(envelope).toMatch(/retry/i);

    const plain = humanizeUploadError('forbidden');
    expect(plain.toLowerCase()).not.toBe('forbidden');
    expect(plain).toMatch(/page address is not allowed/i);

    const proxy = humanizeUploadError('cross-origin request blocked');
    expect(proxy).not.toContain('cross-origin request blocked');
    expect(proxy).toMatch(/page address is not allowed/i);

    // Vitest runs with import.meta.env.DEV, so the loopback hint is present.
    // Production builds omit it — see forbiddenCopy().
    expect(envelope).toMatch(/localhost|127\.0\.0\.1/);
    expect(plain).toMatch(/localhost|127\.0\.0\.1/);
    expect(proxy).toMatch(/localhost|127\.0\.0\.1/);
  });

  it('passes through plain text and truncates long bodies', () => {
    expect(humanizeUploadError('PUT 403')).toBe('PUT 403');
    expect(humanizeUploadError('')).toBe('Upload failed.');
    expect(humanizeUploadError('x'.repeat(500))).toHaveLength(201);
  });
});
