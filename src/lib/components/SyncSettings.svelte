<script lang="ts">
  import { syncExportZip, syncImportZip, syncRebuildNow } from '$lib/ipc';

  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);
  let importInput: HTMLInputElement;

  async function exportNow() {
    busy = true; err = null; status = null;
    try {
      const bytes = await syncExportZip();
      const blob = new Blob([new Uint8Array(bytes)], { type: 'application/zip' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `notias-export-${new Date().toISOString().slice(0, 10)}.zip`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      status = `Exported ${bytes.length.toLocaleString()} bytes`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally { busy = false; }
  }

  async function importNow(ev: Event) {
    const input = ev.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    busy = true; err = null; status = null;
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      const n = await syncImportZip(bytes);
      status = `Imported ${n} file${n === 1 ? '' : 's'} + rebuilt index`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally {
      busy = false;
      input.value = '';
    }
  }

  async function rebuild() {
    busy = true; err = null; status = null;
    try {
      const n = await syncRebuildNow();
      status = `Reindexed ${n} note${n === 1 ? '' : 's'}`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally { busy = false; }
  }
</script>

<section class="sync">
  <h2>Sync</h2>
  <p class="muted">
    Notias does not sync your notes itself. Use your cloud-storage app (Drive,
    Dropbox, iCloud) to sync the <code>notes/</code> folder between machines.
    Use these controls to manually export or import a zip, or rebuild the index.
  </p>

  <div class="row">
    <button onclick={exportNow} disabled={busy}>Export notes.zip</button>
    <button onclick={() => importInput.click()} disabled={busy}>Import zip…</button>
    <input
      type="file"
      accept=".zip,application/zip"
      bind:this={importInput}
      onchange={importNow}
      style="display:none"
    />
    <button onclick={rebuild} disabled={busy}>Rebuild index</button>
  </div>

  {#if status}<p class="status">{status}</p>{/if}
  {#if err}<p class="err">{err}</p>{/if}
</section>

<style>
  .sync { margin: 1.5rem 0; }
  .muted { color: #666; font-size: .9em; margin-bottom: .75rem; }
  code { background: #f4f4f4; padding: 0 .3em; border-radius: 3px; }
  .row { display: flex; gap: .5rem; flex-wrap: wrap; }
  button { padding: .4rem .8rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  .status { color: #178217; margin-top: .5rem; }
  .err { color: #c00; margin-top: .5rem; }
</style>