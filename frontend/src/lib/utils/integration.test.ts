import { describe, it, expect } from 'vitest';
import { rowTotalS, grandTotalS, totalSubs, formatHm, matchAliasToCatalog } from './integration';
import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';

describe('integration helpers', () => {
  it('row total = subs × exposure', () => {
    expect(
      rowTotalS({
        filter: 'L',
        sub_count: 120,
        sub_exposure_s: 120,
        filter_item_id: null,
        gain: null,
        sensor_temp_c: null
      })
    ).toBe(14400);
  });

  it('grand total + sub count sum across rows', () => {
    const rows = [
      {
        filter: 'L',
        sub_count: 120,
        sub_exposure_s: 120,
        filter_item_id: null,
        gain: null,
        sensor_temp_c: null
      },
      {
        filter: 'R',
        sub_count: 40,
        sub_exposure_s: 120,
        filter_item_id: null,
        gain: null,
        sensor_temp_c: null
      }
    ];
    expect(grandTotalS(rows)).toBe(14400 + 4800);
    expect(totalSubs(rows)).toBe(160);
  });

  it('treats blank/NaN inputs as zero', () => {
    expect(
      rowTotalS({
        filter: '',
        sub_count: 0,
        sub_exposure_s: 0,
        filter_item_id: null,
        gain: null,
        sensor_temp_c: null
      })
    ).toBe(0);
  });

  it('formats seconds as Hh MMm', () => {
    expect(formatHm(14400)).toBe('4 h 00 m');
    expect(formatHm(4500)).toBe('1 h 15 m');
    expect(formatHm(0)).toBe('0 h 00 m');
  });
});

describe('matchAliasToCatalog', () => {
  const chip = (id: string, display_name: string, filter_type: string | null): PhotoFilterChip => ({
    id,
    display_name,
    filter_type: filter_type as PhotoFilterChip['filter_type'],
    bandwidth_nm: null,
    position: 0
  });

  const rgb = [
    chip('id-r', 'Baader RGB R CMOS-optimized', 'red'),
    chip('id-g', 'Baader RGB G CMOS-optimized', 'green'),
    chip('id-b', 'Baader RGB B CMOS-optimized', 'blue')
  ];

  it('matches a header alias to the catalog chip by filter_type', () => {
    expect(matchAliasToCatalog('R', rgb)).toBe('id-r');
    expect(matchAliasToCatalog('G', rgb)).toBe('id-g');
    expect(matchAliasToCatalog('B', rgb)).toBe('id-b');
  });

  it('is case- and whitespace-insensitive and resolves synonyms', () => {
    expect(matchAliasToCatalog(' red ', rgb)).toBe('id-r');
    expect(matchAliasToCatalog('Blue', rgb)).toBe('id-b');
  });

  it('returns null when no catalog filter matches the band (e.g. L with no L chip)', () => {
    expect(matchAliasToCatalog('L', rgb)).toBeNull();
    expect(matchAliasToCatalog('Ha', rgb)).toBeNull();
  });

  it('matches narrowband via filter_type', () => {
    const nb = [
      chip('id-ha', 'Astronomik Ha 6nm', 'h_alpha'),
      chip('id-o3', 'Astronomik OIII 6nm', 'oiii')
    ];
    expect(matchAliasToCatalog('Ha', nb)).toBe('id-ha');
    expect(matchAliasToCatalog('OIII', nb)).toBe('id-o3');
  });

  it('falls back to the display name when filter_type is absent', () => {
    const untyped = [chip('id-l', 'ZWO Luminance', null)];
    expect(matchAliasToCatalog('L', untyped)).toBe('id-l');
  });

  it('returns null on an ambiguous (>1) match', () => {
    const dup = [chip('id-r1', 'Baader R', 'red'), chip('id-r2', 'Chroma R', 'red')];
    expect(matchAliasToCatalog('R', dup)).toBeNull();
  });

  it('returns null for an unknown alias', () => {
    expect(matchAliasToCatalog('XYZ', rgb)).toBeNull();
    expect(matchAliasToCatalog('', rgb)).toBeNull();
  });
});
