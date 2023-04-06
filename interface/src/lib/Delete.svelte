<script lang="ts">
    import { fetchAuthorized } from "./api";

    let fileId;
    
    async function deleteFile() {
        const res = await fetchAuthorized(`/api/delete/${fileId}`)
    }
</script>

<div>
    <input type="text" bind:value={fileId} placeholder="file ID"/>
    <button on:click={deleteFile}>Remove</button>
</div>
