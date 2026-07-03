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

  it('passes through plain text and truncates long bodies', () => {
    expect(humanizeUploadError('PUT 403')).toBe('PUT 403');
    expect(humanizeUploadError('')).toBe('Upload failed.');
    expect(humanizeUploadError('x'.repeat(500))).toHaveLength(201);
  });
});
