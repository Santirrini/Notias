export type WireError = { code: string; message: string };

export type SyncStatus = { kind: 'exported'; bytes: number }
  | { kind: 'imported'; count: number }
  | { kind: 'rebuilt'; count: number };

export type CalendarEvent = {
  gcal_id: string;
  summary: string;
  description: string | null;
  starts_at: string;
  ends_at: string;
  updated_at: string;
  source: string;
};

export type CalendarAuthStatus = { connected: boolean; expires_at?: number | null };

export type Result<T> = { ok: true; value: T } | { ok: false; error: WireError };

export type NoteSummary = { id: string; title: string; updated: string; tags: string[] };

export type NoteFrontmatter = {
  id: string;
  title: string;
  tags: string[];
  created: string;
  updated: string;
  links: string[];
  references: string[];
};

export type Note = {
  id: string;
  path: string;
  title: string;
  body: string;
  frontmatter: NoteFrontmatter;
};

// Phase 4 — Estudio autónomo
export type TaskStatus = 'todo' | 'doing' | 'done' | 'cancelled';
export type Task = {
  id: string;
  noteId: string | null;
  title: string;
  priority: number;
  status: TaskStatus;
  dueAt: string | null;
  createdAt: string;
  updatedAt: string;
  doneAt: string | null;
};
export type TaskSummary = {
  id: string;
  title: string;
  priority: number;
  status: TaskStatus;
  dueAt: string | null;
};
export type NewTask = {
  title: string;
  priority?: number;
  noteId?: string | null;
  dueAt?: string | null;
};
export type TaskPatch = {
  title?: string;
  priority?: number;
  status?: TaskStatus;
  dueAt?: string | null;
};

export type DraftCard = { front: string; back: string };
export type Card = {
  id: string;
  noteId: string | null;
  front: string;
  back: string;
  ease: number;
  intervalDays: number;
  repetitions: number;
  dueAt: string;
  createdAt: string;
  suspendedAt: string | null;
};
export type CardSummary = { id: string; front: string; back: string; dueAt: string };
export type SaveCardsInput = { noteId: string | null; cards: DraftCard[] };
export type ReviewOutcome = { quality: 1 | 3 | 4 | 5 };

export type Question =
  | { type: 'mc'; question: string; choices: string[]; answer?: number; rationale?: string }
  | { type: 'short'; question: string; answer?: string; rationale?: string }
  | { type: 'cloze'; question: string; answer?: string; rationale?: string };
export type Quiz = { id: string; noteIds: string[]; questions: Question[]; createdAt: string };
export type QuizResult = {
  quizId: string;
  score: number;
  total: number;
  perQuestion: { index: number; correct: boolean; expected: string; given: string; rationale: string }[];
};
export type NewQuizInput = { noteIds: string[]; count: number };

export type PlanBlock = { day: string; start: string; minutes: number; kind: string; refs: string[]; rationale: string };
export type Plan = { weekStart: string; dailyHoursCap: number; blocks: PlanBlock[] };