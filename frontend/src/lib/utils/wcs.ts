// Gnomonic (tangent-plane) projection from celestial coordinates to image
// pixel coordinates, given the photo's plate-solve telemetry. Pure TS,
// ~30 lines of standard math, no external dependencies. See
// `docs/superpowers/specs/2026-05-28-celestial-identify-overlay-design.md` §7.

export interface Solve {
  raDeg: number;
  decDeg: number;
  pixelScaleArcsec: number;
  rotationDeg: number;
  width: number;
  height: number;
}

/**
 * Project (RA, Dec) → (x, y) pixel coordinates from image top-left,
 * given the solve. Returns `null` if the target is on the antipodal
 * hemisphere (gnomonic projection has no real solution there).
 * `inFrame` is convenience — caller may still want to render an
 * off-frame marker (e.g. an "object N px outside" arrow), so the
 * out-of-frame case still returns valid x/y.
 */
export function projectRaDecToPixel(
  raDeg: number,
  decDeg: number,
  s: Solve
): { x: number; y: number; inFrame: boolean } | null {
  const a = (raDeg * Math.PI) / 180;
  const d = (decDeg * Math.PI) / 180;
  const a0 = (s.raDeg * Math.PI) / 180;
  const d0 = (s.decDeg * Math.PI) / 180;

  const cos_c = Math.sin(d0) * Math.sin(d) + Math.cos(d0) * Math.cos(d) * Math.cos(a - a0);
  if (cos_c <= 0) return null;

  const xi = (Math.cos(d) * Math.sin(a - a0)) / cos_c;
  const eta = (Math.cos(d0) * Math.sin(d) - Math.sin(d0) * Math.cos(d) * Math.cos(a - a0)) / cos_c;

  // radians → arcsec → pixels; image Y axis points down.
  const RAD_TO_ARCSEC = (180 / Math.PI) * 3600;
  const dxPx = (xi * RAD_TO_ARCSEC) / s.pixelScaleArcsec;
  const dyPx = -(eta * RAD_TO_ARCSEC) / s.pixelScaleArcsec;

  // Plate-solve rotation: angle of sky-up relative to image-up,
  // positive counter-clockwise.
  const r = (s.rotationDeg * Math.PI) / 180;
  const xRot = dxPx * Math.cos(r) - dyPx * Math.sin(r);
  const yRot = dxPx * Math.sin(r) + dyPx * Math.cos(r);

  const x = s.width / 2 + xRot;
  const y = s.height / 2 + yRot;
  const inFrame = x >= 0 && x < s.width && y >= 0 && y < s.height;
  return { x, y, inFrame };
}
