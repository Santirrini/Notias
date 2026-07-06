<script lang="ts">
  import { onMount } from 'svelte';
  import { listProviders, enableProvider, testProvider } from '$lib/ipc';
  import ProviderCard from '$lib/components/ProviderCard.svelte';
  import SyncSettings from '$lib/components/SyncSettings.svelte';
  import type { ProviderInfo } from '$lib/types/ai';

  let providers = $state<ProviderInfo[]>([]);
  let ollamaUrl = $state('http://127.0.0.1:11434');
  let ollamaChat = $state('llama3.2');
  let ollamaEmbed = $state('nomic-embed-text');
  let testing = $state(false);
  let testResult = $state<string | null>(null);

  async function refresh() {
    providers = await listProviders();
    const o = providers.find((p) => p.name === 'ollama');
    if (o) {
      // ponytail: settings read from config_json later; keep simple defaults
    }
  }

  async function toggle(name: string, enabled: boolean) {
    if (name === 'ollama') {
      await enableProvider(name, enabled, JSON.stringify({
        base_url: ollamaUrl, chat_model: ollamaChat, embed_model: ollamaEmbed,
      }));
    } else {
      await enableProvider(name, enabled, null);
    }
    await refresh();
  }

  async function test() {
    testing = true; testResult = null;
    try {
      const r = await testProvider('ollama');
      testResult = r.healthy ? 'OK' : `Failed: ${r.detail ?? 'unknown'}`;
    } catch (e) {
      testResult = (e as Error).message;
    } finally { testing = false; }
  }

  onMount(refresh);
</script>

<h1>Providers</h1>
<p class="muted">Local-first by default. Ollama runs offline; cloud keys are stored in OS keyring.</p>

<section>
  <h2>Chat providers</h2>
  {#if providers.length === 0}<p>Loading…</p>{/if}
  {#each providers as p (p.name)}
    {#if p.name === 'ollama'}
      <article class="card ollama">
        <header>
          <strong>{p.name}</strong>
          <span class="badge" class:ok={p.healthy} class:err={!p.healthy}>
            {p.healthy ? 'reachable' : 'offline'}
          </span>
        </header>
        {#if p.detail}<p class="detail">{p.detail}</p>{/if}
        <label class="row"><span>Base URL</span><input bind:value={ollamaUrl} /></label>
        <label class="row"><span>Chat model</span><input bind:value={ollamaChat} /></label>
        <label class="row"><span>Embed model</span><input bind:value={ollamaEmbed} /></label>
        <div class="row">
          <label class="toggle">
            <input type="checkbox" checked={p.enabled}
              onchange={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)} />
            enabled
          </label>
          <button onclick={test} disabled={testing}>{testing ? 'Testing…' : 'Test'}</button>
          {#if testResult}<span class="test">{testResult}</span>{/if}
        </div>
      </article>
    {:else}
      <ProviderCard provider={p} />
      <label class="toggle-row">
        <input type="checkbox" checked={p.enabled}
          onchange={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)} />
        enabled
      </label>
    {/if}
  {/each}
</section>

<section>
  <h2>Transcription</h2>
  <p class="muted">Whisper runs via Groq by default. Set a Groq API key above to enable.</p>
</section>

<SyncSettings />

<style>
  h1 { margin-top: 1rem; }
  .muted { color: #666; font-size: .9em; }
  section { margin: 1.5rem 0; }
  .card { border: 1px solid #ddd; border-radius: 6px; padding: 1rem; margin-bottom: .75rem; }
  .card.ollama { background: #fafafa; }
  header { display: flex; gap: .75rem; align-items: center; margin-bottom: .5rem; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: .8em; }
  .badge.ok { background: #d3f3d3; color: #178217; }
  .badge.err { background: #f3d3d3; color: #c00; }
  .row { display: flex; gap: .5rem; margin-top: .5rem; align-items: center; flex-wrap: wrap; }
  .toggle, .toggle-row { display: inline-flex; gap: .35rem; align-items: center; }
  .toggle-row { margin: .25rem 0 .75rem .5rem; }
  label.row span { min-width: 110px; color: #555; }
  input:not([type]) { padding: .35rem; font-family: ui-monospace, monospace; min-width: 240px; }
  .detail { color: #666; font-size: .85em; margin: .25rem 0; }
  .test { color: #666; font-size: .85em; }
</style>
