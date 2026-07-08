<script lang="ts">
  import { MessageSquare } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import ChatPanel from "$lib/components/ChatPanel.svelte";
</script>

<PageHeader
  title="Chat"
  description="RAG-aware chat over your notes. Pick a provider or let the router decide."
>
  {#snippet icon()}<MessageSquare size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="info"
      title="Backend not reachable"
      description="Chat streams from the Tauri backend. Run via `pnpm tauri dev`."
    />
  {/if}
  <ChatPanel />
</div>

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    max-width: 980px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
</style>
