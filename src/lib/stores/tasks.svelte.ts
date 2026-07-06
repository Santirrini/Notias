import { listTasks, createTask, updateTask, deleteTask } from '$lib/ipc';
import type { NewTask, TaskPatch, TaskStatus, TaskSummary } from '$lib/types';

class TasksStore {
  items = $state<TaskSummary[]>([]);
  filter = $state<TaskStatus | 'all'>('all');

  async load() {
    this.items = await listTasks();
  }

  get visible() {
    return this.filter === 'all' ? this.items : this.items.filter(t => t.status === this.filter);
  }

  async add(input: NewTask) {
    await createTask(input);
    await this.load();
  }

  async setStatus(id: string, status: TaskStatus) {
    await updateTask(id, { status });
    await this.load();
  }

  async patch(id: string, patch: TaskPatch) {
    await updateTask(id, patch);
    await this.load();
  }

  async remove(id: string) {
    await deleteTask(id);
    await this.load();
  }
}

export const tasks = new TasksStore();
