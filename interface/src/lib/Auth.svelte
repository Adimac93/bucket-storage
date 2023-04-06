<script lang="ts">
    import { key } from "./stores";

    interface AuthKey {
        keyId: string,
        key: string
    }

    async function gen_key() {
        const res = await fetch("/api/key");
        const authKey = await res.json() as AuthKey;
        const encodedKey = btoa(`${authKey.keyId}:${authKey.key}`)
        key.set(encodedKey);
    }
</script>

<div>
    <button on:click={gen_key}>Issue</button>
    {#if $key}
        <p>Encoded key: {($key.slice(0,6))}...</p>
    {/if}
</div>
