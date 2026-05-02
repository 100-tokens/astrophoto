<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import AutosaveField from '$lib/components/settings/AutosaveField.svelte';
  let { data } = $props();
  let name = $state(data.profile.display_name);
</script>

<Section title="Profile" description="How you appear under your photos.">
  <Row label="DISPLAY NAME">
    <AutosaveField bind:value={name} name="display_name" action="?/default" />
  </Row>
</Section>
