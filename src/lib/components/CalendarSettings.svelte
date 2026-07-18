<script lang="ts">
  import { Plug, PlugZap, AlertCircle } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { backend } from "$lib/stores/backend.svelte";
  import { calendarAuthStatus, calendarConnect, calendarDisconnect } from "$lib/ipc";
  import { m, localizeError } from "$lib/i18n";

  let status = $state<{ connected: boolean; expires_at?: number | null }>({ connected: false });
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function refresh() {
    if (!backend.available) return;
    const r = await calendarAuthStatus();
    if (r.ok) {
      status = r.value;
      err = null;
    } else if (!r.offline) {
      err = localizeError({ code: r.code, message: r.error });
    }
  }

  async function connect() {
    busy = true;
    err = null;
    const r = await calendarConnect();
    busy = false;
    if (r.ok) {
      await refresh();
      return;
    }
    err = localizeError({ code: r.code, message: r.error });
  }

  async function disconnect() {
    busy = true;
    err = null;
    const r = await calendarDisconnect();
    busy = false;
    if (r.ok) {
      await refresh();
      return;
    }
    err = localizeError({ code: r.code, message: r.error });
  }

  $effect(() => {
    refresh();
  });
</script>

<div class="cal">
  <p class="muted">
    {@html m.calendar_help_paragraph({
      code: "<code>client_secret</code>",
      redirect: "<code>http://127.0.0.1:PORT/callback</code>",
      code_id: "<code>OAUTH_CLIENT_ID</code>",
      code_oauth: "<code>oauth/google.rs</code>",
    })}
  </p>

  <div class="row">
    {#if !backend.available}
      <Badge variant="outline" class="offline">{m.calendar_offline()}</Badge>
    {:else if status.connected}
      <Badge class="ok">{m.calendar_connected()}</Badge>
      <Button variant="outline" onclick={disconnect} disabled={busy}>
        <PlugZap size={14} /> {m.calendar_disconnect()}
      </Button>
    {:else}
      <Badge variant="outline" class="offline">{m.calendar_not_connected()}</Badge>
      <Button onclick={connect} disabled={busy}>
        <Plug size={14} /> {m.calendar_connect()}
      </Button>
    {/if}
  </div>

  {#if err}
    <p class="err-msg"><AlertCircle size={12} /> {err}</p>
  {/if}
</div>

<style>
  .muted {
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
    margin: 0 0 0.625rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  :global(.ok) {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  :global(.offline) {
    color: var(--color-muted-foreground);
  }
  .err-msg {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0.625rem 0 0;
    font-size: 0.8125rem;
    padding: 0.375rem 0.625rem;
    border-radius: var(--radius-sm);
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
  }
</style>
