<script lang="ts">
  import { setProviderKey, deleteProviderKey, hasProviderKey } from '$lib/ipc';
  import type { ProviderInfo } from '$lib/types/ai';

  let { provider }: { provider: ProviderInfo } = $props();

  let keyInput = $state('');
  let showKey = $state(false);
  let saving = $state(false);
  let error: string | null = $state(null);
  let hasKey = $state(false);

  $effect(() => {
    hasProviderKey(provider.name).then((v) => { hasKey = v; });
  });

  async function save() {
    if (!keyInput) return;
    saving = true; error = null;
    try {
      await setProviderKey(provider.name, keyInput);
      hasKey = true;
      keyInput = '';
    } catch (e) {
      error = (e as { message: string }).message;
    } finally { saving = false; }
  }

  async function forget() {
    if (!confirm(`Remove API key for ${provider.name}?`)) return;
    await deleteProviderKey(provider.name);
    hasKey = false;
  }
</script>

<article class="card">
  <header>
    <strong>{provider.name}</strong>
    <span class="badge" class:ok={provider.healthy} class:err={!provider.healthy}>
      {provider.healthy ? 'reachable' : 'offline'}
    </span>
    {#if hasKey}<span class="key-on">key set</span>
    {:else}<span class="key-off">no key</span>{/if}
  </header>
  {#if provider.detail}<p class="detail">{provider.detail}</p>{/if}

  <div class="row">
    <input
      type={showKey ? 'text' : 'password'}
      placeholder={provider.name === 'openai' ? 'sk-...' : 'gsk-...'}
      bind:value={keyInput}
      autocomplete="off"
    />
    <button onclick={() => showKey = !showKey}>{showKey ? 'hide' : 'show'}</button>
  </div>
  <div class="row">
    <button onclick={save} disabled={saving || !keyInput}>Save key</button>
    {#if hasKey}<button onclick={forget} class="forget">Forget</button>{/if}
    {#if error}<span class="err">{error}</span>{/if}
  </div>
</article>

<style>
  .card { border: 1px solid #ddd; border-radius: 6px; padding: 1rem; margin-bottom: .75rem; }
  header { display: flex; gap: .75rem; align-items: center; margin-bottom: .5rem; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: .8em; }
  .badge.ok { background: #d3f3d3; color: #178217; }
  .badge.err { background: #f3d3d3; color: #c00; }
  .key-on { color: #178217; font-size: .85em; }
  .key-off { color: #888; font-size: .85em; }
  .detail { color: #666; font-size: .85em; margin: .25rem 0; }
  .row { display: flex; gap: .5rem; margin-top: .5rem; align-items: center; flex-wrap: wrap; }
  input { flex: 1; min-width: 200px; padding: .35rem; font-family: ui-monospace, monospace; }
  .forget { background: #fee; color: #c00; }
  .err { color: #c00; font-size: .85em; }
</style>
