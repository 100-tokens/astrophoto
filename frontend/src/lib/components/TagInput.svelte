<script lang="ts">
  interface Props {
    name?: string;
    value?: string[];
  }

  let {
    name = 'tags',
    value = $bindable<string[]>([]),
  }: Props = $props();

  let buf = $state('');

  function commit() {
    const s = buf.trim().toLowerCase();
    if (!s) return;
    if (value.includes(s)) { buf = ''; return; }
    if (value.length >= 8) return;
    value = [...value, s];
    buf = '';
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Backspace' && !buf && value.length) {
      value = value.slice(0, -1);
    }
  }
</script>

<!-- Label targets the text input; the hidden input carries the JSON array -->
<!-- svelte-ignore a11y_label_has_associated_control -->
<label class="t-label" for="{name}-input">TAGS · max 8</label>
<div class="tags">
  {#each value as t}
    <span class="chip">
      {t}
      <button
        type="button"
        onclick={() => (value = value.filter((x) => x !== t))}
        aria-label={`remove ${t}`}
      >×</button>
    </span>
  {/each}
  <input
    id="{name}-input"
    bind:value={buf}
    onkeydown={onKeydown}
    onblur={commit}
    class="input input-mono tags-input"
    placeholder={value.length >= 8 ? '' : 'widefield, narrowband…'}
    disabled={value.length >= 8}
    autocomplete="off"
  />
  <!-- JSON array string so the form action receives Vec<String> after JSON.parse -->
  <input type="hidden" {name} value={JSON.stringify(value)} />
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
  }
  .tags .chip button {
    margin-left: 4px;
    background: none;
    color: inherit;
    border: none;
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
  }
  .tags-input {
    flex: 1;
    min-width: 120px;
  }
</style>
