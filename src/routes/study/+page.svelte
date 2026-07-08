<script lang="ts">
  import { page } from "$app/state";
  import {
    GraduationCap,
    BrainCircuit,
    ListChecks,
    CalendarRange,
  } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { Tabs, TabsContent, TabsList, TabsTrigger } from "$lib/components/ui/tabs/index.js";
  import { backend } from "$lib/stores/backend.svelte";
  import SRSReviewer from "$lib/components/SRSReviewer.svelte";
  import QuizRunner from "$lib/components/QuizRunner.svelte";
  import WeeklyPlan from "$lib/components/WeeklyPlan.svelte";

  let tab = $state<"today" | "quizzes" | "plan">("today");
  let selectedNoteIds = $state<string[]>([]);

  $effect(() => {
    selectedNoteIds = page.url.searchParams.getAll("note");
  });
</script>

<PageHeader
  title="Study"
  description="Spaced repetition, auto-graded quizzes, weekly AI plan."
>
  {#snippet icon()}<GraduationCap size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="warning"
      title="Backend not reachable"
      description="Cards, quizzes and plans need the Tauri backend."
    />
  {/if}

  <Tabs value={tab} onValueChange={(v) => (tab = (v as typeof tab) ?? "today")}>
    <TabsList>
      <TabsTrigger value="today">
        <BrainCircuit size={13} />
        Today
      </TabsTrigger>
      <TabsTrigger value="quizzes">
        <ListChecks size={13} />
        Quizzes
      </TabsTrigger>
      <TabsTrigger value="plan">
        <CalendarRange size={13} />
        Plan
      </TabsTrigger>
    </TabsList>

    <TabsContent value="today">
      <SRSReviewer noteId={selectedNoteIds[0]} />
    </TabsContent>
    <TabsContent value="quizzes">
      <QuizRunner noteIds={selectedNoteIds} />
    </TabsContent>
    <TabsContent value="plan">
      <WeeklyPlan />
    </TabsContent>
  </Tabs>
</div>

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    max-width: 1100px;
  }

  :global([data-slot="tabs-trigger"]) {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8125rem;
  }
</style>