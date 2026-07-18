<script lang="ts">
  import { Eye, EyeOff, Trash2 } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { backend } from "$lib/stores/backend.svelte";
  import { setProviderKey, deleteProviderKey, hasProviderKey } from "$lib/ipc";
  import type { ProviderInfo } from "$lib/types/ai";
  import { m, localizeError } from "$lib/i18n";

  let { provider }: { provider: ProviderInfo } = $props();

  let keyInput = $state("");
  let showKey = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let hasKey = $state(false);

  $effect(() => {
    if (!backend.available) return;
    hasProviderKey(provider.name).then((r) => {
      if (r.ok) hasKey = r.value;
    });
  });

  async function save() {
    if (!keyInput) return;
    saving = true;
    error = null;
    const r = await setProviderKey(provider.name, keyInput);
    saving = false;
    if (r.ok) {
      hasKey = true;
      keyInput = "";
      return;
    }
    error = localizeError({ code: r.code, message: r.error });
  }

  async function forget() {
    if (!confirm(m.provider_key_remove_confirm({ name: provider.name }))) return;
    const r = await deleteProviderKey(provider.name);
    if (r.ok) {
      hasKey = false;
    } else {
      error = localizeError({ code: r.code, message: r.error });
    }
  }
</script>

<article class="provider-card">
  <header>
    <strong>{provider.name}</strong>
    <Badge variant={provider.healthy ? "default" : "outline"}>
      {provider.healthy ? m.provider_key_reachable() : m.provider_key_offline()}
    </Badge>
    <Badge variant={hasKey ? "default" : "outline"} class={hasKey ? "key-on" : "key-off"}>
      {hasKey ? m.provider_key_set() : m.provider_key_none()}
    </Badge>
  </header>

  {#if provider.detail}
    <p class="detail">{provider.detail}</p>
  {/if}

  <div class="row">
    <Input
      type={showKey ? "text" : "password"}
      placeholder={provider.name === "openai"
        ? m.provider_key_placeholder_openai()
        : m.provider_key_placeholder_groq()}
      bind:value={keyInput}
      autocomplete="off"
      class="mono-input"
    />
    <Button variant="ghost" size="icon" onclick={() => (showKey = !showKey)} aria-label={m.provider_key_toggle_visibility_aria()}>
      {#if showKey}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
    </Button>
  </div>

  <div class="row">
    <Button onclick={save} disabled={saving || !keyInput || !backend.available}>
      {saving ? m.provider_key_saving() : m.provider_key_save()}
    </Button>
    {#if hasKey}
      <Button variant="destructive" size="sm" onclick={forget} disabled={!backend.available}>
        <Trash2 size={12} /> {m.provider_key_forget()}
      </Button>
    {/if}
  </div>

  {#if error}
    <p class="err-msg">{error}</p>
  {/if}
</article>

<style>
  .provider-card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.875rem 1rem;
    background: var(--color-card);
    margin-bottom: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  header {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .detail {
    margin: 0;
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  :global(.mono-input) {
    flex: 1;
    min-width: 200px;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .err-msg {
    color: var(--color-destructive);
    font-size: 0.8125rem;
    margin: 0;
  }
  :global(.key-on) {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  :global(.key-off) {
    color: var(--color-muted-foreground);
  }
</style>
