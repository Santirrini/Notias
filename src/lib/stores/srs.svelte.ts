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
    try {
      this.queue = await srsQueue(limit);
      this.currentIndex = 0;
      this.showBack = false;
    } catch (e) {
      this.error = (e as { message: string }).message;
    } finally {
      this.busy = false;
    }
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
    try {
      await reviewCard(this.current.id, { quality } satisfies ReviewOutcome);
    } catch (e) {
      this.error = (e as { message: string }).message;
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
    await suspendCard(id, suspended);
    await this.loadQueue(this.queue.length || 20);
  }

  async generate(noteId: string, count: number): Promise<DraftCard[]> {
    return await generateCards(noteId, count);
  }

  async saveBatch(noteId: string | null, cards: DraftCard[]) {
    const input: SaveCardsInput = { noteId, cards };
    return await saveCards(input);
  }
}

export const srs = new SrsStore();
