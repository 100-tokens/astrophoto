import { describe, it, expect } from 'vitest';
import { formatIntegration } from './integration';

describe('formatIntegration', () => {
  it('returns em-dash when zero', () => {
    expect(formatIntegration(0)).toBe('—');
  });
  it('returns em-dash on negative', () => {
    expect(formatIntegration(-5)).toBe('—');
  });
  it('returns em-dash on NaN', () => {
    expect(formatIntegration(Number.NaN)).toBe('—');
  });
  it('formats hours and minutes', () => {
    expect(formatIntegration(3 * 3600 + 14 * 60)).toBe('3h 14m');
  });
  it('drops minutes when whole hours', () => {
    expect(formatIntegration(5 * 3600)).toBe('5h');
  });
  it('shows minutes only under one hour', () => {
    expect(formatIntegration(45 * 60)).toBe('45m');
  });
});
