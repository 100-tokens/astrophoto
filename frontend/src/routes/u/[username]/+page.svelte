<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Photo from '$lib/components/Photo.svelte';
  import FollowButton from '$lib/components/FollowButton.svelte';
  import PhotoTitle from '$lib/components/photos/PhotoTitle.svelte';
  import type { User, Photo as PhotoData } from '$lib/data/photos';

  interface ProfilePhoto extends Omit<PhotoData, 'target'> {
    target: string | null;
    thumbSrc?: string;
  }

  interface PageData {
    profile: User;
    photos: ProfilePhoto[];
    isFollowing?: boolean;
    isSelf?: boolean;
  }

  let { data }: { data: PageData } = $props();

  let u = $derived(data.profile);
  let photos = $derived(data.photos);

  const TABS = [
    { label: 'Frames', count: '' },
    { label: 'Collections', count: '' },
    { label: 'Equipment', count: '' },
    { label: 'About', count: '' }
  ];

  let activeTab = $state(0);
</script>

<AppHeader />

<!-- Hero section -->
<section class="profile-hero">
  <!-- Avatar circle -->
  <div class="profile-avatar" aria-hidden="true">
    {u.initial}
  </div>

  <!-- Center column: name + about + stats -->
  <div class="profile-info">
    <div class="t-eyebrow" style="margin-bottom: 8px;">
      PRACTITIONER · MEMBER SINCE {u.memberSince}
    </div>
    <h1 class="profile-name">
      {u.firstName} <em>{u.surnameItalic}</em>
    </h1>
    <p class="profile-about">{u.about}</p>
    <div class="profile-stats">
      <div>
        <span class="stat-num">{u.frames}</span><br />
        <span class="stat-label">frames</span>
      </div>
      <div>
        <span class="stat-num">{u.integrationTotal}</span><br />
        <span class="stat-label">integration</span>
      </div>
      <div>
        <span class="stat-num">{u.followers.toLocaleString()}</span><br />
        <span class="stat-label">followers</span>
      </div>
      <div>
        <span class="stat-num">{u.collections}</span><br />
        <span class="stat-label">collections</span>
      </div>
    </div>
  </div>

  <!-- Right column: actions + location -->
  <div class="profile-actions">
    {#if !data.isSelf}
      <FollowButton userId={data.profile.username} initialFollowing={data.isFollowing ?? false} />
    {/if}
    <button class="btn btn-secondary">Message</button>
    {#if u.bortle > 0}
      <div class="t-meta profile-location">
        {u.lat} · {u.long}<br />
        Bortle {u.bortle} · SQM {u.sqm}
      </div>
    {/if}
  </div>
</section>

<!-- Equipment strip -->
{#if u.equipment.scope !== '—'}
  <section class="equipment-strip">
    <div><span class="eq-label">SCOPE</span> &nbsp; {u.equipment.scope}</div>
    <div><span class="eq-label">CAM</span> &nbsp; {u.equipment.camera}</div>
    <div><span class="eq-label">MOUNT</span> &nbsp; {u.equipment.mount}</div>
    <div><span class="eq-label">FILTERS</span> &nbsp; {u.equipment.filters}</div>
  </section>
{/if}

<!-- Tabs -->
<section class="tabs-bar">
  {#each TABS as tab, i}
    <button
      class="nav-link {i === activeTab ? 'active' : ''}"
      style="padding: 20px 0; border: none; background: none; cursor: pointer;"
      onclick={() => (activeTab = i)}
    >
      {tab.label}{#if i === 0}
        · {u.frames}{:else if i === 1}
        · {u.collections}{/if}
    </button>
  {/each}
  <div class="tabs-right">
    <span class="t-label">SORT</span>
    <button class="chip">Newest ▾</button>
  </div>
</section>

<!-- Photo grid (4 columns, square) -->
{#if activeTab === 0}
  <section class="photo-grid-section">
    {#if photos.length === 0}
      <p class="t-meta" style="padding: 32px 64px; color: var(--fg-muted);">No photos yet.</p>
    {:else}
      <div class="photo-grid">
        {#each photos as photo}
          <div class="grid-item">
            <a href="/photo/{photo.slug}" class="grid-photo-link" aria-label={photo.target ?? 'Untitled'}>
              <div class="grid-photo-inner">
                <Photo
                  target={photo.target ?? ''}
                  src={photo.thumbSrc}
                  style="position: absolute; inset: 0;"
                />
              </div>
            </a>
            <div class="grid-caption">
              <span class="photo-target"><PhotoTitle photo={{ target: photo.target }} size="md" /></span>
              <span class="photo-integration">{photo.integration}</span>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{:else if activeTab === 1}
  <section class="empty-tab">
    <p class="t-meta">Collections coming in Phase 3.</p>
  </section>
{:else if activeTab === 2}
  <section class="empty-tab">
    <p class="t-meta">Equipment details coming in Phase 3.</p>
  </section>
{:else}
  <section class="empty-tab">
    <p class="t-meta">{u.about}</p>
  </section>
{/if}

<AppFooter />

<style>
  /* ── Hero ─────────────────────────────────────────────────── */
  .profile-hero {
    padding: 64px 64px 32px;
    display: grid;
    grid-template-columns: 120px 1fr auto;
    gap: 32px;
    align-items: start;
    border-bottom: 1px solid var(--border-subtle);
  }

  .profile-avatar {
    width: 120px;
    height: 120px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 56px;
    flex-shrink: 0;
  }

  .profile-name {
    font-family: var(--font-display);
    font-size: 64px;
    font-weight: 400;
    margin: 0;
    line-height: 1;
  }

  .profile-about {
    margin-top: 16px;
    font-size: 15px;
    color: var(--fg-secondary);
    max-width: 640px;
  }

  .profile-stats {
    display: flex;
    gap: 24px;
    margin-top: 24px;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .stat-num {
    color: var(--fg-primary);
    font-size: 22px;
  }

  .stat-label {
    color: var(--fg-muted);
  }

  .profile-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-location {
    margin-top: 8px;
    text-align: right;
  }

  /* ── Equipment strip ──────────────────────────────────────── */
  .equipment-strip {
    padding: 20px 64px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    gap: 32px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    flex-wrap: wrap;
  }

  .eq-label {
    color: var(--fg-muted);
  }

  /* ── Tabs ─────────────────────────────────────────────────── */
  .tabs-bar {
    padding: 0 64px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    gap: 40px;
    align-items: center;
  }

  .tabs-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* ── Photo grid ───────────────────────────────────────────── */
  .photo-grid-section {
    padding: 32px 64px;
  }

  .photo-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
  }

  .grid-photo-link {
    display: block;
  }

  .grid-photo-inner {
    position: relative;
    aspect-ratio: 1 / 1;
    overflow: hidden;
  }

  .grid-caption {
    display: flex;
    justify-content: space-between;
    padding: 8px 2px;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .photo-target {
    color: var(--fg-primary);
  }

  .photo-integration {
    color: var(--fg-muted);
  }

  .empty-tab {
    padding: 48px 64px;
    color: var(--fg-muted);
  }

  /* ── Responsive ───────────────────────────────────────────── */
  @media (max-width: 900px) {
    .profile-hero {
      grid-template-columns: 80px 1fr;
      padding: 32px 32px 24px;
      gap: 20px;
    }

    .profile-avatar {
      width: 80px;
      height: 80px;
      font-size: 36px;
    }

    .profile-name {
      font-size: 40px;
    }

    .profile-actions {
      grid-column: 1 / -1;
      flex-direction: row;
      flex-wrap: wrap;
    }

    .profile-location {
      text-align: left;
      width: 100%;
    }

    .equipment-strip {
      padding: 16px 32px;
    }

    .tabs-bar {
      padding: 0 32px;
      gap: 24px;
      overflow-x: auto;
    }

    .photo-grid-section {
      padding: 24px 32px;
    }

    .photo-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 480px) {
    .photo-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 8px;
    }

    .profile-hero {
      padding: 24px 16px;
    }
  }
</style>
