<script lang="ts">
    import Auth from "./lib/Auth.svelte";
    import Delete from "./lib/Delete.svelte";
    import Download from "./lib/Download.svelte";
    import Upload from "./lib/Upload.svelte";
    import UploadPublic from "./lib/UploadPublic.svelte";
    import { key } from "./lib/stores";
</script>

<main>
    <Auth />
    {#if $key}
        <Upload />
        <UploadPublic/>
        <Download />
        <Delete />
    {/if}
</main>
