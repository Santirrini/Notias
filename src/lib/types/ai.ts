export type ChatProviderKind = 'ollama' | 'openai' | 'groq';

export interface ProviderInfo {
  name: string;
  enabled: boolean;
  healthy: boolean;
  detail: string | null;
}

export interface ProviderStatus {
  healthy: boolean;
  detail: string | null;
}

export interface StreamHandle {
  stream_id: string;
}

export type StreamEvent =
  | { kind: 'chunk'; text: string }
  | { kind: 'done' }
  | { kind: 'error'; message: string };

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface RagHit {
  note_id: string;
  title: string;
  distance: number;
}