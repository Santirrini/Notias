<script lang="ts">
  import { MessageSquare } from "@lucide/svelte";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import ChatPanel from "$lib/components/ChatPanel.svelte";
  import { m } from "$lib/i18n";
</script>

<PageHeader
  title={m.chat_page_title()}
  description={m.chat_page_description()}
>
  {#snippet icon()}<MessageSquare size={22} />{/snippet}
</PageHeader>

<div class="page-pad">
  {#if !backend.available}
    <OfflineCallout
      variant="info"
      title={m.offline_chat_title()}
      description={m.offline_chat_description()}
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
