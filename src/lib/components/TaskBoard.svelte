<script lang="ts">
  import { tasks } from '$lib/stores/tasks.svelte';
  import type { TaskStatus } from '$lib/types';

  const COLS: { key: TaskStatus; label: string }[] = [
    { key: 'todo', label: 'To do' },
    { key: 'doing', label: 'Doing' },
    { key: 'done', label: 'Done' },
    { key: 'cancelled', label: 'Cancelled' },
  ];

  let newTitle = $state('');
  let newPriority: 0 | 1 | 2 = $state(0);
  let dragId = $state<string | null>(null);

  async function add() {
    if (!newTitle.trim()) return;
    await tasks.add({ title: newTitle.trim(), priority: newPriority });
    newTitle = '';
  }

  function onDragStart(id: string) {
    dragId = id;
  }

  async function onDrop(status: TaskStatus) {
    if (!dragId) return;
    await tasks.setStatus(dragId, status);
    dragId = null;
  }

  function items(status: TaskStatus) {
    return tasks.items.filter((t) => t.status === status);
  }
</script>

<header>
  <input
    placeholder="What needs to happen?"
    bind:value={newTitle}
    onkeydown={(e) => { if (e.key === 'Enter') add(); }}
  />
  <select bind:value={newPriority}>
    <option value={0}>Low</option>
    <option value={1}>Med</option>
    <option value={2}>High</option>
  </select>
  <button onclick={add}>Add</button>
</header>

<section class="board">
  {#each COLS as col}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="col"
      role="list"
      ondragover={(e) => e.preventDefault()}
      ondrop={() => onDrop(col.key)}
    >
      <h3>{col.label} <small>({items(col.key).length})</small></h3>
      {#each items(col.key) as t (t.id)}
        <article draggable="true" ondragstart={() => onDragStart(t.id)}>
          <p>{t.title}</p>
          <footer>
            {#if t.priority > 0}<span class={`p${t.priority}`}>P{t.priority}</span>{/if}
            {#if t.dueAt}<small>Due: {t.dueAt.split('T')[0]}</small>{/if}
            <button onclick={() => tasks.remove(t.id)} aria-label="Delete">×</button>
          </footer>
        </article>
      {/each}
    </div>
  {/each}
</section>

<style>
  header { display: flex; gap: 0.5rem; margin-bottom: 1rem; align-items: center; }
  header input { flex: 1; padding: 0.4rem 0.6rem; }
  .board { display: grid; grid-template-columns: repeat(4, 1fr); gap: 1rem; }
  .col { background: var(--bg-elevated, #1a1a1a); padding: 0.75rem; border-radius: 0.5rem; min-height: 60vh; }
  .col h3 { margin: 0 0 0.5rem; display: flex; justify-content: space-between; }
  .col small { opacity: 0.6; font-weight: normal; }
  article { background: #222; padding: 0.6rem; border-radius: 0.3rem; margin-bottom: 0.5rem; cursor: grab; }
  article footer { display: flex; gap: 0.5rem; align-items: center; margin-top: 0.4rem; font-size: 0.8rem; opacity: 0.85; }
  article button { margin-left: auto; }
  .p1 { color: #fa0; font-weight: bold; }
  .p2 { color: #f44; font-weight: bold; }
</style>
