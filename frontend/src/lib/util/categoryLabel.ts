// Friendly labels for the seven photo categories. Mirrors the enum in
// backend/src/discovery/category.rs. The DB stores underscored keys; URLs
// accept either form (the backend normalises hyphen→underscore).
export const CATEGORY_LABELS: Record<string, string> = {
  dso: 'Deep-Sky Objects',
  planetary: 'Planetary',
  lunar: 'Lunar',
  solar: 'Solar',
  wide_field: 'Wide-field',
  nightscape: 'Nightscape',
  other: 'Other'
};

export function categoryLabel(slug: string): string {
  const key = slug.replace(/-/g, '_').toLowerCase();
  return CATEGORY_LABELS[key] ?? slug;
}
