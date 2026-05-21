// Known equipment brands. Used as autocomplete hints in BrandModelInput.
//
// Mirrors the whitelist used by migration 0022's brand backfill — keep it
// in sync if you grow the migration list. The user is free to enter any
// brand name (the input accepts free text); this list just powers the
// `<datalist>` suggestions so the common case is one keystroke.

export const KNOWN_BRANDS: readonly string[] = [
  // Telescopes
  'Sky-Watcher',
  'Celestron',
  'Takahashi',
  'Vixen',
  'Astro-Tech',
  'William Optics',
  'Tele Vue',
  'Meade',
  'Orion',
  'Astro-Physics',
  // Mounts
  'iOptron',
  'Losmandy',
  'Paramount',
  // Cameras (astro + DSLR/mirrorless)
  'ZWO',
  'QHY',
  'Player One',
  'Touptek',
  'Canon',
  'Nikon',
  'Sony',
  'Fujifilm',
  'Pentax',
  // Filters
  'Astronomik',
  'Optolong',
  'Antlia',
  'Baader',
  'Chroma',
  'Astrodon',
  'IDAS'
] as const;
