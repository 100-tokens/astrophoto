/**
 * Build a JSON-LD `<script>` tag string for injection via `{@html}` in
 * `<svelte:head>`.
 *
 * Why this lives in a `.ts` module instead of inline in markup: a literal
 * `<script ...>...</script>` inside an `{@html` `...` `}` template literal trips
 * `svelte-eslint-parser` (it misreads the `<script` opener as a real script
 * element and fails to parse the whole file), and the same literal inside a
 * `.svelte` `<script>` block would prematurely close it. Keeping the tag in a
 * plain `.ts` file dodges both — `ts.parser` reads the template literal as an
 * ordinary string.
 *
 * Pass an already-serialised JSON string. Escape user-controlled `<` to
 * `<` at the call site (so a stray `</script>` in the payload can't break
 * out of the tag); this helper does not transform the JSON.
 */
export function ldJsonScriptTag(json: string): string {
  return `<script type="application/ld+json">${json}</script>`;
}
