import { describe, it, expect } from 'vitest';
import { projectRaDecToPixel, type Solve } from './wcs';

const baseSolve: Solve = {
  raDeg: 180,
  decDeg: 0,
  pixelScaleArcsec: 1,
  rotationDeg: 0,
  width: 1000,
  height: 1000
};

describe('projectRaDecToPixel', () => {
  it('returns image center for the solve center', () => {
    const p = projectRaDecToPixel(180, 0, baseSolve)!;
    expect(p.x).toBeCloseTo(500, 5);
    expect(p.y).toBeCloseTo(500, 5);
    expect(p.inFrame).toBe(true);
  });

  it('returns null for the antipodal hemisphere', () => {
    expect(projectRaDecToPixel(0, 0, baseSolve)).toBeNull();
  });

  it('places +RA offset along the x-axis at rotation 0', () => {
    const p = projectRaDecToPixel(180.05, 0, baseSolve)!;
    // 0.05° = 180", divided by 1"/px = 180 px in xi direction
    expect(p.x).toBeGreaterThan(500);
    expect(Math.abs(p.y - 500)).toBeLessThan(1);
  });

  it('handles RA wrap (center near 0, point just past 359)', () => {
    // Center at 0.05°, point at 359.95° → 0.1° apart (the projection's
    // cos(a - a0) handles the wrap naturally; |359.9°| has same cosine as
    // |0.1°|). At 1"/px = 360 px in a 1000×1000 frame → in frame.
    const wrap = { ...baseSolve, raDeg: 0.05 };
    const p = projectRaDecToPixel(359.95, 0, wrap)!;
    expect(p.inFrame).toBe(true);
    // Should project to ~140 px on the left side (500 - 360).
    expect(p.x).toBeLessThan(500);
    expect(p.x).toBeGreaterThan(0);
  });

  it('rotation 90° puts +RA on the y-axis', () => {
    const rotated = { ...baseSolve, rotationDeg: 90 };
    const p = projectRaDecToPixel(180.05, 0, rotated)!;
    expect(Math.abs(p.x - 500)).toBeLessThan(1);
    expect(p.y).not.toBeCloseTo(500, 1);
  });

  it('marks out-of-frame points as inFrame=false but still returns coords', () => {
    // Far enough in declination to fall outside a 1000×1000 / 1"/px frame.
    const p = projectRaDecToPixel(180, 0.3, baseSolve)!;
    expect(p.inFrame).toBe(false);
    expect(Number.isFinite(p.x)).toBe(true);
    expect(Number.isFinite(p.y)).toBe(true);
  });
});
