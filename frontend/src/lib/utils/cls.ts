/** Join class names, dropping falsy values. */
export function cls(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(' ');
}
