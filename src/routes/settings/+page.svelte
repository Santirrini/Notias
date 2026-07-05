<script lang="ts">
  import { onMount } from 'svelte';
  import { listProviders, enableProvider, testProvider } from '$lib/ipc';
  import type { ProviderInfo } from '$lib/types/ai';

  let providers = $state<ProviderInfo[]>([]);
  let baseUrl = $state('http://127.0.0.1:11434');
  let chatModel = $state('llama3.2');
  let embedModel = $state('nomic-embed-text');
  let testing = $state(false);
  let testResult = $state<string | null>(null);

  async function refresh() {
    providers = await listProviders();
  }

  async function toggleOllama(enabled: boolean) {
    await enableProvider('ollama', enabled, JSON.stringify({ base_url: baseUrl, chat_model: chatModel, embed_model: embedModel }));
    await refresh();
  }

  async function saveConfig() {
    const ollama = providers.find((p) => p.name === 'ollama');
    await enableProvider('ollama', ollama?.enabled ?? false, JSON.stringify({ base_url: baseUrl, chat_model: chatModel, embed_model: embedModel }));
  }

  async function test() {
    testing = true;
    testResult = null;
    try {
      const r = await testProvider('ollama');
      testResult = r.healthy ? 'OK' : `Failed: ${r.detail ?? 'unknown'}`;
    } catch (e) {
      testResult = (e as Error).message;
    } finally {
      testing = false;
    }
  }

  onMount(refresh);
</script>

<h1>Settings — Providers</h1>

{#each providers as p (p.name)}
  <section>
    <h2>{p.name}</h2>
    <label>
      <input type="checkbox" checked={p.enabled}
        onchange={(e) => toggleOllama((e.target as HTMLInputElement).checked)} />
      Enabled
    </label>
    <p>Health: {p.healthy ? 'OK' : (p.detail ?? 'unknown')}</p>
    {#if p.name === 'ollama' && p.enabled}
      <label>Base URL <input bind:value={baseUrl} /></label>
      <label>Chat model <input bind:value={chatModel} /></label>
      <label>Embed model <input bind:value={embedModel} /></label>
      <button onclick={saveConfig}>Save</button>
      <button onclick={test} disabled={testing}>{testing ? 'Testing…' : 'Test connection'}</button>
      {#if testResult}<p>{testResult}</p>{/if}
    {/if}
  </section>
{/each}