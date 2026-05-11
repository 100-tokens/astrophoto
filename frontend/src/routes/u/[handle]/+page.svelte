<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import HeroPage from '$lib/components/profile/HeroPage.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import ProfileEditor from '$lib/components/profile/editor/ProfileEditor.svelte';
  import CoverPickerModal from '$lib/components/profile/editor/CoverPickerModal.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  type EditorSection = 'identity' | 'about' | 'equipment' | 'location' | 'social';

  let editorOpen = $state(false);
  let editorSection = $state<EditorSection | null>(null);
  let coverPickerOpen = $state(false);

  let title = $derived(`${data.profile.display_name} — Astrophoto`);

  // ── SEO / GEO meta ─────────────────────────────────────────────
  const CDN_BASE: string = (import.meta.env.VITE_CDN_BASE_URL as string | undefined) ?? '';

  let metaDescription = $derived.by(() => {
    const p = data.profile;
    if (p.tagline) return p.tagline;
    const stats = p.stats;
    const frames = Number(stats.frames);
    const targets = Number(stats.targets);
    const followers = Number(stats.followers);
    const bits = [
      `${frames} frame${frames === 1 ? '' : 's'}`,
      `${targets} target${targets === 1 ? '' : 's'}`,
      followers > 0 ? `${followers} follower${followers === 1 ? '' : 's'}` : null
    ].filter(Boolean);
    return `${p.display_name} (@${p.handle}) on Astrophoto — ${bits.join(' · ')}.`;
  });

  let canonicalUrl = $derived(`${page.url.origin}/u/${encodeURIComponent(data.profile.handle)}`);

  let ogImage = $derived.by(() => {
    if (!data.profile.cover) return `${page.url.origin}/favicon.svg`;
    return CDN_BASE
      ? `${CDN_BASE}/img/${data.profile.cover.id}?w=1200`
      : `${page.url.origin}/api/photos/${data.profile.cover.id}/thumb/1200`;
  });

  // schema.org Person — gives AI engines a clean photographer entity to
  // resolve "@handle" mentions to. The stats subset becomes
  // additionalProperty so a query like "photographers with 100+ frames"
  // can index against it.
  let jsonLd = $derived.by(() => {
    const p = data.profile;
    const obj: Record<string, unknown> = {
      '@context': 'https://schema.org',
      '@type': 'Person',
      '@id': canonicalUrl,
      name: p.display_name,
      alternateName: `@${p.handle}`,
      url: canonicalUrl,
      ...(p.tagline ? { description: p.tagline } : {}),
      ...(p.bio_html ? { knowsAbout: 'astrophotography' } : {}),
      ...(p.location?.location_text ? { homeLocation: p.location.location_text } : {}),
      additionalProperty: [
        { '@type': 'PropertyValue', name: 'frames', value: p.stats.frames },
        { '@type': 'PropertyValue', name: 'targets', value: p.stats.targets },
        {
          '@type': 'PropertyValue',
          name: 'integrationSeconds',
          value: p.stats.integration_seconds
        },
        { '@type': 'PropertyValue', name: 'followers', value: p.stats.followers }
      ]
    };
    return JSON.stringify(obj).replace(/</g, '\\u003c');
  });
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={metaDescription} />
  <link rel="canonical" href={canonicalUrl} />

  <meta property="og:type" content="profile" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={metaDescription} />
  <meta property="og:url" content={canonicalUrl} />
  <meta property="og:image" content={ogImage} />
  <meta property="profile:username" content={data.profile.handle} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={metaDescription} />
  <meta name="twitter:image" content={ogImage} />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<AppHeader />

<HeroPage
  profile={data.profile}
  viewMode={data.viewMode}
  firstPage={data.firstPage}
  onEditProfile={(s) => {
    editorSection = s ?? null;
    editorOpen = true;
  }}
  onPickCover={() => (coverPickerOpen = true)}
/>

{#if data.viewMode === 'owner'}
  <ProfileEditor
    bind:open={editorOpen}
    bind:section={editorSection}
    onSaved={() => void invalidateAll()}
  />
  <CoverPickerModal
    bind:open={coverPickerOpen}
    handle={data.profile.handle}
    onPicked={() => void invalidateAll()}
  />
{/if}

<LightboxHost />
<AppFooter />
