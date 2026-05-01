/**
 * Placeholder data module — Photo 2 of the design integration.
 *
 * All data is static / synthetic; no network calls.
 * Slugs are derived from target strings: lower-case, non-alphanumeric
 * runs replaced with "-", leading/trailing "-" trimmed.
 * Greek characters (e.g. "ρ") are non-ASCII so they collapse to the
 * separator and get trimmed — `ρ Ophiuchi Cloud` → `ophiuchi-cloud`.
 * That is intentional and deterministic (no unicode flag on the regex).
 */

export interface Photo {
  slug: string;
  target: string;
  ratio: number; // width / height
  integration: string;
  photographer: string;
  photographerSlug: string;
  camera: string;
}

export interface Photographer {
  name: string;
  initial: string;
  frames: number;
  bortle: number;
  location: string;
  caption: string;
}

export interface PhotoDetail {
  slug: string;
  target: string;
  targetSubtitle: string;
  captured: string;
  camera: string;
  cameraSub: string;
  telescope: string;
  telescopeSub: string;
  mount: string;
  filters: string;
  exposure: string;
  exposureTotal: string;
  gain: string;
  ra: string;
  dec: string;
  field: string;
  pixelScale: string;
  publishedDate: string;
  photographer: Photographer;
  appreciations: number;
  comments: number;
  // Allow usage as a gallery Photo too
  ratio: number;
  integration: string;
}

export interface User {
  username: string;
  displayName: string;
  firstName: string;
  surnameItalic: string;
  initial: string;
  about: string;
  frames: number;
  integrationTotal: string;
  followers: number;
  collections: number;
  lat: string;
  long: string;
  bortle: number;
  sqm: number;
  equipment: {
    scope: string;
    camera: string;
    mount: string;
    filters: string;
  };
  memberSince: string;
}

/** Slugify a target string into a URL-safe identifier. */
export function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

/** Slugify a display name into a username handle. */
export function slugifyUsername(name: string): string {
  return slugify(name);
}

// ---------------------------------------------------------------------------
// The 15 gallery photos — ported verbatim from shared.jsx
// (src / Wikipedia hotlinks dropped; gradient is generated from target)
// Note: index 7 is NGC 7000 — its slug is forced to match the rich detail URL.
// ---------------------------------------------------------------------------

const RAW: Array<Omit<Photo, 'slug' | 'photographerSlug'>> = [
  {
    target: 'M16 · Pillars of Creation',
    ratio: 1.16,
    integration: '—',
    photographer: 'Hubble (NASA/ESA)',
    camera: 'Hubble WFC3'
  },
  {
    target: 'M31 · Andromeda Galaxy',
    ratio: 1.5,
    integration: '9h 40m',
    photographer: 'Marie Dubois',
    camera: 'ZWO ASI2600MC'
  },
  {
    target: 'IC 1805 · Heart Nebula',
    ratio: 1.4,
    integration: '14h 06m',
    photographer: 'StarHunter42',
    camera: 'QHY268M'
  },
  {
    target: 'NGC 6960 · Western Veil',
    ratio: 1.5,
    integration: '11h 30m',
    photographer: 'K. Aalto',
    camera: 'ASI2600MM'
  },
  {
    target: 'NGC 3324 · Cosmic Cliffs',
    ratio: 1.7,
    integration: '—',
    photographer: 'JWST',
    camera: 'NIRCam'
  },
  {
    target: 'M42 · Orion Nebula',
    ratio: 1.33,
    integration: '6h 12m',
    photographer: 'L. Petrov',
    camera: 'Canon R6'
  },
  {
    target: 'IC 434 · Horsehead Nebula',
    ratio: 1.0,
    integration: '8h 48m',
    photographer: 'CometChaser_2024',
    camera: 'ASI294MC'
  },
  // index 7 — NGC 7000 (the rich photo-detail subject)
  // Slug is forced to 'ngc-7000-north-america-nebula' so it matches the
  // route param in /photo/ngc-7000-north-america-nebula.
  {
    target: 'NGC 7000 · North America',
    ratio: 1.4,
    integration: '18h 00m',
    photographer: 'Marie Dubois',
    camera: 'ASI2600MC Pro'
  },
  {
    target: 'M33 · Triangulum Galaxy',
    ratio: 1.5,
    integration: '12h 24m',
    photographer: 'P. Halverson',
    camera: 'QHY600M'
  },
  {
    target: 'M51 · Whirlpool Galaxy',
    ratio: 1.0,
    integration: '7h 18m',
    photographer: 'S. Tanaka',
    camera: 'ASI6200MM'
  },
  {
    target: 'ρ Ophiuchi Cloud',
    ratio: 1.5,
    integration: '5h 45m',
    photographer: 'A. Dimov',
    camera: 'Sony A7R V'
  },
  {
    target: 'NGC 281 · Pacman Nebula',
    ratio: 1.3,
    integration: '10h 12m',
    photographer: 'R. Mehta',
    camera: 'ASI533MC Pro'
  },
  {
    target: 'M27 · Dumbbell Nebula',
    ratio: 1.5,
    integration: '4h 30m',
    photographer: 'L. Petrov',
    camera: 'QHY268M'
  },
  {
    target: 'Moon · Mare Imbrium',
    ratio: 1.0,
    integration: '— (1 frame)',
    photographer: 'L. Viatour',
    camera: 'C11 Edge'
  },
  {
    target: 'NGC 2070 · Tarantula',
    ratio: 1.5,
    integration: '9h 00m',
    photographer: 'Southern Sky Co.',
    camera: 'ASI2600MM'
  }
];

export const PHOTOS: Photo[] = RAW.map((p, i) => {
  // Index 7 (NGC 7000) gets a forced slug to coordinate with the rich detail page.
  const slug = i === 7 ? 'ngc-7000-north-america-nebula' : slugify(p.target);
  return {
    ...p,
    slug,
    photographerSlug: slugifyUsername(p.photographer)
  };
});

// ---------------------------------------------------------------------------
// Rich canonical photo detail — NGC 7000
// ---------------------------------------------------------------------------

export const NGC7000: PhotoDetail = {
  slug: 'ngc-7000-north-america-nebula',
  target: 'NGC 7000',
  targetSubtitle: 'North America Nebula',
  captured: '14–17 Mar 2026 · 4 sessions',
  camera: 'ZWO ASI2600MC Pro',
  cameraSub: 'Cooled CMOS, −10 °C',
  telescope: 'Takahashi FSQ-106EDX4',
  telescopeSub: 'f/5, 530 mm',
  mount: '10Micron GM1000 HPS',
  filters: 'Antlia 3 nm SHO',
  exposure: '180 × 360 s',
  exposureTotal: '= 18.0 hours',
  gain: '100',
  ra: '20ʰ 58ᵐ 47ˢ',
  dec: '+44° 19′ 53″',
  field: '1.7° × 1.1°',
  pixelScale: '1.92 ″/px',
  publishedDate: '17 MAR 2026',
  photographer: {
    name: 'Marie Dubois',
    initial: 'M',
    frames: 42,
    bortle: 4,
    location: 'Provence',
    caption:
      'North America Nebula in narrowband, 18 h total integration over 4 nights from a Bortle 4 site in Provence. ' +
      'Hubble palette (SHO), processed in PixInsight with a careful background-extraction pass and a ' +
      'non-linear stretch designed to preserve the dim H-α tendrils running through Pelican.'
  },
  appreciations: 248,
  comments: 12,
  ratio: 1.4,
  integration: '18h 00m'
};

// ---------------------------------------------------------------------------
// Rich canonical user — Marie Dubois
// ---------------------------------------------------------------------------

export const MARIE: User = {
  username: 'marie-dubois',
  displayName: 'Marie Dubois',
  firstName: 'Marie',
  surnameItalic: 'Dubois',
  initial: 'M',
  about:
    'Deep-sky narrowband imaging from a Bortle 4 site in Haute-Provence. ' +
    'Mostly emission nebulae and galaxy clusters. Always happy to share processing notes.',
  frames: 42,
  integrationTotal: '318 h',
  followers: 1204,
  collections: 8,
  lat: '44.1°N',
  long: '6.2°E',
  bortle: 4,
  sqm: 21.8,
  equipment: {
    scope: 'Tak FSQ-106EDX4',
    camera: 'ZWO ASI2600MC Pro',
    mount: '10Micron GM1000 HPS',
    filters: 'Antlia 3nm SHO'
  },
  memberSince: '2026'
};
