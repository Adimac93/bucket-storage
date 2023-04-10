<script lang="ts">
  import { fetchAuthorized } from "./api";

  let loading = false;
  let image;
  let fileId;
  let prompt;

  async function download() {
    const res = await fetchAuthorized(`/api/download/${fileId}`);
    try {
      const contentType = res.headers.get("content-type");
      console.debug(contentType);
      if (contentType === "image/png" || contentType === "image/jpg") {
        loading = true;
        image = URL.createObjectURL(await res.blob());
        loading = false;
      } else {
        prompt = "Could not render file";
        setTimeout(() => {
          prompt = null;
        }, 3000);
      }
    } catch (e) {
      prompt = "Could not determine file type";
      setTimeout(() => {
        prompt = null;
      }, 3000);
    }
  }
</script>

<div>
  <input type="text" bind:value={fileId} placeholder="file ID" />
  <button on:click={download}>Download</button>
  {#if loading}
    <p>Loading image</p>
  {/if}
  {#if image}
    <img
      src={image}
      alt="someting"
      height="auto"
      width={window.innerWidth / 2}
    />
  {/if}
  {#if prompt}
    <p>{prompt}</p>
  {/if}
</div>
