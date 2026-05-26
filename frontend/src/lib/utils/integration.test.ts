import { describe, it, expect } from 'vitest';
import { rowTotalS, grandTotalS, totalSubs, formatHm } from './integration';

describe('integration helpers', () => {
  it('row total = subs × exposure', () => {
    expect(rowTotalS({ filter: 'L', sub_count: 120, sub_exposure_s: 120 })).toBe(14400);
  });

  it('grand total + sub count sum across rows', () => {
    const rows = [
      { filter: 'L', sub_count: 120, sub_exposure_s: 120 },
      { filter: 'R', sub_count: 40, sub_exposure_s: 120 }
    ];
    expect(grandTotalS(rows)).toBe(14400 + 4800);
    expect(totalSubs(rows)).toBe(160);
  });

  it('treats blank/NaN inputs as zero', () => {
    expect(rowTotalS({ filter: '', sub_count: 0, sub_exposure_s: 0 })).toBe(0);
  });

  it('formats seconds as Hh MMm', () => {
    expect(formatHm(14400)).toBe('4 h 00 m');
    expect(formatHm(4500)).toBe('1 h 15 m');
    expect(formatHm(0)).toBe('0 h 00 m');
  });
});
