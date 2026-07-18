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
    const r = await generateQuiz({ noteIds, count });
    this.busy = false;
    if (r.ok) {
      this.quiz = r.value;
      this.answers = new Array(this.quiz.questions.length).fill('');
      return;
    }
    if (!r.offline) this.error = r.error;
  }

  setAnswer(i: number, v: string) {
    this.answers[i] = v;
  }

  async submit() {
    if (!this.quiz) return;
    this.busy = true;
    this.error = null;
    const r = await gradeQuiz(this.quiz, this.answers);
    this.busy = false;
    if (r.ok) {
      this.result = r.value;
    } else if (!r.offline) {
      this.error = r.error;
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
    const r = await generatePlan(this.weekStart, dailyHoursCap);
    this.busy = false;
    if (r.ok) {
      this.plan = r.value;
      return;
    }
    if (!r.offline) this.error = r.error;
  }

  async load() {
    const r = await getPlan(this.weekStart);
    if (r.ok && r.value) this.plan = r.value;
    // offline / not-found / error: leave plan as is, no UI spam.
  }

  async persist() {
    if (!this.plan) return;
    const r = await savePlan(this.plan);
    if (!r.ok && !r.offline) this.error = r.error;
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