import { ragSearch, aiChat } from '$lib/ipc';
import { safeListen, backend } from '$lib/stores/backend.svelte';
import type { StreamEvent, ChatProviderKind } from '$lib/types/ai';

export interface Message {
  role: 'user' | 'assistant';
  content: string;
  citations?: { note_id: string; title: string }[];
}

const MODEL_BY_PROVIDER: Record<ChatProviderKind | 'auto', string> = {
  auto: 'llama3.2',
  ollama: 'llama3.2',
  openai: 'gpt-4o-mini',
  groq: 'llama-3.1-70b-versatile',
};

/**
 * ChatStore subscribes to the chat stream once per `send()` call, using the
 * concrete channel name `provider://stream/{stream_id}` returned by `ai_chat`.
 * Cancels the previous subscription before opening a new one so overlapping
 * sends cannot cross-contaminate.
 *
 * If the backend is offline (plain-browser `pnpm dev`), `safeInvoke` returns
 * `{ ok: false, offline: true }`; we surface a clear "Backend offline"
 * message and do not throw.
 */
export class ChatStore {
  messages = $state<Message[]>([]);
  streaming = $state(false);
  error = $state<string | null>(null);
  preferredProvider: ChatProviderKind | 'auto' = $state('auto');
  private activeStreamId: string | null = null;
  private unlisten: (() => void) | null = null;
  private timeoutHandle: ReturnType<typeof setTimeout> | null = null;

  private clearTimeout() {
    if (this.timeoutHandle) { clearTimeout(this.timeoutHandle); this.timeoutHandle = null; }
  }

  private appendLast(text: string) {
    this.messages = this.messages.map((m, i) =>
      i === this.messages.length - 1 && m.role === 'assistant'
        ? { ...m, content: m.content + text }
        : m
    );
  }

  private async openStreamSubscription(streamId: string) {
    // Cancel any previous listener so events from an earlier stream cannot
    // contaminate this one.
    this.unlisten?.();
    this.unlisten = null;

    const unlisten = await safeListen<StreamEvent>(
      `provider://stream/${streamId}`,
      (payload) => {
        this.clearTimeout();
        const evt = payload;
        if (evt.kind === 'chunk') {
          this.appendLast(evt.text);
        } else if (evt.kind === 'done') {
          this.streaming = false;
          this.activeStreamId = null;
          this.unlisten?.();
          this.unlisten = null;
        } else if (evt.kind === 'error') {
          this.error = evt.message;
          this.streaming = false;
          this.activeStreamId = null;
          this.unlisten?.();
          this.unlisten = null;
        }
      },
    );
    if (unlisten) this.unlisten = unlisten;
    // safeListen already routes failure into backend.recordError.
  }

  async send(text: string) {
    if (!backend.available) {
      this.error = 'Backend offline — start the Tauri runtime to chat.';
      return;
    }
    this.messages = [...this.messages, { role: 'user', content: text }];
    this.streaming = true;
    this.error = null;

    // RAG context. safeInvoke handles offline; we just no-op.
    let context = '';
    let citations: { note_id: string; title: string }[] = [];
    const rag = await ragSearch(text);
    if (rag.ok) {
      citations = rag.value.map((h) => ({ note_id: h.note_id, title: h.title }));
      context = rag.value.map((h) => `[${h.title}]`).join('\n');
    }
    // rag offline/error: keep streaming with no citations, no need to abort.

    const systemPrompt = `You answer using the provided note context. Cite note titles in [brackets]. Context:\n${context || '(no context)'}`;
    const req = {
      provider: this.preferredProvider,
      model: MODEL_BY_PROVIDER[this.preferredProvider],
      messages: [
        { role: 'system' as const, content: systemPrompt },
        { role: 'user' as const, content: text },
      ],
    };

    const chat = await aiChat(req);
    if (!chat.ok) {
      this.error = chat.offline
        ? 'Backend offline'
        : (chat.error || 'Failed to start chat stream');
      this.streaming = false;
      return;
    }

    this.activeStreamId = chat.value.stream_id;
    this.messages = [...this.messages, { role: 'assistant', content: '', citations }];

    // Subscribe to the specific stream channel before chunks can arrive.
    await this.openStreamSubscription(this.activeStreamId);

    // ponytail: 60s timeout in case provider emits chunks but never finishes.
    this.timeoutHandle = setTimeout(() => {
      if (this.streaming) {
        this.error = 'stream timeout';
        this.streaming = false;
        this.activeStreamId = null;
        this.unlisten?.();
        this.unlisten = null;
      }
    }, 60_000);
  }

  destroy() {
    this.clearTimeout();
    this.unlisten?.();
    this.unlisten = null;
  }
}