<script lang="ts">
  import { untrack } from 'svelte';
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import AutosaveField from '$lib/components/settings/AutosaveField.svelte';
  let { data } = $props();
  // untrack: read initial display_name once without subscribing to `data`.
  // `name` is an independently-editable local copy; the server refreshes it
  // on navigation after autosave commits.
  let name = $state(untrack(() => data.profile.display_name));
</script>

<Section title="Profile" description="How you appear under your photos.">
  <Row label="DISPLAY NAME">
    <AutosaveField bind:value={name} name="display_name" action="" />
  </Row>
</Section>
