// Map OpenNGC `object_type` codes (G/Neb/OCl/PN/...) to the CSS variable
// that renders that type's marker color in the overlay. Unknown / null
// types fall back to `--celestial-other`. Variables themselves are
// declared in the app theme — see Phase 3 plan T3.3.

const MAP: Record<string, string> = {
  G: '--celestial-galaxy',
  GPair: '--celestial-galaxy',
  GGroup: '--celestial-galaxy',
  Neb: '--celestial-nebula',
  HII: '--celestial-nebula',
  SNR: '--celestial-nebula',
  EmN: '--celestial-nebula',
  OCl: '--celestial-open-cluster',
  'Cl+N': '--celestial-open-cluster',
  GCl: '--celestial-globular',
  PN: '--celestial-planetary-nebula',
  '*': '--celestial-star',
  '**': '--celestial-star'
};

export function cssVarForType(t: string | null | undefined): string {
  if (!t) return '--celestial-other';
  return MAP[t] ?? '--celestial-other';
}
