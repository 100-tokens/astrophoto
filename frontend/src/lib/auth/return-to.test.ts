import { describe, expect, it } from 'vitest';
import { safeReturnFromUrl, safeReturnPath } from './return-to';

describe('safeReturnPath', () => {
  it('keeps same-origin paths, queries, and hashes', () => {
    expect(safeReturnPath('/upload')).toBe('/upload');
    expect(safeReturnPath('/settings/email?changed=1')).toBe('/settings/email?changed=1');
    expect(safeReturnPath('/u/mira/p/abc#comments')).toBe('/u/mira/p/abc#comments');
  });

  it('rejects open redirects', () => {
    expect(safeReturnPath('https://evil.example/phish')).toBeNull();
    expect(safeReturnPath('//evil.example')).toBeNull();
    expect(safeReturnPath('/\\evil.example')).toBeNull();
    expect(safeReturnPath('javascript:alert(1)')).toBeNull();
    expect(safeReturnPath('')).toBeNull();
    expect(safeReturnPath(null)).toBeNull();
  });
});

describe('safeReturnFromUrl', () => {
  it('prefers next, then return, else home', () => {
    expect(safeReturnFromUrl(new URL('http://app/signin?next=/upload&return=/x'))).toBe('/upload');
    expect(safeReturnFromUrl(new URL('http://app/signin?return=/u/a'))).toBe('/u/a');
    expect(safeReturnFromUrl(new URL('http://app/signin?next=https://evil.example'))).toBe('/');
    expect(safeReturnFromUrl(new URL('http://app/signin'))).toBe('/');
  });
});
