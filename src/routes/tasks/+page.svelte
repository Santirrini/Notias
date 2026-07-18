<script lang="ts">
  import { onMount } from "svelte";
  import { ListTodo } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import TaskBoard from "$lib/components/TaskBoard.svelte";
  import { tasks } from "$lib/stores/tasks.svelte";
  import { m } from "$lib/i18n";

  onMount(() => tasks.load());
</script>

<PageHeader title={m.tasks_page_title()} description={m.tasks_page_description()}>
  {#snippet icon()}<ListTodo size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="warning"
      title={m.offline_tasks_title()}
      description={m.offline_tasks_description()}
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
