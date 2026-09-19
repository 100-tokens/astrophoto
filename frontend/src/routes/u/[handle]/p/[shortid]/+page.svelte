<script lang="ts">
  import PhotoDetailFull from '$lib/components/photos/PhotoDetailFull.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
</script>

<PhotoDetailFull {data} />
