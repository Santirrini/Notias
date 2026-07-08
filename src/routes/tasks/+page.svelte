<script lang="ts">
  import { onMount } from "svelte";
  import { ListTodo } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import TaskBoard from "$lib/components/TaskBoard.svelte";
  import { tasks } from "$lib/stores/tasks.svelte";

  onMount(() => tasks.load());
</script>

<PageHeader title="Tasks" description="A simple Kanban to track what needs to happen.">
  {#snippet icon()}<ListTodo size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="warning"
      title="Backend not reachable"
      description="Tasks need the Tauri backend. Run via `pnpm tauri dev`."
    />
  {/if}
  <TaskBoard />
</div>

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    max-width: 1280px;
  }
</style>
