<script lang="ts">
  /**
   * Deep-links to scientific archives keyed by object name.
   * No proxy, no cache — just URL-encoded search queries that take the user
   * to the archive's own search results in a new tab.
   *
   * For Messier objects we prefer the "M N" alias form (SIMBAD/NED prefer it)
   * over the common name when both are available.
   */
  interface Props {
    /** Canonical name, e.g. "Andromeda Galaxy", "NGC 7000". */
    canonicalName: string;
    /** Aliases array — used to prefer a catalog-form query when present. */
    aliases?: string[];
    /** Slug — used to derive a fallback query when canonicalName is generic. */
    slug?: string;
  }

  let { canonicalName, aliases = [], slug }: Props = $props();

  /**
   * Pick the query that scientific archives are most likely to resolve.
   * Order: any "M N" alias > any "NGC N" alias > any "IC N" alias > canonical.
   * Slug-based fallback is only used when canonical is empty.
   */
  function pickQuery(): string {
    const m = aliases.find((a) => /^M\s+\d+$/.test(a));
    if (m) return m;
    const ngc = aliases.find((a) => /^NGC\s+\d+$/.test(a));
    if (ngc) return ngc;
    const ic = aliases.find((a) => /^IC\s+\d+$/.test(a));
    if (ic) return ic;
    if (canonicalName) return canonicalName;
    if (slug) return slug.replace(/-/g, ' ').toUpperCase();
    return '';
  }

  const query = $derived(pickQuery());
  const enc = $derived(encodeURIComponent(query));

  const links = $derived([
    {
      label: 'SIMBAD',
      hint: 'astronomical database (CDS)',
      href: `https://simbad.cds.unistra.fr/simbad/sim-id?Ident=${enc}`
    },
    {
      label: 'NED',
      hint: 'NASA/IPAC Extragalactic Database',
      href: `https://ned.ipac.caltech.edu/byname?objname=${enc}`
    },
    {
      label: 'ESA / Hubble',
      hint: 'Hubble Space Telescope images',
      href: `https://esahubble.org/images/search/?search=${enc}`
    },
    {
      label: 'ESA / Webb',
      hint: 'James Webb Space Telescope images',
      href: `https://esawebb.org/images/search/?search=${enc}`
    },
    {
      label: 'NASA Image Library',
      hint: 'general NASA imagery',
      href: `https://images.nasa.gov/search?q=${enc}&media_type=image`
    }
  ]);
</script>

{#if query}
  <aside class="external-archives" aria-label="External scientific archives">
    <h2>Find this target on</h2>
    <ul>
      {#each links as l (l.label)}
        <li>
          <a href={l.href} target="_blank" rel="noopener noreferrer">
            <span class="archive-label">{l.label}</span>
            <span class="archive-hint">{l.hint}</span>
            <span aria-hidden="true" class="archive-arrow">↗</span>
          </a>
        </li>
      {/each}
    </ul>
    <p class="search-note">
      Searches are routed by the catalog name <code>{query}</code>. Archives are independent of
      Astrophoto and may show no results for some objects.
    </p>
  </aside>
{/if}

<style>
  .external-archives {
    margin: 1.5rem auto;
    max-width: 1100px;
    padding: 1rem;
    background: var(--bg-elevated, #141414);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: var(--r-md);
  }
  .external-archives h2 {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0 0 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 0.5rem;
  }
  li a {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    background: var(--bg-raised, #1a1a1a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: var(--r-sm, 4px);
    color: inherit;
    text-decoration: none;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  li a:hover {
    border-color: var(--accent);
    background: var(--bg-accent-tint, rgba(232, 164, 58, 0.07));
  }
  .archive-label {
    font-weight: 600;
  }
  .archive-hint {
    font-size: 0.8rem;
    color: var(--fg-muted, #888);
  }
  .archive-arrow {
    color: var(--fg-muted, #888);
    font-size: 0.95rem;
  }
  .search-note {
    margin: 0.75rem 0 0;
    font-size: 0.75rem;
    color: var(--fg-muted, #888);
  }
  .search-note code {
    font-family: var(--font-mono);
    background: var(--bg-canvas);
    padding: 0 0.3em;
    border-radius: var(--r-sm);
  }
</style>
