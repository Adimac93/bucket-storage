<script lang="ts">
    import { key } from "./stores";

    let fileId;
    
    async function deleteFile() {
        const encodedKey = btoa(`${$key.keyId}:${$key.key}`);
        const res = fetch(`/api/delete/${fileId}`,{
            headers: { Authorization: `Basic ${encodedKey}` },
        })
    }
</script>

<input type="text" bind:value={fileId}>
<button on:click={deleteFile}>Remove</button>