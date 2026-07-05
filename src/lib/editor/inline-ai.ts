import { aiComplete } from '$lib/ipc';

const DEBOUNCE_MS = 700;

export interface InlineSuggestion {
  text: string;
  onAccept: () => void;
  onDismiss: () => void;
}

export class InlineAi {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private lastText = '';
  private current: InlineSuggestion | null = null;

  constructor(
    private readonly getText: () => string,
    private readonly onSuggest: (s: InlineSuggestion | null) => void,
  ) {}

  trigger() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(async () => {
      const text = this.getText();
      if (!text || text === this.lastText) return;
      this.lastText = text;
      try {
        const completion = await aiComplete(text + '\n');
        const cont = strip_continuation(completion);
        if (cont) {
          const self = this;
          this.current = {
            text: cont,
            onAccept: () => { self.current = null; self.onSuggest(null); },
            onDismiss: () => { self.current = null; self.onSuggest(null); },
          };
          this.onSuggest(this.current);
        }
      } catch (e) {
        // ponytail: silent failure; UI shows nothing.
      }
    }, DEBOUNCE_MS);
  }

  cancel() {
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    if (this.current) {
      this.current = null;
      this.onSuggest(null);
    }
  }
}

function strip_continuation(s: string): string {
  const cut = s.indexOf('\n\n');
  return cut >= 0 ? s.slice(0, cut) : s.trim();
}