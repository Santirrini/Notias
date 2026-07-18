import { srsQueue, reviewCard, suspendCard, generateCards, saveCards } from '$lib/ipc';
import type { CardSummary, DraftCard, SaveCardsInput, ReviewOutcome } from '$lib/types';

class SrsStore {
  queue = $state<CardSummary[]>([]);
  currentIndex = $state(0);
  showBack = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);

  async loadQueue(limit = 20) {
    this.busy = true;
    this.error = null;
    const r = await srsQueue(limit);
    this.busy = false;
    if (r.ok) {
      this.queue = r.value;
      this.currentIndex = 0;
      this.showBack = false;
      return;
    }
    if (!r.offline) this.error = r.error;
    // offline: queue stays empty; the UI shows its "Backend offline" callout.
  }

  get current() {
    return this.queue[this.currentIndex];
  }

  get hasNext() {
    return this.currentIndex < this.queue.length - 1;
  }

  reveal() {
    this.showBack = true;
  }

  async grade(quality: 1 | 3 | 4 | 5) {
    if (!this.current) return;
    const r = await reviewCard(this.current.id, { quality } satisfies ReviewOutcome);
    if (!r.ok) {
      this.error = r.offline ? "Backend offline" : r.error;
      return;
    }
    this.currentIndex += 1;
    this.showBack = false;
  }

  async skip() {
    // ponytail: skip just advances to next; doesn't write a review. Suspend is the durable "drop this".
    if (!this.hasNext) return;
    this.currentIndex += 1;
    this.showBack = false;
  }

  async toggleSuspend(id: string, suspended: boolean) {
    const r = await suspendCard(id, suspended);
    if (!r.ok && !r.offline) this.error = r.error;
    await this.loadQueue(this.queue.length || 20);
  }

  /** Generate draft cards from a note. Returns drafts or null on failure. */
  async generate(noteId: string, count: number): Promise<DraftCard[] | null> {
    const r = await generateCards(noteId, count);
    if (r.ok) return r.value;
    this.error = r.offline ? "Backend offline" : r.error;
    return null;
  }

  /** Persist a batch of draft cards. Returns saved cards or null on failure. */
  async saveBatch(noteId: string | null, cards: DraftCard[]) {
    const input: SaveCardsInput = { noteId, cards };
    const r = await saveCards(input);
    if (r.ok) return r.value;
    this.error = r.offline ? "Backend offline" : r.error;
    return null;
  }
}

export const srs = new SrsStore();