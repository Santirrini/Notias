<script lang="ts">
  import { onMount } from 'svelte';
  import { tasks } from '$lib/stores/tasks.svelte';
  import TaskBoard from '$lib/components/TaskBoard.svelte';

  onMount(() => tasks.load());
</script>

<main>
  <h1>Tasks</h1>
  <TaskBoard />
</main>

<style>
  main { padding: 1rem; }
  h1 { margin: 0 0 1rem; }
</style>
