<script lang="ts">
  import FieldShell from './FieldShell.svelte';

  // CategoryRadio — segmented radio group. Six first-class options at full
  // width, "OTHER" pushed below as a discrete typographic underline option.
  //
  // IMPORTANT contract: the hidden input MUST emit the lowercase token
  // ('dso', 'planetary', 'lunar', 'solar', 'wide_field', 'nightscape',
  // 'other') so the backend column matches existing rows and other UI
  // (filters, browse) keeps working. The visible labels are uppercased
  // for display only.

  interface Props {
    name?: string;
    value?: string;
  }

  type Opt = { value: string; label: string };
  const OPTS: Opt[] = [
    { value: 'dso', label: 'DSO' },
    { value: 'planetary', label: 'PLANETARY' },
    { value: 'lunar', label: 'LUNAR' },
    { value: 'solar', label: 'SOLAR' },
    { value: 'wide_field', label: 'WIDE FIELD' },
    { value: 'nightscape', label: 'NIGHTSCAPE' }
  ];
  const OTHER_VALUE = 'other';

  let { name = 'category', value = $bindable('dso') }: Props = $props();

  let isOther = $derived(value === OTHER_VALUE);

  function setActive(v: string) {
    value = v;
  }

  function onKey(e: KeyboardEvent, idx: number) {
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      e.preventDefault();
      const next = OPTS[(idx + 1) % OPTS.length];
      if (next) {
        setActive(next.value);
        focusBtn((idx + 1) % OPTS.length);
      }
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      e.preventDefault();
      const prev = OPTS[(idx - 1 + OPTS.length) % OPTS.length];
      if (prev) {
        setActive(prev.value);
        focusBtn((idx - 1 + OPTS.length) % OPTS.length);
      }
    } else if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      const o = OPTS[idx];
      if (o) setActive(o.value);
    }
  }

  function focusBtn(idx: number) {
    const root = document.querySelector(`[data-cat-root="${name}"]`);
    if (!root) return;
    const btns = root.querySelectorAll<HTMLButtonElement>('button[role="radio"]');
    btns[idx]?.focus();
  }
</script>

<FieldShell label="CATEGORY" full>
  <div class="cat-group" role="radiogroup" aria-label="Category" data-cat-root={name}>
    {#each OPTS as o, i (o.value)}
      {@const on = value === o.value}
      <button
        type="button"
        role="radio"
        class:active={on}
        aria-checked={on}
        tabindex={on || (!OPTS.some((x) => x.value === value) && i === 0) ? 0 : -1}
        onclick={() => setActive(o.value)}
        onkeydown={(e) => onKey(e, i)}
      >
        {#if on}<span class="cat-stripe" aria-hidden="true"></span>{/if}
        <span>{o.label}</span>
      </button>
    {/each}
  </div>
  <div class="cat-other">
    <span class="t-meta cat-or">OR</span>
    <button
      type="button"
      class="cat-other-btn"
      class:active={isOther}
      onclick={() => setActive(OTHER_VALUE)}
    >
      Other — describe in caption
    </button>
  </div>
  <input type="hidden" {name} {value} />
</FieldShell>

<style>
  .cat-group {
    display: flex;
    align-items: stretch;
    border: 1px solid var(--border-default);
    border-radius: var(--r-sm);
    background: var(--bg-base);
    overflow: hidden;
  }
  .cat-group button {
    flex: 1;
    height: 42px;
    background: transparent;
    color: var(--fg-secondary);
    border-right: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    cursor: pointer;
    position: relative;
    padding: 0;
  }
  .cat-group button:last-child {
    border-right: 0;
  }
  .cat-group button:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px rgba(232, 164, 58, 0.4);
  }
  .cat-group button.active {
    background: rgba(232, 164, 58, 0.1);
    color: var(--fg-primary);
  }
  .cat-stripe {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--accent);
  }
  .cat-other {
    margin-top: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cat-or {
    color: var(--fg-faint);
  }
  .cat-other-btn {
    background: transparent;
    border: 0;
    padding: 2px 0;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--fg-muted);
    text-decoration: none;
    cursor: pointer;
    text-underline-offset: 3px;
  }
  .cat-other-btn:hover {
    color: var(--fg-secondary);
  }
  .cat-other-btn.active {
    color: var(--accent);
    text-decoration: underline;
  }
  .cat-other-btn:focus-visible {
    outline: 2px solid rgba(232, 164, 58, 0.4);
    outline-offset: 2px;
  }
  @media (max-width: 640px) {
    .cat-group {
      flex-wrap: wrap;
    }
    .cat-group button {
      flex: 0 0 calc(100% / 3);
      border-bottom: 1px solid var(--border-subtle);
    }
    .cat-group button:nth-child(n + 4) {
      border-bottom: 0;
    }
    .cat-group button:nth-child(3n) {
      border-right: 0;
    }
  }
</style>
