<script lang="ts">
  import { Download, Upload, RefreshCw, AlertCircle, Check } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { backend } from "$lib/stores/backend.svelte";
  import { syncExportZip, syncImportZip, syncRebuildNow } from "$lib/ipc";
  import { m, localizeError, formatBytes } from "$lib/i18n";

  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);
  let importInput: HTMLInputElement | undefined;

  async function exportNow() {
    if (!backend.available) return;
    busy = true;
    err = null;
    status = null;
    const r = await syncExportZip();
    busy = false;
    if (!r.ok) {
      err = localizeError({ code: r.code, message: r.error });
      return;
    }
    const bytes = r.value;
    const blob = new Blob([new Uint8Array(bytes)], { type: "application/zip" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `notias-export-${new Date().toISOString().slice(0, 10)}.zip`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
    status = m.sync_status_exported({ count: formatBytes(bytes.length) });
  }

  async function importNow(ev: Event) {
    const input = ev.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    busy = true;
    err = null;
    status = null;
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      const r = await syncImportZip(bytes);
      if (r.ok) {
        status = m.sync_status_imported({ count: r.value });
      } else {
        err = localizeError({ code: r.code, message: r.error });
      }
    } finally {
      busy = false;
      input.value = "";
    }
  }

  async function rebuild() {
    if (!backend.available) return;
    busy = true;
    err = null;
    status = null;
    const r = await syncRebuildNow();
    busy = false;
    if (r.ok) {
      status = m.sync_status_reindexed({ count: r.value });
    } else {
      err = localizeError({ code: r.code, message: r.error });
    }
  }
</script>

<div class="sync">
  <p class="muted">
    {@html m.sync_help_paragraph({
      code: "<code>notes/</code>",
    })}
  </p>

  <div class="row">
    <Button onclick={exportNow} disabled={busy || !backend.available}>
      <Download size={14} /> {m.sync_export_notes()}
    </Button>
    <Button variant="outline" onclick={() => importInput?.click()} disabled={busy || !backend.available}>
      <Upload size={14} /> {m.sync_import_zip()}
    </Button>
    <input
      type="file"
      accept=".zip,application/zip"
      bind:this={importInput}
      onchange={importNow}
      style="display:none"
    />
    <Button variant="ghost" onclick={rebuild} disabled={busy || !backend.available}>
      <RefreshCw size={14} class={busy ? "animate-spin" : ""} /> {m.sync_rebuild_index()}
    </Button>
  </div>

  {#if status}
    <p class="status"><Check size={12} /> {status}</p>
  {/if}
  {#if err}
    <p class="err-msg"><AlertCircle size={12} /> {err}</p>
  {/if}
</div>

<style>
  .muted {
    color: var(--color-muted-foreground);
    font-size: 0.875rem;
    margin: 0 0 0.75rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .status,
  .err-msg {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0.625rem 0 0;
    font-size: 0.8125rem;
    padding: 0.375rem 0.625rem;
    border-radius: var(--radius-sm);
  }
  .status {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .err-msg {
    background: var(--color-destructive-subtle);
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
