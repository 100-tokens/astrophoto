<script lang="ts">
  import '../app.css';

  let { children, data } = $props();
</script>

<header>
  <a href="/">Astrophoto</a>
  {#if data.user}
    <span>Hi, {data.user.displayName}</span>
  {/if}
</header>

<main>
  {@render children()}
</main>

<style>
  header {
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  main {
    padding: 0 2rem 4rem;
    max-width: 64rem;
    margin: 0 auto;
  }
</style>
