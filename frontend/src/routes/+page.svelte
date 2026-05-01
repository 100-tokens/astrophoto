<script lang="ts">
  let { data } = $props();
  let healthLine = $derived(
    data.health ? `backend: ${data.health.status} (db ${data.health.db})` : 'backend: unreachable'
  );
</script>

<h1>Astrophoto</h1>
<p>Upload, tag, and share your astrophotographs.</p>

<p class="health">{healthLine}</p>

<style>
  h1 {
    margin-top: 4rem;
    font-size: clamp(2rem, 6vw, 3.5rem);
  }
  .health {
    font-family: ui-monospace, monospace;
    color: #888;
    margin-top: 2rem;
  }
</style>
