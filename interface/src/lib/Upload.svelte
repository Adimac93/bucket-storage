<script lang="ts">
  import { key } from "./stores";

  let files;

  async function upload() {
    const formData = new FormData();
    formData.append("file", files[0]);
    const encodedKey = btoa(`${$key.keyId}:${$key.key}`);
    const res = await fetch("/api/upload", {
      method: "POST",
      headers: { Authorization: `Basic ${encodedKey}` },
      body: formData,
    });
  }
</script>

<button on:click={upload}>Upload</button>
<input type="file" bind:files />
