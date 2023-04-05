<script lang="ts">
  import { key, type AuthKey } from "./stores";

  async function gen_key() {
    const res = await fetch("/api/key");
    key.set((await res.json()) as AuthKey);
  }
</script>

<button on:click={gen_key}>Issue</button>
{#if $key}
  <p>Key ID: {$key.keyId}</p>
  <p>Key: {$key.key}</p>
{/if}
