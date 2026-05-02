<script lang="ts">
  import DraftCard from './DraftCard.svelte';
  let { drafts }: { drafts: Array<{ id: string; target?: string | null; original_name: string; last_step?: string | null; status: string }> } = $props();
  let displayed = $derived(drafts.slice(0, 3));
</script>

<section class="callout">
  <div class="callout-header">
    <span class="t-eyebrow accent">● {drafts.length} DRAFTS · NOT YET PUBLISHED</span>
    <a href="/account/frames?filter=drafts" class="t-meta">SEE ALL DRAFTS →</a>
  </div>
  <div class="grid">
    {#each displayed as draft (draft.id)}
      <DraftCard photo={draft} />
    {/each}
  </div>
</section>

<style>
  .callout { padding: 24px 64px; background: rgba(208, 160, 80, 0.06); margin: 24px -64px; }
  .callout-header { display: flex; justify-content: space-between; margin-bottom: 16px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } .callout { padding: 24px; margin: 24px 0; } }
</style>
