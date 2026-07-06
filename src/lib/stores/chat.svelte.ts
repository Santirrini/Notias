import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ragSearch, aiChat } from '$lib/ipc';
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

// ponytail: ChatStore subscribes to `provider://stream/*` ONCE at construction so events
// emitted between `aiChat(req)` returning and `listen(...)` resolving are not dropped.
// The store filters by the active stream_id.

export class ChatStore {
  messages = $state<Message[]>([]);
  streaming = $state(false);
  error = $state<string | null>(null);
  preferredProvider: ChatProviderKind | 'auto' = $state('auto');
  private activeStreamId: string | null = null;
  private unlisten: UnlistenFn | null = null;
  private timeoutHandle: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    listen<StreamEvent>('provider://stream/*', (e) => {
      const m = e.event.match(/^provider:\/\/stream\/(.+)$/);
      if (!m) return;
      const sid = m[1];
      if (sid !== this.activeStreamId) return;
      const evt = e.payload;
      this.clearTimeout();
      if (evt.kind === 'chunk') {
        this.appendLast(evt.text);
      } else if (evt.kind === 'done') {
        this.streaming = false;
        this.activeStreamId = null;
      } else if (evt.kind === 'error') {
        this.error = evt.message;
        this.streaming = false;
        this.activeStreamId = null;
      }
    }).then((ul) => { this.unlisten = ul; });
  }

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

  async send(text: string) {
    this.messages = [...this.messages, { role: 'user', content: text }];
    this.streaming = true;
    this.error = null;

    let context = '';
    let citations: { note_id: string; title: string }[] = [];
    try {
      const hits = await ragSearch(text);
      citations = hits.map((h) => ({ note_id: h.note_id, title: h.title }));
      context = hits.map((h) => `[${h.title}]`).join('\n');
    } catch (e) {
      console.warn('rag failed', e);
    }

    const systemPrompt = `You answer using the provided note context. Cite note titles in [brackets]. Context:\n${context || '(no context)'}`;
    const req = {
      provider: this.preferredProvider,
      model: MODEL_BY_PROVIDER[this.preferredProvider],
      messages: [
        { role: 'system' as const, content: systemPrompt },
        { role: 'user' as const, content: text },
      ],
    };

    try {
      const { stream_id } = await aiChat(req);
      this.activeStreamId = stream_id;
      this.messages = [...this.messages, { role: 'assistant', content: '', citations }];
      // ponytail: 60s timeout in case provider emits chunks but never finishes.
      this.timeoutHandle = setTimeout(() => {
        if (this.streaming) {
          this.error = 'stream timeout';
          this.streaming = false;
          this.activeStreamId = null;
        }
      }, 60_000);
    } catch (e) {
      this.error = (e as Error).message;
      this.streaming = false;
    }
  }

  destroy() {
    this.clearTimeout();
    this.unlisten?.();
  }
}
