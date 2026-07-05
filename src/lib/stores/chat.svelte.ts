import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ragSearch, aiChat } from '$lib/ipc';
import type { StreamEvent } from '$lib/types/ai';

export interface Message {
  role: 'user' | 'assistant';
  content: string;
  citations?: { note_id: string; title: string }[];
}

export class ChatStore {
  messages = $state<Message[]>([]);
  streaming = $state(false);
  error = $state<string | null>(null);
  private unlisten: UnlistenFn | null = null;
  private currentId = 0;

  async send(text: string) {
    this.unlisten?.();
    this.currentId += 1;
    const myId = this.currentId;

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
      model: 'llama3.2',
      messages: [
        { role: 'system' as const, content: systemPrompt },
        { role: 'user' as const, content: text },
      ],
    };

    try {
      const { stream_id } = await aiChat(req);
      const channel = `provider://stream/${stream_id}`;
      let assistant = '';
      this.messages = [...this.messages, { role: 'assistant', content: '', citations }];

      const ul = await listen<StreamEvent>(channel, (e) => {
        if (myId !== this.currentId) return;
        const evt = e.payload;
        if (evt.kind === 'chunk') {
          assistant += evt.text;
          this.messages = this.messages.map((m, i) =>
            i === this.messages.length - 1 ? { ...m, content: assistant } : m
          );
        } else if (evt.kind === 'done') {
          this.streaming = false;
        } else if (evt.kind === 'error') {
          this.error = evt.message;
          this.streaming = false;
        }
      });
      this.unlisten = ul;
    } catch (e) {
      this.error = (e as Error).message;
      this.streaming = false;
    }
  }

  destroy() {
    this.unlisten?.();
  }
}