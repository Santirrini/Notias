import { generateQuiz, gradeQuiz, generatePlan, savePlan, getPlan } from '$lib/ipc';
import type { Plan, Quiz, QuizResult } from '$lib/types';

class StudyStore {
  quiz = $state<Quiz | null>(null);
  answers = $state<string[]>([]);
  result = $state<QuizResult | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);

  async startQuiz(noteIds: string[], count: number) {
    this.busy = true;
    this.error = null;
    this.result = null;
    try {
      this.quiz = await generateQuiz({ noteIds, count });
      this.answers = new Array(this.quiz.questions.length).fill('');
    } catch (e) {
      this.error = (e as { message: string }).message;
    } finally {
      this.busy = false;
    }
  }

  setAnswer(i: number, v: string) {
    this.answers[i] = v;
  }

  async submit() {
    if (!this.quiz) return;
    this.busy = true;
    try {
      this.result = await gradeQuiz(this.quiz, this.answers);
    } finally {
      this.busy = false;
    }
  }

  reset() {
    this.quiz = null;
    this.answers = [];
    this.result = null;
    this.error = null;
  }
}

function weekStartThisMonday(): string {
  const d = new Date();
  const day = d.getUTCDay();
  const diff = day === 0 ? -6 : 1 - day;
  d.setUTCDate(d.getUTCDate() + diff);
  return d.toISOString().split('T')[0];
}

class PlanStore {
  plan = $state<Plan | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);
  weekStart = $state(weekStartThisMonday());

  async generate(dailyHoursCap = 4) {
    this.busy = true;
    this.error = null;
    try {
      this.plan = await generatePlan(this.weekStart, dailyHoursCap);
    } catch (e) {
      this.error = (e as { message: string }).message;
    } finally {
      this.busy = false;
    }
  }

  async load() {
    const got = await getPlan(this.weekStart);
    if (got) this.plan = got;
  }

  async persist() {
    if (!this.plan) return;
    await savePlan(this.plan);
  }

  moveBlock(i: number, toDay: string) {
    if (!this.plan) return;
    const b = this.plan.blocks[i];
    if (!b) return;
    this.plan.blocks[i] = { ...b, day: toDay };
  }
}

export const study = new StudyStore();
export const plan = new PlanStore();
