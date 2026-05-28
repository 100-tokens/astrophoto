import { describe, it, expect } from 'vitest';
import { cssVarForType } from './celestial-colors';

describe('cssVarForType', () => {
  it('maps galaxies to --celestial-galaxy', () => {
    expect(cssVarForType('G')).toBe('--celestial-galaxy');
    expect(cssVarForType('GPair')).toBe('--celestial-galaxy');
    expect(cssVarForType('GGroup')).toBe('--celestial-galaxy');
  });

  it('maps nebula variants together', () => {
    for (const t of ['Neb', 'HII', 'SNR', 'EmN']) {
      expect(cssVarForType(t)).toBe('--celestial-nebula');
    }
  });

  it('maps OCl and Cl+N to open cluster', () => {
    expect(cssVarForType('OCl')).toBe('--celestial-open-cluster');
    expect(cssVarForType('Cl+N')).toBe('--celestial-open-cluster');
  });

  it('maps GCl to globular', () => {
    expect(cssVarForType('GCl')).toBe('--celestial-globular');
  });

  it('maps PN to planetary nebula', () => {
    expect(cssVarForType('PN')).toBe('--celestial-planetary-nebula');
  });

  it('maps * and ** to star', () => {
    expect(cssVarForType('*')).toBe('--celestial-star');
    expect(cssVarForType('**')).toBe('--celestial-star');
  });

  it('falls back to --celestial-other for unknown / null / undefined', () => {
    expect(cssVarForType(undefined)).toBe('--celestial-other');
    expect(cssVarForType(null)).toBe('--celestial-other');
    expect(cssVarForType('')).toBe('--celestial-other');
    expect(cssVarForType('UnknownCode')).toBe('--celestial-other');
    expect(cssVarForType('Dup')).toBe('--celestial-other');
  });
});
