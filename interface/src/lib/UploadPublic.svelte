<script lang="ts">
    import { fetchAuthorized } from "./api";

    let files: FileList;

    async function getUploadUrl() {
        const res = await fetchAuthorized("/api/upload/key");
        const json = await res.json();
        if (res.ok) {
            await upload(json.uploadId);
        } else {
            console.error(json.errorInfo);
        }
        
    }

    async function upload(key: string) {
        const formData = new FormData();
        const filesArr = Array.from(files);
        filesArr.forEach(file => {
            formData.append("file", file);
        })

        const res = await fetch(`/api/upload/${key}`, {
            method: "POST",
            body: formData
        });

    }
</script>

<div>
    <p>Upload with public API call</p>
    <button on:click={getUploadUrl} disabled={files === undefined}>Upload</button>
    <input type="file" bind:files />
</div>
