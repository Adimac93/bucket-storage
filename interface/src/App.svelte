<script lang="ts">
  import Auth from "./lib/Auth.svelte";
  import Download from "./lib/Download.svelte";
  import Upload from "./lib/Upload.svelte";
  import { key } from "./lib/stores";
</script>

<main>
  <Auth />
  {#if $key}
    <Upload />
    <Download />
  {/if}
</main>
