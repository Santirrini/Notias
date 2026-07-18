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
  import { m } from "$lib/i18n";

  let tab = $state<"today" | "quizzes" | "plan">("today");
  let selectedNoteIds = $state<string[]>([]);

  $effect(() => {
    selectedNoteIds = page.url.searchParams.getAll("note");
  });
</script>

<PageHeader
  title={m.study_page_title()}
  description={m.study_page_description()}
>
  {#snippet icon()}<GraduationCap size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="warning"
      title={m.offline_study_title()}
      description={m.offline_study_description()}
    />
  {/if}

  <Tabs value={tab} onValueChange={(v) => (tab = (v as typeof tab) ?? "today")}>
    <TabsList>
      <TabsTrigger value="today">
        <BrainCircuit size={13} />
        {m.study_tab_today()}
      </TabsTrigger>
      <TabsTrigger value="quizzes">
        <ListChecks size={13} />
        {m.study_tab_quizzes()}
      </TabsTrigger>
      <TabsTrigger value="plan">
        <CalendarRange size={13} />
        {m.study_tab_plan()}
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