<script lang="ts">
  import {
    Settings,
    Sparkles,
    RefreshCw,
    Cloud,
    Server,
    KeyRound,
    CircleCheck,
    CircleAlert,
    Cpu,
    FileText,
    type Icon as IconType,
  } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import SectionCard from "$lib/components/SectionCard.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import LanguagePicker from "$lib/i18n/components/LanguagePicker.svelte";
  import PaperAppearance from "$lib/components/PaperAppearance.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import ProviderCard from "$lib/components/ProviderCard.svelte";
  import SyncSettings from "$lib/components/SyncSettings.svelte";
  import CalendarSettings from "$lib/components/CalendarSettings.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import { listProviders, enableProvider, testProvider } from "$lib/ipc";
  import { toast } from "svelte-sonner";
  import type { ProviderInfo } from "$lib/types/ai";
  import { m, localizeError } from "$lib/i18n";

  let providers = $state<ProviderInfo[]>([]);
  let ollamaUrl = $state("http://127.0.0.1:11434");
  let ollamaChat = $state("llama3.2");
  let ollamaEmbed = $state("nomic-embed-text");
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; detail?: string } | null>(null);

  async function refresh() {
    if (!backend.available) return;
    const r = await listProviders();
    if (r.ok) providers = r.value;
  }

  async function toggle(name: string, enabled: boolean) {
    const config =
      name === "ollama"
        ? JSON.stringify({
            base_url: ollamaUrl,
            chat_model: ollamaChat,
            embed_model: ollamaEmbed,
          })
        : null;
    const r = await enableProvider(name, enabled, config);
    if (!r.ok) {
      toast.error(localizeError({ code: r.code, message: r.error }));
      return;
    }
    await refresh();
    toast.success(
      enabled
        ? m.settings_provider_toggled_enabled({ name: cap(name) })
        : m.settings_provider_toggled_disabled({ name: cap(name) }),
    );
  }

  async function test() {
    testing = true;
    testResult = null;
    const r = await testProvider("ollama");
    testing = false;
    if (r.ok) {
      testResult = { ok: r.value.healthy, detail: r.value.detail ?? undefined };
      if (r.value.healthy) toast.success(m.settings_provider_ollama_reachable());
      else toast.error(m.settings_provider_ollama_unreachable(), {
        description: r.value.detail ?? undefined,
      });
    } else {
      testResult = {
        ok: false,
        detail: r.offline ? m.err_offline() : (r.error ?? m.settings_provider_test_failed()),
      };
    }
  }

  $effect(() => {
    refresh();
  });

  function providerIcon(name: string): typeof IconType {
    if (name === "ollama") return Server;
    return KeyRound;
  }

  function providerLabel(name: string): string {
    if (name === "ollama") return m.settings_provider_label_ollama();
    if (name === "openai") return m.settings_provider_label_openai();
    if (name === "groq") return m.settings_provider_label_groq();
    return name;
  }

  function cap(s: string): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function providerDescription(name: string): string {
    if (name === "ollama") return m.settings_provider_desc_ollama();
    if (name === "openai") return m.settings_provider_desc_openai();
    if (name === "groq") return m.settings_provider_desc_groq();
    return "";
  }
</script>

<PageHeader title={m.settings_page_title()} description={m.settings_page_description()}>
  {#snippet icon()}<Settings size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="warning"
      title={m.offline_settings_title()}
      description={m.offline_settings_description()}
    />
  {/if}

  <SectionCard title={m.settings_language_label()} description={m.settings_language_description()}>
    {#snippet icon()}<Settings size={14} />{/snippet}
    <LanguagePicker />
  </SectionCard>

  <SectionCard title={m.settings_paper_title()} description={m.settings_paper_description()}>
    {#snippet icon()}<FileText size={14} />{/snippet}
    <PaperAppearance />
  </SectionCard>

  <SectionCard title={m.settings_providers_title()} description={m.settings_providers_description()}>
    {#snippet icon()}<Cpu size={14} />{/snippet}
    {#if providers.length === 0}
      <p class="hint">{backend.available ? m.settings_loading() : m.settings_no_backend()}</p>
    {:else}
      <div class="providers">
        {#each providers as p (p.name)}
          {#if p.name === "ollama"}
            <article class="provider-card">
              <header>
                <span class="prov-icon"><Server size={14} /></span>
                <div class="prov-head-text">
                  <strong>{providerLabel(p.name)}</strong>
                  <small>{providerDescription(p.name)}</small>
                </div>
                <Badge
                  variant={p.healthy ? "default" : "outline"}
                  class={p.healthy ? "ok" : "offline"}
                >
                  {#if p.healthy}
                    <CircleCheck size={10} /> {m.settings_provider_status_reachable()}
                  {:else}
                    <CircleAlert size={10} /> {m.settings_provider_status_offline()}
                  {/if}
                </Badge>
              </header>
              {#if p.detail}<p class="detail">{p.detail}</p>{/if}

              <div class="field">
                <Label for="ollama-url">{m.settings_provider_ollama_url_label()}</Label>
                <Input id="ollama-url" bind:value={ollamaUrl} />
              </div>
              <div class="field-row">
                <div class="field">
                  <Label for="ollama-chat">{m.settings_provider_chat_model_label()}</Label>
                  <Input id="ollama-chat" bind:value={ollamaChat} />
                </div>
                <div class="field">
                  <Label for="ollama-embed">{m.settings_provider_embed_model_label()}</Label>
                  <Input id="ollama-embed" bind:value={ollamaEmbed} />
                </div>
              </div>

              <div class="row">
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={p.enabled}
                    onchange={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)}
                  />
                  <span class="toggle-track"><span class="toggle-thumb"></span></span>
                  <span class="toggle-label">{m.settings_provider_enabled_label()}</span>
                </label>
                <Button variant="outline" size="sm" onclick={test} disabled={testing || !backend.available}>
                  {#if testing}<RefreshCw size={12} class="animate-spin" /> {m.settings_provider_testing()}
                  {:else}
                    <RefreshCw size={12} /> {m.settings_provider_test_button()}
                  {/if}
                </Button>
                {#if testResult}
                  <span class="test" class:ok={testResult.ok}>
                    {#if testResult.ok}
                      <CircleCheck size={11} /> {m.settings_provider_test_ok()}
                    {:else}
                      <CircleAlert size={11} /> {testResult.detail ?? m.settings_provider_test_failed()}
                    {/if}
                  </span>
                {/if}
              </div>
            </article>
          {:else}
            <article class="provider-card">
              <header>
                <span class="prov-icon"><KeyRound size={14} /></span>
                <div class="prov-head-text">
                  <strong>{cap(p.name)}</strong>
                  <small>{providerDescription(p.name)}</small>
                </div>
                <Badge
                  variant={p.healthy ? "default" : "outline"}
                  class={p.healthy ? "ok" : "offline"}
                >
                  {#if p.healthy}
                    <CircleCheck size={10} /> {m.settings_provider_status_ready()}
                  {:else}
                    <CircleAlert size={10} /> {m.settings_provider_status_no_key()}
                  {/if}
                </Badge>
              </header>
              <ProviderCard provider={p} />
              <label class="toggle-row">
                <input
                  type="checkbox"
                  checked={p.enabled}
                  onchange={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)}
                />
                <span class="toggle-track"><span class="toggle-thumb"></span></span>
                <span class="toggle-label">{m.settings_provider_enabled_label()}</span>
              </label>
            </article>
          {/if}
        {/each}
      </div>
    {/if}
  </SectionCard>

  <SectionCard title={m.settings_transcription_title()} description={m.settings_transcription_description()}>
    {#snippet icon()}<Sparkles size={14} />{/snippet}
    <p class="hint">{m.settings_transcription_help()}</p>
  </SectionCard>

  <SectionCard title={m.settings_sync_title()} description={m.settings_sync_description()}>
    {#snippet icon()}<Cloud size={14} />{/snippet}
    <SyncSettings />
  </SectionCard>

  <SectionCard title={m.settings_calendar_title()} description={m.settings_calendar_description()}>
    <CalendarSettings />
  </SectionCard>
</div>

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 880px;
  }
  .providers {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .hint {
    margin: 0;
    color: var(--color-muted-foreground);
    font-size: 0.875rem;
    line-height: 1.55;
  }

  .provider-card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.875rem 1rem;
    background: var(--color-card);
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }
  .provider-card header {
    display: flex;
    gap: 0.625rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .prov-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .prov-head-text {
    flex: 1;
    min-width: 0;
  }
  .prov-head-text strong {
    display: block;
    font-size: 0.9375rem;
    color: var(--color-foreground);
  }
  .prov-head-text small {
    display: block;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
    margin-top: 2px;
  }
  .detail {
    margin: 0;
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
  }

  :global(.ok) {
    background: var(--color-success-subtle);
    color: var(--color-success);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.65rem;
  }
  :global(.offline) {
    color: var(--color-muted-foreground);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.65rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  @media (max-width: 600px) {
    .field-row {
      grid-template-columns: 1fr;
    }
  }

  .row {
    display: flex;
    gap: 0.625rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--color-foreground);
    cursor: pointer;
  }
  .toggle input,
  .toggle-row input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .toggle-track {
    position: relative;
    width: 32px;
    height: 18px;
    background: var(--color-muted);
    border-radius: 999px;
    transition: background 120ms ease;
    flex-shrink: 0;
    border: 1px solid var(--color-border);
  }
  .toggle-thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 14px;
    height: 14px;
    border-radius: 999px;
    background: var(--color-background);
    transition: transform 120ms ease;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .toggle input:checked + .toggle-track {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  .toggle input:checked + .toggle-track .toggle-thumb {
    transform: translateX(14px);
  }
  .toggle-label {
    user-select: none;
  }
  .toggle-row {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    margin: 0.25rem 0 0 0;
    color: var(--color-muted-foreground);
    cursor: pointer;
  }

  .test {
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .test.ok {
    color: var(--color-success);
  }
  .test:not(.ok) {
    color: var(--color-destructive);
  }

  :global(.animate-spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>