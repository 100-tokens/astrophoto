// Per-field provenance for the verify form's "● FROM EXIF / FROM SETUP"
// chips. The photo's columns are a merge of three sources — file
// metadata (EXIF / XISF / plate-solve), an applied equipment setup, and
// the user's own edits — and the merged row alone can't say which. This
// reconstructs the most honest label we can without a backend provenance
// map:
//
//   - An equipment field whose value equals the applied setup's value for
//     that role → FROM SETUP.
//   - `mount` / `focal_modifier` / `guiding` are never present in a file
//     header, so they are never FROM EXIF — only FROM SETUP or unlabeled.
//   - `camera` / `scope` that don't match the setup are treated as
//     FROM EXIF: that preserves the legitimate case (a setup-less upload
//     whose camera came straight from EXIF). A hand-typed value is the
//     rare miss we accept rather than dropping the common true case.
//   - Acquisition scalars are only ever written by parsing/solve, never by
//     apply-setup (see backend/src/photos/apply_setup.rs), so any present
//     value is FROM EXIF.

/** Equipment fields, in the order they appear on the form. */
const EQUIPMENT = ['camera', 'scope', 'mount', 'focal_modifier', 'guiding'] as const;
/** Equipment fields that can never originate from a file header. */
const NEVER_EXIF = new Set<string>(['mount', 'focal_modifier', 'guiding']);
// Per-capture scalars: only ever written by parsing/plate-solve, never by
// apply-setup — a present value is FROM EXIF.
const ACQUISITION_NUMERIC = [
  'iso',
  'exposure_s',
  'gain',
  'sensor_temp_c',
  'sessions',
  'ra_deg',
  'dec_deg'
] as const;

// FRAMING scalars derived from the optical train. apply-setup computes
// these from the telescope focal × focal-modifier factor (and focal ÷
// aperture). When a setup is applied they read FROM SETUP; with no setup a
// present value came from the file header → FROM EXIF.
const FRAMING_NUMERIC = ['focal_mm', 'aperture_f'] as const;

export type ProvenancePhoto = Partial<
  Record<(typeof EQUIPMENT)[number], string | null> &
    Record<(typeof ACQUISITION_NUMERIC)[number], number | null> &
    Record<(typeof FRAMING_NUMERIC)[number], number | null> & { lens: string | null }
>;

export type SetupValues = Partial<Record<(typeof EQUIPMENT)[number], string | null>>;

function nonEmpty(v: string | null | undefined): string {
  return (v ?? '').trim();
}

export function computeProvenance(
  photo: ProvenancePhoto,
  setup: SetupValues | null,
  opts?: { solved?: boolean }
): { fromExif: Set<string>; fromSetup: Set<string>; fromSolve: Set<string> } {
  const fromExif = new Set<string>();
  const fromSetup = new Set<string>();
  const fromSolve = new Set<string>();
  // A plate-solve measures the true focal/aperture (from pixel scale) and
  // RA/Dec — that's ground truth, so it outranks both a setup's theoretical
  // optical train and the file header. See spec A (focal precedence) + B.
  const solved = opts?.solved ?? false;

  for (const field of EQUIPMENT) {
    const value = nonEmpty(photo[field]);
    if (!value) continue;
    const setupValue = setup ? nonEmpty(setup[field]) : '';
    if (setupValue && value === setupValue) {
      fromSetup.add(field);
    } else if (!NEVER_EXIF.has(field)) {
      fromExif.add(field);
    }
    // else: a mount/focal_modifier/guiding value not from the setup — it
    // was hand-typed, so it gets no provenance chip.
  }

  if (nonEmpty(photo.lens)) fromExif.add('lens');
  for (const field of ACQUISITION_NUMERIC) {
    if (photo[field] == null) continue;
    // RA/Dec are plate-solve outputs: FROM SOLVE when solved, else FROM EXIF.
    if (solved && (field === 'ra_deg' || field === 'dec_deg')) {
      fromSolve.add(field);
    } else {
      fromExif.add(field);
    }
  }
  // FRAMING: a solve measures it (wins); else derived from the setup's optical
  // train when one is applied; else it came from the file header. (Mirrors the
  // equipment-field philosophy: the common true case, accepting a hand-typed
  // override as the rare miss.)
  const hasSetup = setup != null;
  for (const field of FRAMING_NUMERIC) {
    if (photo[field] == null) continue;
    if (solved) fromSolve.add(field);
    else (hasSetup ? fromSetup : fromExif).add(field);
  }

  return { fromExif, fromSetup, fromSolve };
}
