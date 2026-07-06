<script lang="ts">
  import { calendarAuthStatus, calendarConnect, calendarDisconnect } from '$lib/ipc';

  let status = $state<{ connected: boolean; expires_at?: number | null }>({ connected: false });
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function refresh() {
    try { status = await calendarAuthStatus(); } catch (e) { err = (e as Error).message; }
  }

  async function connect() {
    busy = true; err = null;
    try { await calendarConnect(); await refresh(); }
    catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  async function disconnect() {
    busy = true; err = null;
    try { await calendarDisconnect(); await refresh(); }
    catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  $effect(() => { refresh(); });
</script>

<section class="cal">
  <h2>Google Calendar</h2>
  <p class="muted">
    OAuth round-trip uses PKCE; no client_secret in the binary. Register your
    own OAuth client (type: Web application, redirect URI:
    <code>http://127.0.0.1:PORT/callback</code>) and edit
    <code>OAUTH_CLIENT_ID</code> in <code>oauth/google.rs</code> before building.
  </p>
  <div class="row">
    {#if status.connected}
      <span class="badge ok">connected</span>
      <button onclick={disconnect} disabled={busy}>Disconnect</button>
    {:else}
      <span class="badge err">not connected</span>
      <button onclick={connect} disabled={busy}>Connect…</button>
    {/if}
  </div>
  {#if err}<p class="err">{err}</p>{/if}
</section>

<style>
  .cal { margin: 1.5rem 0; }
  .muted { color: #666; font-size: .85em; margin-bottom: .5rem; }
  code { background: #f4f4f4; padding: 0 .3em; border-radius: 3px; font-size: .85em; }
  .row { display: flex; gap: .5rem; align-items: center; }
  button { padding: .35rem .7rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: .8em; }
  .badge.ok { background: #d3f3d3; color: #178217; }
  .badge.err { background: #f3d3d3; color: #c00; }
  .err { color: #c00; margin-top: .5rem; }
</style>