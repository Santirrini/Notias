<script lang="ts">
  import { writeFile, remove } from "@tauri-apps/plugin-fs";
  import { tempDir } from "@tauri-apps/api/path";
  import {
    Mic,
    Loader2,
    AlertTriangle,
    Check,
    AudioLines,
    CircleStop,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { safeAiTranscribe, backend } from "$lib/stores/backend.svelte";
  import { m } from "$lib/i18n";

  let { onInsert }: { onInsert?: (text: string) => void } = $props();

  type Phase =
    | { kind: "idle" }
    | { kind: "requesting" }
    | { kind: "recording"; startedAt: number }
    | { kind: "uploading" }
    | { kind: "done"; transcript: string }
    | { kind: "error"; message: string };

  let phase: Phase = $state({ kind: "idle" });
  let now = $state(Date.now());

  let mediaRecorder: MediaRecorder | null = null;
  let chunks: BlobPart[] = [];
  let stream: MediaStream | null = null;

  // Lightweight "tick" to refresh the timecode while recording.
  let tickTimer: ReturnType<typeof setInterval> | null = null;
  function startTicker() {
    if (tickTimer) clearInterval(tickTimer);
    now = Date.now();
    tickTimer = setInterval(() => (now = Date.now()), 250);
  }
  function stopTicker() {
    if (tickTimer) clearInterval(tickTimer);
    tickTimer = null;
  }

  const elapsed = $derived.by(() => {
    if (phase.kind !== "recording") return 0;
    return Math.max(0, (now - phase.startedAt) / 1000);
  });
  const elapsedLabel = $derived(formatTime(elapsed));
  const levelBars = $derived(makeBars(elapsed));

  function formatTime(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  function makeBars(seed: number): number[] {
    return Array.from({ length: 5 }, (_, i) => {
      const v = Math.abs(Math.sin((seed + i) * 1.7));
      return Math.round(8 + v * 16);
    });
  }

  async function start() {
    chunks = [];
    phase = { kind: "requesting" };
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mime = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : "audio/webm";
      mediaRecorder = new MediaRecorder(stream, { mimeType: mime });
      mediaRecorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunks.push(e.data);
      };
      mediaRecorder.start();
      phase = { kind: "recording", startedAt: Date.now() };
      startTicker();
    } catch (e) {
      const rawMsg = (e as Error).message ?? "";
      const full = m.audio_mic_permission({ message: rawMsg });
      backend.recordError(full);
      phase = { kind: "error", message: full };
      cleanup();
    }
  }

  function cleanup() {
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
    mediaRecorder = null;
    stopTicker();
  }

  async function stop() {
    if (!mediaRecorder || mediaRecorder.state === "inactive") return;
    const finished = new Promise<void>((resolve) => {
      mediaRecorder!.onstop = () => resolve();
    });
    mediaRecorder.stop();
    await finished;
    stopTicker();

    const secs = (Date.now() - (phase.kind === "recording" ? phase.startedAt : Date.now())) / 1000;
    if (secs < 0.5) {
      phase = { kind: "error", message: m.audio_too_short() };
      cleanup();
      return;
    }

    phase = { kind: "uploading" };
    try {
      const blob = new Blob(chunks, { type: "audio/webm" });
      const buf = new Uint8Array(await blob.arrayBuffer());
      const dir = await tempDir();
      const path = `${dir}/notias-${Date.now()}.webm`;
      await writeFile(path, buf);
      const transcript = await safeAiTranscribe(path);
      if (path) {
        try {
          await remove(path);
        } catch {
          /* ignore */
        }
      }
      if (transcript == null) {
        phase = { kind: "error", message: m.audio_transcription_unavailable() };
      } else {
        phase = { kind: "done", transcript };
      }
    } catch (e) {
      const rawMsg = (e as Error).message;
      phase = { kind: "error", message: rawMsg || m.audio_recording_failed() };
    } finally {
      cleanup();
    }
  }

  function insert() {
    if (phase.kind === "done" && onInsert) {
      onInsert(phase.transcript);
      phase = { kind: "idle" };
    }
  }

  function reset() {
    phase = { kind: "idle" };
  }
</script>

<div class="rec" aria-live="polite">
  <div class="rec-head">
    <span class="rec-icon"><Mic size={14} /></span>
    <strong>{m.audio_voice_note_title()}</strong>
    <span class="rec-hint">
      {@html m.audio_press_space_hint({ key: "<kbd>Space</kbd>" })}
    </span>
  </div>

  {#if phase.kind === "idle" || phase.kind === "error" || phase.kind === "done"}
    <div class="row">
      <Button size="sm" onclick={start}>
        <Mic size={14} /> {m.audio_record_button()}
      </Button>
    </div>
  {:else if phase.kind === "requesting"}
    <div class="row">
      <Button size="sm" disabled>
        <Loader2 size={14} class="spin" /> {m.audio_requesting_mic()}
      </Button>
    </div>
  {:else if phase.kind === "recording"}
    <div class="row recording-row">
      <Button size="sm" variant="destructive" onclick={stop}>
        <CircleStop size={12} /> {m.audio_stop_button()}
      </Button>
      <span class="time">{elapsedLabel}</span>
      <span class="bars" aria-hidden="true">
        {#each levelBars as h, i (i)}
          <span class="bar" style:height="{h}px"></span>
        {/each}
      </span>
      <span class="rec-dot" aria-hidden="true"></span>
      <span class="rec-state">{m.audio_recording_state()}</span>
    </div>
  {:else if phase.kind === "uploading"}
    <div class="row">
      <Button size="sm" disabled>
        <Loader2 size={14} class="spin" /> {m.audio_transcribing()}
      </Button>
    </div>
  {/if}

  {#if phase.kind === "done"}
    <div class="preview">
      <p class="preview-label">
        <Check size={12} class="ok" /> {m.audio_transcript_label()}
      </p>
      <pre>{phase.transcript}</pre>
      <div class="row">
        <Button size="sm" onclick={insert}>
          <AudioLines size={13} /> {m.audio_insert_into_note()}
        </Button>
        <Button size="sm" variant="ghost" onclick={reset}>{m.audio_discard()}</Button>
      </div>
    </div>
  {/if}

  {#if phase.kind === "error"}
    <div class="error" role="alert">
      <AlertTriangle size={12} />
      <span>{phase.message}</span>
      <Button size="xs" variant="ghost" onclick={reset}>{m.audio_dismiss()}</Button>
    </div>
  {/if}
</div>

<svelte:window
  onkeydown={(e) => {
    if (e.code !== "Space") return;
    const target = e.target as HTMLElement;
    if (target && ["INPUT", "TEXTAREA"].includes(target.tagName)) return;
    if (target && target.isContentEditable) return;
    e.preventDefault();
    if (phase.kind === "idle" || phase.kind === "error" || phase.kind === "done") start();
    else if (phase.kind === "recording") stop();
  }}
/>

<style>
  .rec {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.75rem;
  }

  .rec-head {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8125rem;
    color: var(--color-foreground);
  }
  .rec-head strong {
    font-weight: 600;
  }
  .rec-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .rec-hint {
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
    margin-left: auto;
  }

  .row {
    display: inline-flex;
    align-items: center;
    gap: 0.625rem;
    flex-wrap: wrap;
  }

  .recording-row {
    background: color-mix(in srgb, var(--color-destructive) 8%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    margin-left: -0.5rem;
    margin-right: -0.5rem;
  }
  .time {
    font-variant-numeric: tabular-nums;
    font-size: 0.9375rem;
    color: var(--color-destructive);
    font-weight: 600;
  }
  .bars {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 24px;
  }
  .bar {
    display: inline-block;
    width: 3px;
    background: var(--color-destructive);
    border-radius: 1px;
    transition: height 120ms ease;
  }
  .rec-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-destructive);
    animation: pulse 1.2s ease-in-out infinite;
    margin-left: auto;
  }
  .rec-state {
    color: var(--color-destructive);
    font-size: 0.75rem;
    font-weight: 500;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.7); }
  }

  .preview {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.625rem 0.875rem;
    background: var(--color-background);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .preview-label {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
  }
  :global(.preview-label .ok) {
    color: var(--color-success, #107c10);
  }
  pre {
    white-space: pre-wrap;
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-foreground);
    line-height: 1.5;
    font-family: inherit;
  }

  .error {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-destructive);
    background: var(--color-destructive-subtle);
    border: 1px solid color-mix(in srgb, var(--color-destructive) 35%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
  }

  :global(.spin) {
    animation: rec-spin 1s linear infinite;
  }
  @keyframes rec-spin {
    to { transform: rotate(360deg); }
  }
  :global(.rec kbd) {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.6875rem;
    background: var(--color-background);
    padding: 0 4px;
    border-radius: 3px;
    border: 1px solid var(--color-border);
    color: var(--color-muted-foreground);
  }
</style>