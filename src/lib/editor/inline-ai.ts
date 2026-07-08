/**
 * Inline AI suggestions.
 *
 * The class is now a tiny orchestrator: it does the AI call, deduplicates
 * against the last-seen text, and surfaces the result via a callback. The
 * caller (Milkdown.svelte) is responsible for:
 *   - debouncing (idle timer on transactions)
 *   - positioning (coordsAtCursor)
 *   - inserting (insertTextAtCursor) on accept
 *
 * Keeping this module UI-agnostic means we can unit-test it later and reuse
 * the same flow from other surfaces (e.g. a chat sidebar).
 */

import { safeAiComplete } from "$lib/stores/backend.svelte";

export interface InlineSuggestion {
  text: string;
  onAccept: () => void;
  onDismiss: () => void;
}

export type SuggestCallback = (s: InlineSuggestion | null) => void;
export type ErrorCallback = (msg: string) => void;

export class InlineAi {
  private lastText = "";
  private current: InlineSuggestion | null = null;
  private inflight = false;

  constructor(
    private readonly getText: () => string,
    private readonly onSuggest: SuggestCallback,
    private readonly onError?: ErrorCallback,
  ) {}

  /** Run an AI completion against the current editor text. No debounce — caller controls timing. */
  async trigger(): Promise<void> {
    if (this.inflight) return;
    const text = this.getText();
    if (!text || text === this.lastText) return;
    this.lastText = text;
    this.inflight = true;
    try {
      const completion = await safeAiComplete(text + "\n");
      if (completion == null) {
        // Backend offline / provider missing — surface to caller silently.
        this.onError?.("AI unavailable");
        return;
      }
      const cleaned = stripContinuation(completion);
      if (!cleaned) return;
      this.current = {
        text: cleaned,
        onAccept: () => this.dismiss(),
        onDismiss: () => this.dismiss(),
      };
      this.onSuggest(this.current);
    } finally {
      this.inflight = false;
    }
  }

  /** Force-trigger regardless of dedupe (used by the manual "✨ Suggest" button). */
  async force(): Promise<void> {
    this.lastText = "";
    await this.trigger();
  }

  dismiss() {
    this.current = null;
    this.onSuggest(null);
  }

  cancel() {
    this.dismiss();
  }

  /** True if a suggestion is currently shown. */
  get hasSuggestion(): boolean {
    return this.current !== null;
  }
}

function stripContinuation(s: string): string {
  const cut = s.indexOf("\n\n");
  const out = cut >= 0 ? s.slice(0, cut) : s;
  return out.trim();
}
