import { describe, it, expect } from 'vitest';
import { computeProvenance } from './provenance';

describe('computeProvenance', () => {
  it('labels equipment matching the applied setup as FROM SETUP, not FROM EXIF', () => {
    // The M20 case: camera/scope/mount all came from "Mon setup principal".
    const photo = {
      camera: 'ZWO ASI533MM Pro',
      scope: 'Celestron EdgeHD 8',
      mount: 'ZWO AM5',
      focal_modifier: null,
      guiding: null,
      focal_mm: 1478.326,
      gain: 0
    };
    const setup = {
      camera: 'ZWO ASI533MM Pro',
      scope: 'Celestron EdgeHD 8',
      mount: 'ZWO AM5',
      focal_modifier: null,
      guiding: null
    };
    const { fromExif, fromSetup } = computeProvenance(photo, setup);

    // FRAMING (focal_mm) is derived from the optical train, so with a setup
    // applied it reads FROM SETUP alongside the equipment fields.
    expect(fromSetup).toEqual(new Set(['camera', 'scope', 'mount', 'focal_mm']));
    // The mount must NOT be tagged FROM EXIF — that was the lie we're fixing.
    expect(fromExif.has('mount')).toBe(false);
    expect(fromExif.has('camera')).toBe(false);
    expect(fromExif.has('scope')).toBe(false);
    expect(fromExif.has('focal_mm')).toBe(false);
    // Per-capture scalars are never from a setup; present → FROM EXIF.
    expect(fromExif.has('gain')).toBe(true);
    expect(fromSetup.has('gain')).toBe(false);
  });

  it('labels FRAMING (focal_mm/aperture_f) FROM EXIF when no setup is applied', () => {
    const photo = { focal_mm: 530, aperture_f: 5 };
    const { fromExif, fromSetup } = computeProvenance(photo, null);
    expect(fromExif.has('focal_mm')).toBe(true);
    expect(fromExif.has('aperture_f')).toBe(true);
    expect(fromSetup.size).toBe(0);
  });

  it('never tags mount/focal_modifier/guiding FROM EXIF when no setup is applied', () => {
    const photo = { camera: 'Nikon D850', mount: 'Sky-Watcher EQ6-R', guiding: 'OAG' };
    const { fromExif, fromSetup } = computeProvenance(photo, null);

    // Camera can legitimately come from EXIF.
    expect(fromExif.has('camera')).toBe(true);
    // Mount/guiding can't come from a file and there is no setup → no chip.
    expect(fromExif.has('mount')).toBe(false);
    expect(fromExif.has('guiding')).toBe(false);
    expect(fromSetup.size).toBe(0);
  });

  it('keeps camera/scope as FROM EXIF when present but not matching the setup', () => {
    const photo = { camera: 'ZWO ASI2600MC', scope: 'William Optics RedCat 51' };
    const setup = { camera: 'Canon EOS R', scope: 'Celestron EdgeHD 8' };
    const { fromExif, fromSetup } = computeProvenance(photo, setup);

    expect(fromExif.has('camera')).toBe(true);
    expect(fromExif.has('scope')).toBe(true);
    expect(fromSetup.size).toBe(0);
  });

  it('treats numeric 0 / present acquisition fields as recovered, blanks as absent', () => {
    const photo = {
      iso: null,
      gain: 0,
      sensor_temp_c: -10,
      sessions: null,
      ra_deg: 271.0,
      dec_deg: 0,
      lens: '',
      exposure_s: null
    };
    const { fromExif } = computeProvenance(photo, null);

    expect(fromExif.has('gain')).toBe(true); // 0 is a real value
    expect(fromExif.has('sensor_temp_c')).toBe(true);
    expect(fromExif.has('ra_deg')).toBe(true);
    expect(fromExif.has('dec_deg')).toBe(true); // 0° dec is valid
    expect(fromExif.has('iso')).toBe(false);
    expect(fromExif.has('sessions')).toBe(false);
    expect(fromExif.has('lens')).toBe(false); // empty string
    expect(fromExif.has('exposure_s')).toBe(false);
  });
});
