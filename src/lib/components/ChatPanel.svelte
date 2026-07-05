<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ChatStore } from '$lib/stores/chat.svelte';
  const store = new ChatStore();
  let input = $state('');

  function send() {
    if (!input.trim()) return;
    const t = input;
    input = '';
    store.send(t);
  }

  onDestroy(() => store.destroy());
</script>

<div class="chat">
  <div class="messages">
    {#each store.messages as m}
      <div class={m.role}>
        <p>{m.content}</p>
        {#if m.citations && m.citations.length}
          <ul class="citations">
            {#each m.citations as c}<li>{c.title}</li>{/each}
          </ul>
        {/if}
      </div>
    {/each}
    {#if store.streaming}<p class="thinking">…</p>{/if}
    {#if store.error}<p class="error">{store.error}</p>{/if}
  </div>
  <form onsubmit={(e) => { e.preventDefault(); send(); }}>
    <input bind:value={input} placeholder="Ask anything…" />
    <button type="submit">Send</button>
  </form>
</div>

<style>
  .chat { display: flex; flex-direction: column; height: 100%; }
  .messages { flex: 1; overflow-y: auto; }
  form { display: flex; gap: 0.5rem; }
  .citations { font-size: 0.85em; color: var(--text-muted, #888); }
  .error { color: red; }
</style>