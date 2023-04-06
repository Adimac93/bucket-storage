<script lang="ts">
    import { fetchAuthorized } from "./api";

    let files: FileList;

    async function upload() {
        const formData = new FormData();
        const filesArr = Array.from(files);
        filesArr.forEach(file => {
            formData.append("file", file);
        })
        
        const res = await fetchAuthorized("/api/upload", {
            method: "POST",
            body: formData,
        });
    }
</script>

<div>
    <p>Upload with private API call</p>
    <button on:click={upload} disabled={files === undefined}>Upload</button>
    <input type="file" bind:files />
</div>
