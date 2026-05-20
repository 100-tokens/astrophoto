<script lang="ts">
  // VerifyHero — eyebrow + display H1 + italic intro paragraph. Editorial,
  // mono-typed, matches signin/signup density. See handoff §VerifyHero.

  interface Props {
    eyebrow?: string;
    title?: string;
    intro?: string;
  }

  let {
    eyebrow = 'NEW FRAME',
    title = 'Verify the data.',
    intro = "Your camera and the plate-solver already wrote down most of this. Glance through, correct anything off, fill what's still empty — none of it is required."
  }: Props = $props();
</script>

<header class="hero">
  <div class="t-eyebrow">{eyebrow}</div>
  <h1 class="hero-title">{title}</h1>
  {#if intro}
    <p class="hero-intro">{intro}</p>
  {/if}
</header>

<style>
  .hero {
    padding: 0;
  }
  .hero-title {
    font-family: var(--font-display);
    font-size: 64px;
    font-weight: 500;
    line-height: 1;
    letter-spacing: -0.02em;
    margin: 12px 0 14px;
  }
  .hero-intro {
    font-family: var(--font-display);
    font-style: italic;
    font-weight: 400;
    font-size: 17px;
    line-height: 1.5;
    color: var(--fg-secondary);
    max-width: 720px;
    margin: 0;
  }
  @media (max-width: 768px) {
    .hero-title {
      font-size: 44px;
    }
    .hero-intro {
      font-size: 15px;
    }
  }
</style>
