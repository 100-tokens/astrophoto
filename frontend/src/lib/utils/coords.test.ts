import { describe, it, expect } from 'vitest';
import { formatRA, formatDec } from './coords';

describe('formatRA', () => {
  it('formats M31 RA (10.6847° → 00ʰ42ᵐ44ˢ)', () => {
    expect(formatRA(10.6847)).toBe('00ʰ42ᵐ44ˢ');
  });
  it('formats value at 0', () => {
    expect(formatRA(0.0)).toBe('00ʰ00ᵐ00ˢ');
  });
  it('wraps 360 back to 0', () => {
    expect(formatRA(360.0)).toBe('00ʰ00ᵐ00ˢ');
  });
});

describe('formatDec', () => {
  it('formats M31 Dec (+41.2693° → +41°16′09″)', () => {
    expect(formatDec(41.2693)).toBe('+41°16′09″');
  });
  it('formats negative value (-29.866° → -29°51′58″)', () => {
    expect(formatDec(-29.866)).toBe('-29°51′58″');
  });
  it('formats small positive value (0.001° → +00°00′04″)', () => {
    expect(formatDec(0.001)).toBe('+00°00′04″');
  });
});
