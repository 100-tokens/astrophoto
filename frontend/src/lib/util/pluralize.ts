/** Format a count with en-US locale and the right singular/plural noun. */
export function pluralize(n: bigint | number, singular: string, plural?: string): string {
  const num = Number(n);
  const noun = num === 1 ? singular : (plural ?? `${singular}s`);
  return `${num.toLocaleString('en-US')} ${noun}`;
}
