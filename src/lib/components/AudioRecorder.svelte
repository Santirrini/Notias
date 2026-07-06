<script lang="ts">
  import { writeFile } from '@tauri-apps/plugin-fs';
  import { tempDir } from '@tauri-apps/api/path';
  import { aiTranscribe } from '$lib/ipc';

  let { onInsert }: { onInsert?: (text: string) => void } = $props();

  let recording = $state(false);
  let busy = $state(false);
  let error: string | null = $state(null);
  let transcript: string | null = $state(null);

  let mediaRecorder: MediaRecorder | null = null;
  let chunks: BlobPart[] = [];
  let startedAt = 0;
  let stream: MediaStream | null = null;

  async function start() {
    error = null; transcript = null; chunks = [];
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
        ? 'audio/webm;codecs=opus'
        : 'audio/webm';
      mediaRecorder = new MediaRecorder(stream, { mimeType: mime });
      mediaRecorder.ondataavailable = (e) => { if (e.data.size > 0) chunks.push(e.data); };
      mediaRecorder.start();
      startedAt = Date.now();
      recording = true;
    } catch (e) {
      error = `mic permission denied: ${(e as Error).message}`;
      cleanup();
    }
  }

  function cleanup() {
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
    mediaRecorder = null;
  }

  async function stop() {
    if (!mediaRecorder || mediaRecorder.state === 'inactive') return;
    const finished = new Promise<void>((resolve) => {
      mediaRecorder!.onstop = () => resolve();
    });
    mediaRecorder.stop();
    await finished;
    recording = false;

    const elapsed = (Date.now() - startedAt) / 1000;
    if (elapsed < 0.5) { error = 'too short — record at least 0.5s'; cleanup(); return; }

    busy = true;
    try {
      const blob = new Blob(chunks, { type: 'audio/webm' });
      const buf = new Uint8Array(await blob.arrayBuffer());
      const dir = await tempDir();
      const path = `${dir}/notias-${Date.now()}.webm`;
      // ponytail: plugin-fs writeFile expects the path argument as the absolute filesystem path,
      // without a separate baseDir option here (the path already includes tempDir()).
      await writeFile(path, buf);
      transcript = await aiTranscribe(path);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
      cleanup();
    }
  }

  function insert() {
    if (transcript && onInsert) onInsert(transcript);
  }
</script>

<div class="rec">
  {#if !recording && !busy}
    <button onclick={start}>Record</button>
  {:else if recording}
    <button onclick={stop}>Stop</button>
    <span class="status">recording…</span>
  {:else}
    <span class="status">transcribing…</span>
  {/if}
  {#if transcript}
    <div class="preview">
      <p class="preview-label">Transcript (click Insert to add to your note):</p>
      <pre>{transcript}</pre>
      <button onclick={insert}>Insert</button>
    </div>
  {/if}
  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  .rec { padding: 0.5rem 0; }
  .rec button { padding: 0.4rem 0.9rem; }
  .status { color: #666; font-size: 0.9em; margin-left: 0.5rem; }
  .preview { margin-top: 0.5rem; padding: 0.75rem; background: #f4f4f4; border-radius: 4px; }
  .preview-label { color: #555; font-size: 0.85em; margin: 0 0 0.35rem; }
  pre { white-space: pre-wrap; margin: 0 0 0.5rem; font-size: 0.9em; }
  .err { color: #c00; font-size: 0.85em; margin-top: 0.5rem; }
</style>
