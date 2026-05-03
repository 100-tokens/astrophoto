<script lang="ts">
  import type { SocialLink } from '$lib/api/SocialLink';
  import type { SocialPlatform } from '$lib/api/SocialPlatform';

  let {
    links = [],
    onCommit
  }: {
    links?: SocialLink[];
    onCommit: (patch: { social_links: SocialLink[] }) => Promise<void>;
  } = $props();

  let local = $state<SocialLink[]>(structuredClone(links));
  let saved = $state<SocialLink[]>(structuredClone(links));

  const PLATFORMS: SocialPlatform[] = [
    'twitter',
    'instagram',
    'bluesky',
    'astrobin',
    'mastodon',
    'youtube',
    'website'
  ];

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(local);
  }

  async function commit() {
    if (!changed()) return;
    await onCommit({ social_links: local });
    saved = structuredClone(local);
  }

  function add() {
    if (local.length >= 6) return;
    const used = new Set(local.map((l) => l.platform));
    const next = PLATFORMS.find((p) => !used.has(p)) ?? 'website';
    local = [...local, { platform: next, url: '' }];
  }

  function remove(i: number) {
    local = local.filter((_, idx) => idx !== i);
    void commit();
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Social links</legend>
  {#each local as link, i (i)}
    <div class="row">
      <select
        value={link.platform}
        onchange={(e) => {
          local = local.map((l, idx) =>
            idx === i
              ? { ...l, platform: (e.target as HTMLSelectElement).value as SocialPlatform }
              : l
          );
        }}
      >
        {#each PLATFORMS as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
      <input
        type="url"
        placeholder="https://…"
        value={link.url}
        oninput={(e) => {
          local = local.map((l, idx) =>
            idx === i ? { ...l, url: (e.target as HTMLInputElement).value } : l
          );
        }}
      />
      <button type="button" class="remove" aria-label="Remove" onclick={() => remove(i)}>×</button>
    </div>
  {/each}
  {#if local.length < 6}
    <button type="button" class="add" onclick={add}>+ Add link</button>
  {/if}
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
  }
  .row {
    display: grid;
    grid-template-columns: 140px 1fr 32px;
    gap: 8px;
    margin-bottom: 8px;
  }
  .row input,
  .row select {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 6px 8px;
    font-size: 13px;
  }
  .remove {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid var(--border-subtle);
    cursor: pointer;
  }
  .add {
    background: transparent;
    color: var(--accent);
    border: 0;
    padding: 6px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
