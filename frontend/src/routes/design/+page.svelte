<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Button from '$lib/components/Button.svelte';
  import Chip from '$lib/components/Chip.svelte';
  import CornerMarks from '$lib/components/CornerMarks.svelte';
  import ExifTable from '$lib/components/ExifTable.svelte';
  import Input from '$lib/components/Input.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import MarkReticle from '$lib/components/MarkReticle.svelte';
  import Photo from '$lib/components/Photo.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import Wordmark from '$lib/components/Wordmark.svelte';

  // Color token swatches
  const colorTokens = [
    { token: '--bg-canvas', hex: '#0c0a08', role: 'Page background' },
    { token: '--bg-base', hex: '#100d0a', role: 'Default surface' },
    { token: '--bg-raised', hex: '#16120e', role: 'Cards, panels' },
    { token: '--bg-elevated', hex: '#1d1812', role: 'Hover, popovers' },
    { token: '--border-subtle', hex: '#221d17', role: 'Hairlines' },
    { token: '--border-default', hex: '#2c2620', role: 'Inputs, dividers' },
    { token: '--border-strong', hex: '#3a322a', role: 'Buttons' },
    { token: '--fg-primary', hex: '#f8f1e6', role: 'Headlines, copy' },
    { token: '--fg-secondary', hex: '#d6cdba', role: 'Body, secondary' },
    { token: '--fg-muted', hex: '#9c9384', role: 'Meta, captions' },
    { token: '--fg-faint', hex: '#6a6358', role: 'Disabled' },
    { token: '--accent', hex: '#e8a43a', role: 'Sodium amber' },
    { token: '--accent-hover', hex: '#f0b455', role: 'Hover' },
    { token: '--accent-press', hex: '#c98920', role: 'Active' },
    { token: '--accent-dim', hex: '#7a5a18', role: 'Borders on accent' },
    { token: '--accent-ink', hex: '#0c0a08', role: 'Text on accent' },
    { token: '--success', hex: '#6b8e4e', role: 'Success' },
    { token: '--warning', hex: '#c98920', role: 'Warning' },
    { token: '--danger', hex: '#a8453a', role: 'Danger' },
    { token: '--info', hex: '#6b7d8e', role: 'Info' },
    { token: '--star-O', hex: '#aac4ff', role: 'Hot blue star' },
    { token: '--star-G', hex: '#fff4d8', role: 'Sun-like star' },
    { token: '--star-K', hex: '#ffcf94', role: 'Orange star' },
    { token: '--star-M', hex: '#ff9966', role: 'Red dwarf' }
  ];

  // Canonical NGC 7000 EXIF example from README
  const exifRows = [
    { label: 'Target', value: 'NGC 7000' },
    { label: 'Common name', value: 'North America Nebula' },
    { label: 'Captured', value: '2024-08-14 – 2024-08-18' },
    { label: 'Camera', value: 'ZWO ASI2600MC Pro', sublabel: 'Cooled CMOS, −10 °C' },
    { label: 'Telescope', value: 'Askar FRA500 f/7' },
    { label: 'Mount', value: 'Sky-Watcher EQ6-R Pro' },
    { label: 'Filters', value: 'Optolong L-eXtreme (dual-band)' },
    { label: 'Exposure', value: '180 × 360 s', sublabel: '= 18.0 hours', valueAccent: true },
    { label: 'Gain', value: '100 (unity)' },
    { label: 'RA / Dec', value: '20ʰ 58ᵐ 47ˢ / +44° 19′' },
    { label: 'Field', value: '2.8° × 2.0°' },
    { label: 'Pixel scale', value: '1.92 ″/px' }
  ];

  let inputVal = $state('');
  let monoInputVal = $state('20ʰ 58ᵐ 47ˢ / +44° 19′');
  let textareaVal = $state('Narrowband, 18 h integration over 4 nights...');

  interface PhotoSample {
    target: string;
    name: string;
  }
  const photoSamples: PhotoSample[] = [
    { target: 'NGC 7000', name: 'North America Nebula' },
    { target: 'M31', name: 'Andromeda Galaxy' },
    { target: 'Moon', name: 'Mare Imbrium' }
  ];
</script>

<svelte:head>
  <meta name="robots" content="noindex" />
  <title>Design System — Astrophoto</title>
</svelte:head>

<AppHeader active="Gallery" />

<main style="max-width: 1200px; margin: 0 auto; padding: 64px 32px;">
  <!-- Header -->
  <div
    style="padding-bottom: 40px; border-bottom: 1px solid var(--border-default); margin-bottom: 64px;"
  >
    <div class="t-eyebrow" style="color: var(--accent); margin-bottom: 16px;">
      ● ASTROPHOTO · DESIGN SYSTEM · v0.1
    </div>
    <h1 class="t-display" style="font-size: 64px; margin: 0 0 16px 0;">
      <em>Visual</em> language
    </h1>
    <p style="font-size: 16px; max-width: 640px; margin: 0;">
      Observatory at 3am. Warm near-black, sodium amber, a refined transitional serif against a
      technical sans and mono. Every photo is the hero — the chrome recedes.
    </p>
  </div>

  <!-- ─── 1. COLOR TOKENS ─── -->
  <section style="margin-bottom: 80px;">
    <div class="t-eyebrow" style="margin-bottom: 24px;">COLOR TOKENS</div>
    <div style="display: flex; flex-wrap: wrap; gap: 12px;">
      {#each colorTokens as { token, hex, role }}
        <div style="flex: 0 0 140px; min-width: 120px;">
          <div
            style="width: 100%; height: 64px; background: {hex}; border: 1px solid var(--border-subtle); border-radius: 2px;"
          ></div>
          <div
            style="margin-top: 6px; font-family: var(--font-mono); font-size: 11px; line-height: 1.5;"
          >
            <div style="color: var(--fg-primary);">{token}</div>
            <div style="color: var(--fg-muted);">{hex}</div>
            <div style="color: var(--fg-faint);">{role}</div>
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- ─── 2. TYPOGRAPHY ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 32px;">TYPOGRAPHY</div>
    <div style="display: flex; flex-direction: column; gap: 32px;">
      <div>
        <div class="t-meta" style="margin-bottom: 8px;">
          DISPLAY 88 · Source Serif 4 · weight 600
        </div>
        <div class="t-display" style="font-size: 88px; line-height: 0.95;">
          the work you <em>make</em>
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">
          DISPLAY 64 · Source Serif 4 · weight 600
        </div>
        <div class="t-display" style="font-size: 64px;">
          A quiet archive of <em>the night sky</em>
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">
          DISPLAY 48 · Source Serif 4 · weight 600
        </div>
        <div class="t-display" style="font-size: 48px;">
          NGC 7000 · <em>North America</em> Nebula
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">
          DISPLAY 32 · Source Serif 4 · weight 600
        </div>
        <div class="t-display" style="font-size: 32px;">
          A serious home for <em>the work you make</em>
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">BODY 16 · Inter · weight 400</div>
        <p style="font-size: 16px; margin: 0;">
          Astrophotographers work at 1am with dark-adapted eyes. Every upload is both a technical
          artifact and an aesthetic object worth presenting beautifully.
        </p>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">UI 14 · Inter · weight 400</div>
        <span style="font-size: 14px; font-family: var(--font-ui);">
          Buttons, navigation, form labels. Default body copy.
        </span>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">UI 13 · Inter · weight 400</div>
        <span style="font-size: 13px; font-family: var(--font-ui); color: var(--fg-secondary);">
          Dense lists, table rows, secondary metadata.
        </span>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">MONO 13 · JetBrains Mono</div>
        <span class="t-mono" style="font-size: 13px;">
          180 × 360 s = 18.0 hours · gain 100 · −10 °C · 20ʰ 58ᵐ 47ˢ / +44° 19′
        </span>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 8px;">
          LABEL 11 · JetBrains Mono · uppercase 0.16em
        </div>
        <span class="t-eyebrow">MEASUREMENTS, NOT PROSE · EXIF DATA · TARGETS</span>
      </div>
    </div>
  </section>

  <!-- ─── 3. BUTTONS ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">BUTTONS</div>
    <div style="display: flex; flex-direction: column; gap: 20px;">
      <div>
        <div class="t-label" style="margin-bottom: 12px;">PRIMARY</div>
        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
          <Button variant="primary" size="sm">Primary sm</Button>
          <Button variant="primary">Primary</Button>
          <Button variant="primary" size="lg">Primary lg</Button>
        </div>
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 12px;">SECONDARY</div>
        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
          <Button variant="secondary" size="sm">Secondary sm</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="secondary" size="lg">Secondary lg</Button>
        </div>
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 12px;">GHOST</div>
        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
          <Button variant="ghost" size="sm">Ghost sm</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="ghost" size="lg">Ghost lg</Button>
        </div>
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 12px;">DANGER</div>
        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
          <Button variant="danger" size="sm">Danger sm</Button>
          <Button variant="danger">Danger</Button>
          <Button variant="danger" size="lg">Danger lg</Button>
        </div>
      </div>
    </div>
  </section>

  <!-- ─── 4. INPUTS ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">
      INPUTS <span class="t-meta" style="text-transform: none; letter-spacing: 0;"
        >(click to focus)</span
      >
    </div>
    <div style="display: flex; flex-direction: column; gap: 16px; max-width: 400px;">
      <div>
        <div class="t-label" style="margin-bottom: 6px;">EMAIL (standard)</div>
        <Input bind:value={inputVal} placeholder="you@somewhere.com" type="email" />
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 6px;">RA / DEC (mono)</div>
        <Input bind:value={monoInputVal} mono={true} />
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 6px;">CAPTION (textarea)</div>
        <Textarea bind:value={textareaVal} rows={4} />
      </div>

      <div>
        <div class="t-label" style="margin-bottom: 6px;">SELECT</div>
        <select class="select">
          <option>SHO narrowband</option>
          <option>HOO broadband</option>
          <option>LRGB</option>
        </select>
      </div>
    </div>
  </section>

  <!-- ─── 5. CHIPS ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">CHIPS / BADGES</div>
    <div style="display: flex; flex-wrap: wrap; gap: 8px;">
      <Chip>Galaxies</Chip>
      <Chip>SHO narrowband</Chip>
      <Chip>Bortle 4</Chip>
      <Chip>HOO broadband</Chip>
      <Chip accent={true}>● 248 ♡</Chip>
      <Chip accent={true}>PUBLISHED</Chip>
      <Chip accent={true}>● LIVE</Chip>
    </div>
  </section>

  <!-- ─── 6. EXIF TABLE ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">EXIF TABLE · NGC 7000 example</div>
    <div style="max-width: 480px;">
      <ExifTable rows={exifRows} />
    </div>
  </section>

  <!-- ─── 7. LOGO LOCKUP ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">LOGO LOCKUP</div>
    <div style="display: flex; flex-direction: column; gap: 24px;">
      <div>
        <div class="t-meta" style="margin-bottom: 12px;">Mark only — 3 sizes</div>
        <div style="display: flex; align-items: center; gap: 24px;">
          <MarkReticle size={20} color="var(--accent)" />
          <MarkReticle size={28} color="var(--accent)" />
          <MarkReticle size={48} color="var(--accent)" />
          <MarkReticle size={72} color="var(--accent)" />
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 12px;">Wordmark only</div>
        <div style="display: flex; align-items: baseline; gap: 32px; flex-wrap: wrap;">
          <Wordmark size={16} />
          <Wordmark size={22} />
          <Wordmark size={32} />
          <Wordmark size={48} italic={true} />
        </div>
      </div>

      <div>
        <div class="t-meta" style="margin-bottom: 12px;">Full lockup — 3 sizes</div>
        <div style="display: flex; flex-direction: column; gap: 20px; align-items: flex-start;">
          <Logo size={20} />
          <Logo size={28} />
          <Logo size={48} />
        </div>
      </div>
    </div>
  </section>

  <!-- ─── 8. HEADER & FOOTER ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">HEADER & FOOTER</div>

    <div class="t-meta" style="margin-bottom: 12px;">
      Header — Gallery active (auth state reflects current session)
    </div>
    <div style="border: 1px solid var(--border-subtle); margin-bottom: 32px; overflow: hidden;">
      <AppHeader active="Gallery" />
    </div>

    <div class="t-meta" style="margin-bottom: 12px;">Footer</div>
    <div style="border: 1px solid var(--border-subtle); overflow: hidden;">
      <AppFooter />
    </div>
  </section>

  <!-- ─── 9. PHOTO PLACEHOLDER ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">
      PHOTO PLACEHOLDERS · deterministic gradient per target
    </div>
    <div
      style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 16px;"
    >
      {#each photoSamples as sample}
        <div>
          <Photo target={sample.target} style="height: 200px;" />
          <div class="t-meta" style="margin-top: 6px;">{sample.target} · {sample.name}</div>
        </div>
      {/each}
    </div>
  </section>

  <!-- ─── 10. CORNER MARKS ─── -->
  <section
    style="margin-bottom: 80px; border-top: 1px solid var(--border-subtle); padding-top: 48px;"
  >
    <div class="t-eyebrow" style="margin-bottom: 24px;">CORNER REGISTRATION MARKS</div>
    <div style="display: inline-block; position: relative;">
      <Photo target="IC 1805" style="width: 320px; height: 240px;" />
      <CornerMarks size={14} color="var(--accent)" inset={-8} />
    </div>
    <div class="t-meta" style="margin-top: 12px;">
      IC 1805 · Heart Nebula · 320×240 placeholder with 4 corner marks at −8px inset
    </div>
  </section>
</main>

<AppFooter />
