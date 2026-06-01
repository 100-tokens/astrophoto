<script lang="ts">
  // SOTA assisted text input (ARIA combobox) backed by a list-of-values.
  //
  // - Type-ahead filters the LOV (case/punctuation-insensitive); usage counts
  //   are shown so the most-used existing value is obvious.
  // - Keyboard: ↓/↑ move, Enter picks the highlighted option, Esc closes.
  // - Free entry is ALWAYS allowed (you can create a genuinely new value).
  // - Duplicate-avoidance is advisory, never blocking:
  //     · typed value matches an existing one only after normalising
  //       (lowercase + strip non-alphanumerics) → "Did you mean 'X'?" with a
  //       one-click apply (catches "SkyWatcher" vs "Sky-Watcher").
  //     · typed value has no match at all → "New — will be added to the catalog".
  //     · exact existing value → a quiet "existing" tick.
  //   (The DB's real uniqueness is on the exact canonical name; this is help,
  //   not a guarantee.)

  import type { CatalogValue } from '$lib/api/CatalogValue';

  type Props = {
    value: string;
    options: CatalogValue[];
    label: string;
    placeholder?: string;
    disabled?: boolean;
    required?: boolean;
    onChange: (value: string) => void;
  };

  let {
    value,
    options,
    label,
    placeholder = '',
    disabled = false,
    required = false,
    onChange
  }: Props = $props();

  let open = $state(false);
  let highlighted = $state(-1);
  const uid = `cb-${Math.random().toString(36).slice(2, 8)}`;

  const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

  let filtered = $derived.by(() => {
    const raw = value.trim();
    if (!raw) return options;
    const lc = raw.toLowerCase();
    const nv = norm(raw);
    return options.filter((o) => o.value.toLowerCase().includes(lc) || norm(o.value).includes(nv));
  });

  let exact = $derived(options.some((o) => o.value === value.trim()) && value.trim() !== '');
  let near = $derived.by(() => {
    const raw = value.trim();
    if (!raw || exact) return null;
    const nv = norm(raw);
    return options.find((o) => norm(o.value) === nv && o.value !== raw)?.value ?? null;
  });
  let isNew = $derived(value.trim().length > 0 && !exact && !near);

  function commit(v: string) {
    onChange(v);
    open = false;
    highlighted = -1;
  }

  function onInput(e: Event) {
    onChange((e.target as HTMLInputElement).value);
    open = true;
    highlighted = -1;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      open = true;
      highlighted = Math.min(highlighted + 1, filtered.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      highlighted = Math.max(-1, highlighted - 1);
    } else if (e.key === 'Enter') {
      const opt = filtered[highlighted];
      if (open && opt) {
        e.preventDefault();
        commit(opt.value);
      }
    } else if (e.key === 'Escape') {
      open = false;
      highlighted = -1;
    }
  }
</script>

<div class="cb">
  <span class="cb-label" class:req={required}>{label}</span>
  <div class="cb-field">
    <input
      class="cb-input"
      role="combobox"
      aria-expanded={open && filtered.length > 0}
      aria-controls="{uid}-list"
      aria-autocomplete="list"
      aria-activedescendant={highlighted >= 0 ? `${uid}-opt-${highlighted}` : undefined}
      {placeholder}
      {disabled}
      autocomplete="off"
      spellcheck="false"
      {value}
      oninput={onInput}
      onkeydown={onKeydown}
      onfocus={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 120)}
    />
    {#if open && filtered.length > 0}
      <ul class="cb-list" id="{uid}-list" role="listbox">
        {#each filtered as opt, i (opt.value)}
          <li
            id="{uid}-opt-{i}"
            role="option"
            aria-selected={i === highlighted}
            class:hl={i === highlighted}
            onmousedown={(e) => {
              e.preventDefault();
              commit(opt.value);
            }}
          >
            <span class="cb-opt-val">{opt.value}</span>
            <span class="cb-opt-count">{opt.count}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if near}
    <button
      type="button"
      class="cb-hint cb-hint--warn"
      onmousedown={(e) => e.preventDefault()}
      onclick={() => commit(near)}
    >
      Did you mean <strong>{near}</strong>? — use it
    </button>
  {:else if isNew}
    <span class="cb-hint cb-hint--new">New — will be added to the catalog.</span>
  {:else if exact}
    <span class="cb-hint cb-hint--ok">✓ existing catalog value</span>
  {/if}
</div>

<style>
  .cb {
    display: flex;
    flex-direction: column;
    gap: 6px;
    position: relative;
  }
  .cb-label {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .cb-label.req::after {
    content: ' *';
    color: var(--accent);
  }
  .cb-field {
    position: relative;
  }
  .cb-input {
    width: 100%;
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
  .cb-list {
    position: absolute;
    z-index: 30;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    margin: 0;
    padding: 4px 0;
    list-style: none;
    max-height: 240px;
    overflow-y: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default, var(--border-subtle));
    box-shadow: var(--shadow-md);
  }
  .cb-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 7px 10px;
    cursor: pointer;
    font-size: 14px;
  }
  .cb-list li.hl,
  .cb-list li:hover {
    background: var(--bg-raised);
  }
  .cb-opt-count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
  .cb-hint {
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: left;
    background: none;
    border: 0;
    padding: 0;
  }
  .cb-hint--warn {
    color: var(--accent);
    cursor: pointer;
    text-decoration: none;
  }
  .cb-hint--warn:hover {
    text-decoration: underline;
  }
  .cb-hint--new {
    color: var(--fg-muted);
  }
  .cb-hint--ok {
    color: var(--fg-muted);
    opacity: 0.7;
  }
</style>
