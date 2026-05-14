<script lang="ts">
  import { page } from '$app/state';
  import FollowButton from '$lib/components/FollowButton.svelte';

  let {
    targetUserId,
    isOwner,
    onEditProfile
  }: {
    targetUserId: string;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let initialFollowing = $derived(page.data.user?.following_ids?.includes(targetUserId) ?? false);
</script>

<div class="actions">
  {#if isOwner}
    <button type="button" class="btn-primary" onclick={onEditProfile}>Edit profile</button>
  {:else}
    <FollowButton userId={targetUserId} {initialFollowing} />
  {/if}
</div>

<style>
  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    /* Keep the Edit/Follow button at its natural width when the parent
       grid collapses to a single column on mobile — without this the
       button stretches the full viewport width and reads as a primary
       page CTA, not a hero-row affordance. */
    align-items: flex-start;
  }
  .btn-primary {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 10px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
