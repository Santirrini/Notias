<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import {
    Send,
    AlertCircle,
    Bot,
    User,
    Sparkles,
    StopCircle,
    FileText,
    ArrowRight,
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
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import { goto } from "$app/navigation";
  import { backend } from "$lib/stores/backend.svelte";
  import { ChatStore } from "$lib/stores/chat.svelte";
  import type { ChatProviderKind } from "$lib/types/ai";
  import { m, localizeError } from "$lib/i18n";

  const store = new ChatStore();
  let input = $state("");
  let inputEl = $state<HTMLElement | null>(null);
  let messagesEl = $state<HTMLElement | null>(null);

  function scrollToBottom() {
    tick().then(() => {
      if (messagesEl) {
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }
    });
  }

  async function send() {
    if (!input.trim() || store.streaming) return;
    const t = input;
    input = "";
    store.send(t);
    scrollToBottom();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  $effect(() => {
    // Auto-scroll on new content
    if (store.messages.length) scrollToBottom();
  });

  onDestroy(() => store.destroy());
</script>

<div class="chat">
  <header class="head">
    <div class="head-left">
      <Bot size={16} />
      <h2>{m.chat_title()}</h2>
    </div>
    <Badge variant="outline" class="status">
      <Sparkles size={10} />
      {m.chat_rag_status({
        provider:
          store.preferredProvider === "auto"
            ? m.chat_provider_auto()
            : store.preferredProvider,
      })}
    </Badge>
  </header>

  <div class="messages" role="log" aria-live="polite" bind:this={messagesEl}>
    {#if store.messages.length === 0 && !store.streaming && !store.error}
      <div class="empty">
        <div class="empty-icon">
          <Sparkles size={26} />
        </div>
        <p>{m.chat_empty_title()}</p>
        <p class="hint">
          {m.chat_empty_hint({
            link: `<button type="button" class="link" onclick={() => goto("/settings")">${m.chat_empty_settings_link()}<ArrowRight size={11} /></button>`,
          }).replace(/<button|class="link"|onclick|<\/button>|<ArrowRight|size=\{11\}|\/>/g, "")}
        </p>
      </div>
    {/if}

    {#each store.messages as msg, i (i)}
      <div class="msg {msg.role}">
        <span class="avatar">
          {#if msg.role === "user"}<User size={14} />{:else}<Bot size={14} />{/if}
        </span>
        <div class="bubble">
          <p>{msg.content}</p>
          {#if msg.citations && msg.citations.length}
            <ul class="citations">
              {#each msg.citations as c}
                <li>
                  <Badge variant="outline">
                    <FileText size={9} />
                    {c.title}
                  </Badge>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    {/each}

    {#if store.streaming}
      <div class="msg assistant">
        <span class="avatar"><Bot size={14} /></span>
        <div class="bubble thinking">
          <span></span><span></span><span></span>
        </div>
      </div>
    {/if}
    {#if store.error}
      <p class="err-msg"><AlertCircle size={12} /> {localizeError({ message: store.error })}</p>
    {/if}
  </div>

  <form class="composer" onsubmit={(e) => { e.preventDefault(); send(); }}>
    <Select
      type="single"
      value={store.preferredProvider}
      onValueChange={(v) => (store.preferredProvider = (v as ChatProviderKind | "auto") ?? "auto")}
    >
      <SelectTrigger class="provider-trigger" aria-label={m.chat_provider_select_aria()}>
        {store.preferredProvider === "auto"
          ? m.chat_provider_auto()
          : store.preferredProvider.charAt(0).toUpperCase() + store.preferredProvider.slice(1)}
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="auto">{m.chat_provider_auto_full()}</SelectItem>
        <SelectItem value="ollama">Ollama</SelectItem>
        <SelectItem value="openai">OpenAI</SelectItem>
        <SelectItem value="groq">Groq</SelectItem>
      </SelectContent>
    </Select>
    <Input
      bind:ref={inputEl}
      bind:value={input}
      placeholder={backend.available ? m.chat_input_placeholder_online() : m.chat_input_placeholder_offline()}
      onkeydown={onKeyDown}
      disabled={!backend.available}
      class="chat-input"
    />
    <Tooltip.Provider delayDuration={250}>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              type="submit"
              disabled={!input.trim() || !backend.available || store.streaming}
              aria-label={store.streaming ? m.chat_streaming_aria() : m.chat_send_aria()}
              {...props}
            >
              {#if store.streaming}
                <StopCircle size={14} />
              {:else}
                <Send size={14} />
              {/if}
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={6}>
          {store.streaming ? m.chat_streaming_tooltip() : m.chat_send_tooltip()}
        </Tooltip.Content>
      </Tooltip.Root>
    </Tooltip.Provider>
  </form>
</div>

<style>
  .chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 70vh;
    background: var(--color-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-muted);
  }
  .head-left {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-foreground);
  }
  .head-left h2 {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
  }
  .head-left :global(svg) {
    color: var(--color-accent);
  }
  :global(.status) {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-foreground);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.625rem;
    color: var(--color-muted-foreground);
    text-align: center;
    max-width: 380px;
    padding: 2rem 1rem;
  }
  .empty-icon {
    width: 56px;
    height: 56px;
    border-radius: 999px;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 0.25rem;
  }
  .empty p {
    margin: 0;
    font-size: 0.9375rem;
    line-height: 1.5;
    color: var(--color-foreground);
    font-weight: 500;
  }
  .empty .hint {
    font-size: 0.8125rem;
    color: var(--color-muted-foreground);
    font-weight: 400;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--color-accent);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }

  .msg {
    display: flex;
    gap: 0.625rem;
    align-items: flex-start;
    max-width: 92%;
    animation: msg-in 200ms ease;
  }
  @keyframes msg-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .msg.user {
    align-self: flex-end;
    flex-direction: row-reverse;
  }
  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--color-muted);
    color: var(--color-foreground);
    flex-shrink: 0;
  }
  .msg.user .avatar {
    background: var(--color-accent);
    color: var(--color-accent-foreground);
  }
  .bubble {
    background: var(--color-background);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.625rem 0.875rem;
    line-height: 1.55;
    font-size: 0.875rem;
    color: var(--color-foreground);
  }
  .msg.user .bubble {
    background: var(--color-accent-subtle);
    border-color: transparent;
  }
  .bubble p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .citations {
    list-style: none;
    margin: 0.625rem 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .citations :global(.badge) {
    font-size: 0.7rem;
    gap: 0.25rem;
  }

  .thinking {
    display: inline-flex;
    gap: 4px;
    padding: 0.75rem 0.875rem;
  }
  .thinking span {
    display: inline-block;
    width: 6px;
    height: 6px;
    background: var(--color-muted-foreground);
    border-radius: 999px;
    animation: bounce 1.2s infinite ease-in-out;
  }
  .thinking span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .thinking span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes bounce {
    0%, 80%, 100% {
      opacity: 0.3;
      transform: translateY(0);
    }
    40% {
      opacity: 1;
      transform: translateY(-3px);
    }
  }

  .err-msg {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    align-self: center;
    color: var(--color-destructive);
    background: var(--color-destructive-subtle);
    padding: 0.375rem 0.625rem;
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    margin: 0;
  }

  .composer {
    display: flex;
    gap: 0.5rem;
    padding: 0.625rem 0.75rem;
    border-top: 1px solid var(--color-border);
    background: var(--color-muted);
  }
  :global(.composer .provider-trigger) {
    height: 36px;
    min-width: 130px;
  }
  :global(.composer .chat-input) {
    flex: 1;
    height: 36px;
    background: var(--color-background);
  }
</style>