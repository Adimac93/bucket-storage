<script lang="ts">
  import { key } from "./stores";

  let image;
  let fileId;
  async function download() {
    const encodedKey = btoa(`${$key.keyId}:${$key.key}`);
    const res = await fetch(`/api/download/${fileId}`, {
      headers: { Authorization: `Basic ${encodedKey}` },
    });

    image = URL.createObjectURL(await res.blob());
    image;
  }
</script>

<input type="text" bind:value={fileId} />
<button on:click={download}>Download</button>
{#if image}
  <img src={image} alt="someting" height="auto" width={window.innerWidth / 2} />
{/if}
