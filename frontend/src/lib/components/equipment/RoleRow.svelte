<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		kind: string;
		value?: string;
		badge?: string;
		expanded?: boolean;
		onToggle?: () => void;
		children?: Snippet;
	};

	let { kind, value = '', badge, expanded = false, onToggle, children }: Props = $props();
</script>

<div class="role-row">
	<div class="role-row-head">
		<span class="t-label">{kind}</span>
		<div class="role-row-input">
			<input class="input input-mono" {value} />
			{#if badge}
				<span class="chip">{badge}</span>
			{/if}
		</div>
		<button type="button" class="btn btn-ghost btn-sm" onclick={() => onToggle?.()}>
			{expanded ? 'Hide specs' : 'Edit specs'}
		</button>
	</div>
	{#if expanded}
		<div class="role-row-panel">{@render children?.()}</div>
	{/if}
</div>

<style>
	.role-row {
		border-top: 1px solid var(--border-subtle);
		padding: 20px 0;
	}
	.role-row-head {
		display: grid;
		grid-template-columns: 140px 1fr auto;
		gap: 16px;
		align-items: center;
	}
	.role-row-input {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.role-row-panel {
		margin-left: 156px;
		margin-top: 16px;
	}
</style>
