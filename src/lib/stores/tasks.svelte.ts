import { listTasks, createTask, updateTask, deleteTask } from '$lib/ipc';
import { safeInvoke } from '$lib/stores/backend.svelte';
import type { NewTask, TaskPatch, TaskStatus, TaskSummary } from '$lib/types';

class TasksStore {
  items = $state<TaskSummary[]>([]);
  filter = $state<TaskStatus | 'all'>('all');

  async load() {
    // safeInvoke swallows the "no Tauri runtime" rejection so `onMount` callers
    // don't have to wrap every load() in a try/catch. The store keeps the
    // previous `items` value when offline — components show their own empty
    // state via the `backend` store's `available` flag.
    const r = await safeInvoke<TaskSummary[]>('list_tasks');
    if (r.ok) this.items = r.value;
  }

  get visible() {
    return this.filter === 'all' ? this.items : this.items.filter(t => t.status === this.filter);
  }

  async add(input: NewTask) {
    const r = await safeInvoke('create_task', { input });
    if (r.ok) await this.load();
  }

  async setStatus(id: string, status: TaskStatus) {
    const r = await safeInvoke('update_task', { id, patch: { status } });
    if (r.ok) await this.load();
  }

  async patch(id: string, patch: TaskPatch) {
    const r = await safeInvoke('update_task', { id, patch });
    if (r.ok) await this.load();
  }

  async remove(id: string) {
    const r = await safeInvoke('delete_task', { id });
    if (r.ok) await this.load();
  }
}

export const tasks = new TasksStore();
