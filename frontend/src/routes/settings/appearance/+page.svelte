<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  let { data } = $props();
</script>

<Section title="Appearance" description="The interface adapts to dark-adapted eyes by default.">
  <Row label="THEME">
    <form method="POST" action="?/setTheme" class="chip-group">
      <button name="theme" value="dark" class:active={data.preferences.theme === 'dark'}
        >DARK</button
      >
      <button name="theme" value="light" class:active={data.preferences.theme === 'light'}
        >LIGHT</button
      >
    </form>
  </Row>
  <Row label="DENSITY">
    <form method="POST" action="?/setDensity" class="chip-group">
      <button name="density" value="work" class:active={data.preferences.density === 'work'}
        >WORK</button
      >
      <button name="density" value="data" class:active={data.preferences.density === 'data'}
        >DATA</button
      >
    </form>
  </Row>
</Section>

<style>
  .chip-group {
    display: flex;
    gap: 8px;
  }
  .chip-group button {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.1em;
    padding: 6px 12px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--fg-muted);
    cursor: pointer;
  }
  .chip-group button.active {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--bg-accent-tint);
  }
</style>
