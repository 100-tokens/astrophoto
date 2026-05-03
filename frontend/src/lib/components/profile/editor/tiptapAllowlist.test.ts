import { describe, it, expect } from 'vitest';
import { ALLOWED_HTML_TAGS } from './tiptapAllowlist';
import shared from '../../../../../../backend/data/bio-allowed-tags.json';

describe('tiptapAllowlist', () => {
  it('matches the shared backend JSON', () => {
    expect([...ALLOWED_HTML_TAGS].sort()).toEqual([...shared.tags].sort());
  });
});
