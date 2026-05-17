<script lang="ts">
  import './filter-chip.css';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import { FILTER_TYPE_META, bandwidthLabel } from '$lib/equipment/filter-types';

  type Props = {
    filter: PhotoFilterChip;
    draggable?: boolean;
    removable?: boolean;
    compact?: boolean;
    dragging?: boolean;
    onRemove?: () => void;
    onAddType?: () => void;
  };

  let {
    filter,
    draggable = false,
    removable = false,
    compact = false,
    dragging = false,
    onRemove,
    onAddType
  }: Props = $props();

  const meta = $derived(filter.filter_type ? FILTER_TYPE_META[filter.filter_type] : null);
  const isUntyped = $derived(!meta);
  const bw = $derived(bandwidthLabel(filter));
  const cls = $derived(
    'fchip ' +
      (isUntyped ? 'is-untyped' : 'is-' + filter.filter_type) +
      (dragging ? ' is-dragging' : '')
  );
  const title = $derived(
    meta ? `${meta.label}${bw ? ' · ' + bw : ''}` : 'Untyped filter — add a type'
  );
</script>

<span class={cls} {title}>
  <span class="fchip-badge">{meta ? meta.code : '?'}</span>
  <span class="fchip-name">{filter.display_name}</span>
  {#if bw && !compact}
    <span class="fchip-bw">{bw}</span>
  {/if}
  {#if isUntyped && !compact}
    <button class="fchip-addtype" type="button" onclick={onAddType}>+ type</button>
  {/if}
  {#if draggable}
    <span class="fchip-grip" title="Drag to reorder">
      <svg width="8" height="12" viewBox="0 0 8 12" fill="currentColor">
        <circle cx="1.5" cy="2" r="0.9" />
        <circle cx="6.5" cy="2" r="0.9" />
        <circle cx="1.5" cy="6" r="0.9" />
        <circle cx="6.5" cy="6" r="0.9" />
        <circle cx="1.5" cy="10" r="0.9" />
        <circle cx="6.5" cy="10" r="0.9" />
      </svg>
    </span>
  {/if}
  {#if removable}
    <button
      class="fchip-x"
      type="button"
      onclick={onRemove}
      title="Remove"
      aria-label="Remove {filter.display_name}"
    >
      <svg
        width="9"
        height="9"
        viewBox="0 0 9 9"
        fill="none"
        stroke="currentColor"
        stroke-width="1.4"
        stroke-linecap="round"
      >
        <path d="M2 2 L7 7 M7 2 L2 7" />
      </svg>
    </button>
  {/if}
</span>
