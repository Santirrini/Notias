<script lang="ts">
  import { onMount } from "svelte";
  import {
    Home,
    CheckCircle2,
    AlertCircle,
    RefreshCw,
    NotebookText,
    MessageSquare,
    ListTodo,
    Calendar,
    GraduationCap,
    Sparkles,
    Settings as SettingsIcon,
    ArrowRight,
    BookOpen,
    Cloud,
    Zap,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import SectionCard from "$lib/components/SectionCard.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import { ping } from "$lib/ipc";
  import { goto } from "$app/navigation";
  import { m, localizeError } from "$lib/i18n";

  let result = $state<string | null>(null);
  let error = $state<string | null>(null);
  let checking = $state(false);

  async function check() {
    checking = true;
    error = null;
    const r = await ping();
    checking = false;
    if (r.ok) {
      result = r.value;
      return;
    }
    if (r.offline) {
      error = m.err_offline();
    } else {
      error = localizeError({ code: r.code, message: r.error });
    }
  }

  onMount(() => {
    if (backend.available) check();
  });

  const shortcuts = $derived([
    {
      href: "/notes",
      kbd: ["Ctrl", "N"],
      icon: NotebookText,
      title: m.home_shortcut_new_title(),
      hint: m.home_shortcut_new_hint(),
    },
    {
      href: "/chat",
      kbd: ["Ctrl", "K"],
      icon: MessageSquare,
      title: m.home_shortcut_palette_title(),
      hint: m.home_shortcut_palette_hint(),
    },
    {
      href: "/study",
      kbd: ["Ctrl", "Shift", "L"],
      icon: GraduationCap,
      title: m.home_shortcut_theme_title(),
      hint: m.home_shortcut_theme_hint(),
    },
    {
      href: "/settings",
      kbd: ["Ctrl", ","],
      icon: SettingsIcon,
      title: m.home_shortcut_settings_title(),
      hint: m.home_shortcut_settings_hint(),
    },
  ]);

  const features = $derived([
    {
      icon: BookOpen,
      title: m.home_feature_local_title(),
      body: m.home_feature_local_body(),
    },
    {
      icon: Sparkles,
      title: m.home_feature_ai_title(),
      body: m.home_feature_ai_body(),
    },
    {
      icon: ListTodo,
      title: m.home_feature_tasks_title(),
      body: m.home_feature_tasks_body(),
    },
    {
      icon: Calendar,
      title: m.home_feature_calendar_title(),
      body: m.home_feature_calendar_body(),
    },
    {
      icon: Cloud,
      title: m.home_feature_sync_title(),
      body: m.home_feature_sync_body(),
    },
    {
      icon: Zap,
      title: m.home_feature_slash_title(),
      body: m.home_feature_slash_body(),
    },
  ]);
</script>

<PageHeader title={m.home_page_intro_title()} description={m.home_page_intro_description()}>
  {#snippet icon()}<Home size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="info"
      title={m.offline_home_title()}
      description={m.offline_home_description()}
      dismissable={true}
    />
  {/if}

  <SectionCard title={m.home_card_status_title()} description={m.home_card_status_desc()}>
    {#snippet icon()}<Cloud size={14} />{/snippet}
    <div class="status-row">
      {#if result}
        <Badge class="ok"><CheckCircle2 size={12} /> {m.home_status_reachable()}</Badge>
        <code class="pong">{result}</code>
      {:else if error}
        <Badge variant="destructive">
          <AlertCircle size={12} /> {m.home_status_unreachable()}
        </Badge>
        <code class="err">{error}</code>
      {:else}
        <Badge variant="outline">{@render LoaderDot()} {m.home_checking()}</Badge>
      {/if}
      <Button variant="outline" size="sm" onclick={check} disabled={checking}>
        <RefreshCw size={12} class={checking ? "animate-spin" : ""} />
        {checking ? m.home_pinging() : m.home_check_again()}
      </Button>
    </div>
  </SectionCard>

  <SectionCard title={m.home_card_quick_title()} description={m.home_card_quick_desc()}>
    {#snippet icon()}<Zap size={14} />{/snippet}
    <div class="shortcuts">
      {#each shortcuts as s}
        <button type="button" class="shortcut" onclick={() => goto(s.href)}>
          <span class="shortcut-icon">
            <s.icon size={18} />
          </span>
          <div class="shortcut-body">
            <strong>{s.title}</strong>
            <span class="hint">{s.hint}</span>
          </div>
          <span class="shortcut-kbd">
            {#each s.kbd as k, i}<kbd>{k}</kbd>{#if i < s.kbd.length - 1}<span class="plus">+</span>{/if}{/each}
          </span>
          <ArrowRight size={14} class="arrow" />
        </button>
      {/each}
    </div>
  </SectionCard>

  <SectionCard title={m.home_card_features_title()} description={m.home_card_features_desc()}>
    <div class="features">
      {#each features as f}
        <article class="feature">
          <span class="feature-icon"><f.icon size={16} /></span>
          <div>
            <strong>{f.title}</strong>
            <p>{f.body}</p>
          </div>
        </article>
      {/each}
    </div>
  </SectionCard>
</div>

{#snippet LoaderDot()}
  <span class="dot-loader" aria-hidden="true"></span>
{/snippet}

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    max-width: 980px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    flex-wrap: wrap;
  }
  .pong {
    background: var(--color-success-subtle);
    color: var(--color-success);
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .err {
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  :global(.ok) {
    background: var(--color-success-subtle);
    color: var(--color-success);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  .dot-loader {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    border: 2px solid var(--color-muted);
    border-top-color: var(--color-accent);
    animation: spin 0.7s linear infinite;
    margin-right: 4px;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  :global(.animate-spin) {
    animation: spin 1s linear infinite;
  }

  .shortcuts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 0.625rem;
  }
  .shortcut {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-background);
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    color: inherit;
    transition: border-color 120ms ease, transform 100ms ease, box-shadow 120ms ease, background 120ms ease;
  }
  .shortcut:hover {
    border-color: var(--color-accent);
    background: var(--color-accent-subtle);
  }
  .shortcut:hover :global(.arrow) {
    opacity: 1;
    transform: translateX(0);
    color: var(--color-accent);
  }
  .shortcut:active {
    transform: scale(0.99);
  }
  .shortcut-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .shortcut-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .shortcut-body strong {
    font-size: 0.9rem;
    color: var(--color-foreground);
  }
  .hint {
    font-size: 0.75rem;
    color: var(--color-muted-foreground);
  }
  .shortcut-kbd {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 0.65rem;
    flex-shrink: 0;
  }
  .shortcut-kbd kbd {
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
  }
  .shortcut-kbd .plus {
    color: var(--color-muted-foreground);
    font-size: 0.65rem;
  }
  :global(.shortcut .arrow) {
    color: var(--color-muted-foreground);
    opacity: 0;
    transform: translateX(-4px);
    transition: opacity 100ms ease, transform 100ms ease, color 100ms ease;
    flex-shrink: 0;
  }

  .features {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 0.75rem;
  }
  .feature {
    display: flex;
    gap: 0.625rem;
    padding: 0.75rem;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .feature:hover {
    border-color: var(--color-border);
    background: var(--color-muted);
  }
  .feature-icon {
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
  .feature strong {
    display: block;
    font-size: 0.875rem;
    color: var(--color-foreground);
  }
  .feature p {
    margin: 0.25rem 0 0;
    font-size: 0.75rem;
    color: var(--color-muted-foreground);
    line-height: 1.5;
  }
</style>