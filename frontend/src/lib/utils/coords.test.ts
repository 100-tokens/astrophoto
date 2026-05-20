import { describe, it, expect } from 'vitest';
import {
  formatRA,
  formatDec,
  parseRaToDeg,
  parseDecToDeg,
  formatRaHms,
  formatDecDms
} from './coords';

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

// ---------------------------------------------------------------------------
// parseRaToDeg
// ---------------------------------------------------------------------------

describe('parseRaToDeg', () => {
  it('parses a plain decimal degree string', () => {
    expect(parseRaToDeg('337.774')).toBeCloseTo(337.774, 6);
  });
  it('parses negative decimal degrees', () => {
    expect(parseRaToDeg('-12.5')).toBeCloseTo(-12.5, 6);
  });
  it('parses HMS without spaces (20h59m17.2s → 314.8217°)', () => {
    // 20h59m17.2s = (20 + 59/60 + 17.2/3600) hours = 20.9881111... h
    // × 15 = 314.8216666...°
    const got = parseRaToDeg('20h59m17.2s');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(314.8217, 3);
  });
  it('parses HMS with spaces', () => {
    const got = parseRaToDeg('20h 59m 17.2s');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(314.8217, 3);
  });
  it('parses HMS in uppercase', () => {
    const got = parseRaToDeg('20H 59M 17.2S');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(314.8217, 3);
  });
  it('returns null for empty string', () => {
    expect(parseRaToDeg('')).toBeNull();
    expect(parseRaToDeg('   ')).toBeNull();
  });
  it('returns null for garbage', () => {
    expect(parseRaToDeg('not a number')).toBeNull();
    expect(parseRaToDeg('20h59m')).toBeNull();
  });
  it('rejects HMS with minutes ≥ 60', () => {
    expect(parseRaToDeg('20h 60m 00s')).toBeNull();
  });
  it('rejects HMS with seconds ≥ 60', () => {
    expect(parseRaToDeg('20h 59m 60s')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// parseDecToDeg
// ---------------------------------------------------------------------------

describe('parseDecToDeg', () => {
  it('parses a plain decimal degree string', () => {
    expect(parseDecToDeg('+44.529')).toBeCloseTo(44.529, 6);
  });
  it('parses an unsigned decimal as positive', () => {
    expect(parseDecToDeg('44.529')).toBeCloseTo(44.529, 6);
  });
  it('parses a negative decimal', () => {
    expect(parseDecToDeg('-23.456')).toBeCloseTo(-23.456, 6);
  });
  it('parses DMS with unicode primes (+44° 31′ 44″)', () => {
    // 44 + 31/60 + 44/3600 = 44.52888...
    const got = parseDecToDeg('+44° 31′ 44″');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(44.528889, 5);
  });
  it('parses DMS with ASCII primes (-23°27\'44")', () => {
    // -(23 + 27/60 + 44/3600) = -23.46222...
    const got = parseDecToDeg('-23°27\'44"');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(-23.462222, 5);
  });
  it('parses DMS with mixed unicode + ASCII primes', () => {
    const got = parseDecToDeg("+44° 31' 44″");
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(44.528889, 5);
  });
  it('parses DMS without seconds (+44° 31′)', () => {
    const got = parseDecToDeg('+44° 31′');
    expect(got).not.toBeNull();
    expect(got!).toBeCloseTo(44 + 31 / 60, 6);
  });
  it('parses DMS without minutes/seconds (+44°)', () => {
    expect(parseDecToDeg('+44°')).toBeCloseTo(44, 6);
  });
  it('returns null for empty or whitespace', () => {
    expect(parseDecToDeg('')).toBeNull();
    expect(parseDecToDeg('   ')).toBeNull();
  });
  it('returns null for garbage', () => {
    expect(parseDecToDeg('not a dec')).toBeNull();
  });
  it('rejects HMS-style input for DEC', () => {
    expect(parseDecToDeg('20h 59m 17s')).toBeNull();
  });
  it('rejects DMS with minutes ≥ 60', () => {
    expect(parseDecToDeg('+44° 60′ 00″')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// formatRaHms / formatDecDms
// ---------------------------------------------------------------------------

describe('formatRaHms', () => {
  it('formats 314.8217° as 20h 59m 17.2s (round-trip)', () => {
    expect(formatRaHms(314.8217)).toBe('20h 59m 17.2s');
  });
  it('formats 0 as 00h 00m 0.0s', () => {
    expect(formatRaHms(0)).toBe('00h 00m 0.0s');
  });
  it('wraps negatives into [0, 360)', () => {
    expect(formatRaHms(-15)).toBe('23h 00m 0.0s');
  });
});

describe('formatDecDms', () => {
  it('formats +44.528889° as +44° 31′ 44.0″', () => {
    expect(formatDecDms(44.528889)).toBe('+44° 31′ 44.0″');
  });
  it('formats negative declination', () => {
    expect(formatDecDms(-23.462222)).toBe('-23° 27′ 44.0″');
  });
  it('formats 0 as +00° 00′ 0.0″', () => {
    expect(formatDecDms(0)).toBe('+00° 00′ 0.0″');
  });
});
