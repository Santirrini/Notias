<script lang="ts">
  import {
    Plus,
    X,
    AlertTriangle,
    Calendar as CalendarIcon,
    GripVertical,
    CheckCircle2,
    CircleDashed,
    CircleDot,
    XCircle,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
  } from "$lib/components/ui/select/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { tasks } from "$lib/stores/tasks.svelte";
  import type { TaskStatus } from "$lib/types";
  import type { Component } from "svelte";
  import { m, formatDate } from "$lib/i18n";

  type PriorityKey = "tasks_priority_high" | "tasks_priority_med" | "tasks_priority_low";
  type ColumnKey = "tasks_col_todo" | "tasks_col_doing" | "tasks_col_done" | "tasks_col_cancelled";

  const COLS: { key: TaskStatus; labelKey: ColumnKey; tone: string; icon: Component }[] = [
    { key: "todo", labelKey: "tasks_col_todo", tone: "muted", icon: CircleDashed },
    { key: "doing", labelKey: "tasks_col_doing", tone: "accent", icon: CircleDot },
    { key: "done", labelKey: "tasks_col_done", tone: "success", icon: CheckCircle2 },
    { key: "cancelled", labelKey: "tasks_col_cancelled", tone: "muted", icon: XCircle },
  ];

  function colLabel(key: ColumnKey): string {
    return m[key]();
  }

  let newTitle = $state("");
  let newPriority: 0 | 1 | 2 = $state(0);
  let newDue = $state("");
  let dragId = $state<string | null>(null);
  let dragOverCol = $state<TaskStatus | null>(null);

  async function add() {
    if (!newTitle.trim()) return;
    await tasks.add({
      title: newTitle.trim(),
      priority: newPriority,
      dueAt: newDue ? new Date(newDue).toISOString() : null,
    });
    newTitle = "";
    newDue = "";
    newPriority = 0;
  }

  function onDragStart(id: string, e: DragEvent) {
    dragId = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id);
    }
  }

  function onDragOver(col: TaskStatus, e: DragEvent) {
    e.preventDefault();
    dragOverCol = col;
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  }

  function onDragLeave(col: TaskStatus) {
    if (dragOverCol === col) dragOverCol = null;
  }

  async function onDrop(status: TaskStatus, e: DragEvent) {
    e.preventDefault();
    const id = dragId ?? e.dataTransfer?.getData("text/plain") ?? null;
    if (!id) return;
    await tasks.setStatus(id, status);
    dragId = null;
    dragOverCol = null;
  }

  function onDragEnd() {
    dragId = null;
    dragOverCol = null;
  }

  function items(status: TaskStatus) {
    return tasks.items.filter((t) => t.status === status);
  }

  function priorityKey(p: number): PriorityKey {
    if (p >= 2) return "tasks_priority_high";
    if (p >= 1) return "tasks_priority_med";
    return "tasks_priority_low";
  }

  function priorityLabel(p: number): {
    label: string;
    variant: "secondary" | "outline" | "destructive";
  } {
    const k = priorityKey(p);
    const variant: "secondary" | "outline" | "destructive" =
      p >= 2 ? "destructive" : p >= 1 ? "outline" : "secondary";
    return { label: m[k](), variant };
  }

  function priorityDot(p: number): string {
    if (p >= 2) return "var(--color-destructive)";
    if (p >= 1) return "var(--color-warning)";
    return "var(--color-muted-foreground)";
  }

  function formatDue(d: string): string {
    try {
      const date = new Date(d);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      const due = new Date(date);
      due.setHours(0, 0, 0, 0);
      const diff = (due.getTime() - today.getTime()) / 86_400_000;
      if (diff < 0) return m.dates_overdue_with_date({ date: formatDate(date) });
      if (diff === 0) return m.tasks_due_today();
      if (diff === 1) return m.tasks_due_tomorrow();
      if (diff < 7) return m.tasks_due_in_days({ days: Math.round(diff) });
      return m.tasks_due_on({ date: formatDate(date) });
    } catch {
      return d;
    }
  }

  function isOverdue(d: string | null): boolean {
    if (!d) return false;
    try {
      const date = new Date(d);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      return date.getTime() < today.getTime();
    } catch {
      return false;
    }
  }
</script>

<header class="head">
  <Input
    placeholder={m.tasks_input_placeholder()}
    bind:value={newTitle}
    onkeydown={(e) => { if (e.key === "Enter") add(); }}
    class="input"
  />
  <Input
    type="date"
    bind:value={newDue}
    class="date-input"
    aria-label={m.tasks_input_due_aria()}
  />
  <Select
    type="single"
    value={String(newPriority)}
    onValueChange={(v) => (newPriority = (Number(v) as 0 | 1 | 2) ?? 0)}
  >
    <SelectTrigger class="select-trigger">
      <span class="pri-dot" style:background={priorityDot(newPriority)}></span>
      {m[priorityKey(newPriority)]()}
    </SelectTrigger>
    <SelectContent>
      <SelectItem value="0">
        <span class="pri-dot" style:background={priorityDot(0)}></span> {m.tasks_priority_low()}
      </SelectItem>
      <SelectItem value="1">
        <span class="pri-dot" style:background={priorityDot(1)}></span> {m.tasks_priority_med()}
      </SelectItem>
      <SelectItem value="2">
        <span class="pri-dot" style:background={priorityDot(2)}></span> {m.tasks_priority_high()}
      </SelectItem>
    </SelectContent>
  </Select>
  <Button onclick={add} disabled={!newTitle.trim()}>
    <Plus size={14} /> {m.tasks_add_button()}
  </Button>
</header>

{#if tasks.items.length === 0}
  <div class="empty-board">
    <span class="empty-icon"><AlertTriangle size={20} /></span>
    <div>
      <p>{m.tasks_empty_title()}</p>
      <small>
        {m.tasks_empty_hint({
          key_enter: "<kbd>Enter</kbd>",
          strong: "<strong>Add</strong>",
        }).replace(/<kbd>|<strong>|<\/kbd>|<\/strong>/g, "")}
      </small>
    </div>
  </div>
{:else}
  <section class="board">
    {#each COLS as col}
      <div
        class="col"
        class:drop={dragOverCol === col.key}
        role="list"
        ondragover={(e) => onDragOver(col.key, e)}
        ondragleave={() => onDragLeave(col.key)}
        ondrop={(e) => onDrop(col.key, e)}
      >
        <header>
          <span class="label">
            <col.icon size={12} />
            {colLabel(col.labelKey)}
          </span>
          <span class="count">{items(col.key).length}</span>
        </header>
        {#each items(col.key) as t (t.id)}
          {@const pl = priorityLabel(t.priority)}
          {@const overdue = isOverdue(t.dueAt)}
          <article
            draggable="true"
            class:dragging={dragId === t.id}
            ondragstart={(e) => onDragStart(t.id, e)}
            ondragend={onDragEnd}
          >
            <span class="grip" aria-hidden="true"><GripVertical size={12} /></span>
            <p class="title">{t.title}</p>
            <footer>
              <Badge variant={pl.variant} class="pri-badge">
                <span class="pri-dot" style:background={priorityDot(t.priority)}></span>
                {pl.label}
              </Badge>
              {#if t.dueAt}
                <small class:overdue>
                  <CalendarIcon size={10} />
                  {formatDue(t.dueAt)}
                </small>
              {/if}
              <button
                type="button"
                class="del"
                onclick={() => tasks.remove(t.id)}
                aria-label={m.tasks_delete_task_aria()}
                title={m.tasks_delete_title()}
              >
                <X size={12} />
              </button>
            </footer>
          </article>
        {/each}
      </div>
    {/each}
  </section>
{/if}

<style>
  .head {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
    align-items: center;
    flex-wrap: wrap;
  }
  :global(.head .input) {
    flex: 1;
    min-width: 200px;
    height: 36px;
  }
  :global(.head .date-input) {
    height: 36px;
    width: 150px;
    font-variant-numeric: tabular-nums;
  }
  :global(.head .select-trigger) {
    height: 36px;
    min-width: 110px;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  .empty-board {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1.25rem 1.5rem;
    background: var(--color-muted);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-md);
    color: var(--color-foreground);
  }
  .empty-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 999px;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .empty-board p {
    margin: 0;
    font-weight: 500;
    color: var(--color-foreground);
  }
  .empty-board small {
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
    display: block;
    margin-top: 0.125rem;
  }

  .board {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }
  @media (max-width: 1100px) {
    .board {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 700px) {
    .board {
      grid-template-columns: 1fr;
    }
  }

  .col {
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    padding: 0.75rem;
    border-radius: var(--radius-md);
    min-height: 50vh;
    transition: background 120ms ease, border-color 120ms ease, box-shadow 120ms ease;
  }
  .col.drop {
    background: var(--color-accent-subtle);
    border-color: var(--color-accent);
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--color-accent) 50%, transparent);
  }
  .col > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }
  .label {
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-foreground);
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }
  .label :global(svg) {
    color: var(--color-muted-foreground);
  }
  .count {
    background: var(--color-background);
    border: 1px solid var(--color-border);
    font-size: 0.7rem;
    padding: 1px 7px;
    border-radius: 999px;
    color: var(--color-muted-foreground);
    font-variant-numeric: tabular-nums;
    min-width: 24px;
    text-align: center;
  }

  article {
    background: var(--color-card);
    border: 1px solid var(--color-border);
    padding: 0.625rem 0.75rem;
    border-radius: var(--radius-sm);
    margin-bottom: 0.4rem;
    cursor: grab;
    transition: transform 100ms ease, box-shadow 100ms ease, opacity 100ms ease;
    position: relative;
  }
  article:hover {
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.08);
    border-color: color-mix(in srgb, var(--color-accent) 30%, var(--color-border));
  }
  article:active {
    cursor: grabbing;
  }
  article.dragging {
    opacity: 0.4;
  }
  article .title {
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-foreground);
    line-height: 1.35;
    padding-left: 18px;
  }
  article footer {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.5rem;
    flex-wrap: wrap;
  }
  article footer small {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
  }
  article footer small.overdue {
    color: var(--color-destructive);
    font-weight: 500;
  }
  :global(.pri-badge) {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.65rem;
  }
  .pri-dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .grip {
    position: absolute;
    top: 0.625rem;
    left: 0.4rem;
    color: var(--color-muted-foreground);
    opacity: 0.5;
  }
  article:hover .grip {
    opacity: 1;
  }

  .del {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--color-muted-foreground);
    cursor: pointer;
    transition: opacity 100ms ease, background 100ms ease, color 100ms ease;
    opacity: 0;
  }
  article:hover .del {
    opacity: 1;
  }
  .del:hover {
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
  }
</style>