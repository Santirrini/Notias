<script lang="ts">
  /**
   * Global client-error overlay.
   *
   * Subscribes to the `notias:error` CustomEvent dispatched by
   * `src/hooks.client.ts`. Every unhandled hydration / runtime failure lands
   * here so the user always sees a breadcrumb instead of a silent blank page.
   *
   * Only renders in dev or when `?debug=1` is present in the URL. Production
   * keeps relying on the SvelteKit default + console.error so we don't leak
   * stack traces to end users.
   */
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { AlertTriangle, Copy, Check, X } from "@lucide/svelte";
  import { m } from "$lib/i18n";

  interface ErrorDetail {
    message: string;
    stack?: string;
    pathname?: string;
  }

  let entry = $state<ErrorDetail | null>(null);
  let copied = $state(false);

  const visible = $derived.by(() => {
    if (!entry) return false;
    // Gate: dev OR explicit debug query flag.
    if (import.meta.env.DEV) return true;
    return page.url?.searchParams.get("debug") === "1";
  });

  onMount(() => {
    function onError(ev: Event) {
      const ce = ev as CustomEvent<ErrorDetail>;
      entry = ce.detail ?? { message: m.overlay_error_unknown() };
      copied = false;
    }
    window.addEventListener("notias:error", onError);
    return () => window.removeEventListener("notias:error", onError);
  });

  async function copyStack() {
    if (!entry) return;
    const text = [entry.message, entry.stack ?? m.overlay_error_no_stack()].join("\n\n");
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      /* clipboard may be blocked; ignore */
    }
  }

  function dismiss() {
    entry = null;
  }
</script>

{#if visible}
  <aside class="err-overlay" role="alert" aria-live="assertive">
    <header>
      <span class="icon"><AlertTriangle size={14} /></span>
      <strong>{m.overlay_error_title()}</strong>
      <button type="button" class="close" onclick={dismiss} aria-label={m.overlay_error_dismiss_aria()}>
        <X size={14} />
      </button>
    </header>
    <p class="msg">{entry?.message}</p>
    {#if entry?.pathname}
      <p class="ctx">{m.overlay_error_at_pathname()} <code>{entry.pathname}</code></p>
    {/if}
    {#if entry?.stack}
      <pre class="stack">{entry.stack}</pre>
    {/if}
    <footer>
      <button type="button" class="copy" onclick={copyStack}>
        {#if copied}<Check size={12} /> {m.overlay_error_copied()}{:else}<Copy size={12} /> {m.overlay_error_copy()}{/if}
      </button>
    </footer>
  </aside>
{/if}

<style>
  .err-overlay {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 1000;
    max-width: 420px;
    width: calc(100vw - 2rem);
    background: var(--color-card);
    border: 1px solid color-mix(in srgb, var(--color-destructive) 45%, var(--color-border));
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    color: var(--color-foreground);
    font-size: 0.8125rem;
    overflow: hidden;
    animation: slide-in 160ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  @keyframes slide-in {
    from { transform: translateY(-8px); opacity: 0; }
    to   { transform: translateY(0); opacity: 1; }
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: color-mix(in srgb, var(--color-destructive) 12%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--color-destructive) 25%, transparent);
  }
  header strong {
    flex: 1;
    color: var(--color-destructive);
    font-weight: 600;
  }
  .icon {
    display: inline-flex;
    color: var(--color-destructive);
  }
  .close {
    background: transparent;
    border: 0;
    padding: 0.125rem;
    color: var(--color-muted-foreground);
    border-radius: 3px;
    cursor: pointer;
    display: inline-flex;
  }
  .close:hover { background: rgba(0, 0, 0, 0.05); color: var(--color-foreground); }
  .msg {
    margin: 0.625rem 0.75rem 0;
    color: var(--color-foreground);
    font-weight: 500;
  }
  .ctx {
    margin: 0.25rem 0.75rem 0;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
  }
  .ctx code {
    font-family: var(--font-mono);
    font-size: 0.7rem;
  }
  .stack {
    margin: 0.5rem 0.75rem;
    padding: 0.5rem;
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    line-height: 1.45;
    max-height: 160px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--color-foreground);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    padding: 0.5rem 0.75rem;
    border-top: 1px solid var(--color-border);
    background: var(--color-muted);
  }
  .copy {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: transparent;
    border: 1px solid var(--color-border);
    color: var(--color-foreground);
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.75rem;
  }
  .copy:hover { background: var(--color-card); }
</style>