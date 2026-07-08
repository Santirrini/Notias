<script lang="ts">
  import { Plug, PlugZap, AlertCircle } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { backend } from "$lib/stores/backend.svelte";
  import { calendarAuthStatus, calendarConnect, calendarDisconnect } from "$lib/ipc";

  let status = $state<{ connected: boolean; expires_at?: number | null }>({ connected: false });
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function refresh() {
    if (!backend.available) return;
    try {
      status = await calendarAuthStatus();
      err = null;
    } catch (e) {
      err = (e as Error).message;
    }
  }

  async function connect() {
    busy = true;
    err = null;
    try {
      await calendarConnect();
      await refresh();
    } catch (e) {
      err = (e as Error).message;
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    busy = true;
    err = null;
    try {
      await calendarDisconnect();
      await refresh();
    } catch (e) {
      err = (e as Error).message;
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    refresh();
  });
</script>

<div class="cal">
  <p class="muted">
    OAuth round-trip uses PKCE; no <code>client_secret</code> in the binary. Register your
    own OAuth client (type: Web application, redirect URI:
    <code>http://127.0.0.1:PORT/callback</code>) and edit
    <code>OAUTH_CLIENT_ID</code> in <code>oauth/google.rs</code> before building.
  </p>

  <div class="row">
    {#if !backend.available}
      <Badge variant="outline" class="offline">offline</Badge>
    {:else if status.connected}
      <Badge class="ok">connected</Badge>
      <Button variant="outline" onclick={disconnect} disabled={busy}>
        <PlugZap size={14} /> Disconnect
      </Button>
    {:else}
      <Badge variant="outline" class="offline">not connected</Badge>
      <Button onclick={connect} disabled={busy}>
        <Plug size={14} /> Connect…
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
  code {
    background: var(--color-muted);
    padding: 0.0625rem 0.375rem;
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
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
