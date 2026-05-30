// Apparent-size buckets for the /t catalog filter, hinted by the focal length
// they suit. Each maps to an inclusive lower / exclusive upper bound on the
// object's major axis (arcmin); `min`/`max` undefined means open-ended. The
// hint is guidance text only — no sensor/FOV math (see the design spec).

export interface SizeBucket {
  key: string;
  label: string;
  /** Focal-length range this size tends to frame well. */
  hint: string;
  min?: number;
  max?: number;
}

export const SIZE_BUCKETS: SizeBucket[] = [
  { key: 'xl', label: 'Very large · >60′', hint: '<400mm', min: 60 },
  { key: 'l', label: 'Large · 30–60′', hint: '400–800mm', min: 30, max: 60 },
  { key: 'm', label: 'Medium · 10–30′', hint: '700–1500mm', min: 10, max: 30 },
  { key: 's', label: 'Small · 2–10′', hint: '1500mm+', min: 2, max: 10 }
];

export function sizeBucketByKey(key: string | null | undefined): SizeBucket | undefined {
  if (!key) return undefined;
  return SIZE_BUCKETS.find((b) => b.key === key);
}
