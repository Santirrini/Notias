<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { browser } from "$app/environment";
  import { ModeWatcher } from "mode-watcher";
  import { Toaster } from "$lib/components/ui/sonner/index.js";

  import NavRail from "$lib/components/NavRail.svelte";
  import SectionTabs from "$lib/components/SectionTabs.svelte";
  import PageList from "$lib/components/PageList.svelte";
  import Topbar from "$lib/components/Topbar.svelte";
  import RecoveryBanner from "$lib/components/RecoveryBanner.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import ErrorOverlay from "$lib/components/ErrorOverlay.svelte";

  import { recoveryRequired } from "$lib/ipc";
  import { sections } from "$lib/stores/sections.svelte";
  import { backend } from "$lib/stores/backend.svelte";

  let { children } = $props();
  let recovery = $state(false);

  // Responsive state.
  let viewport = $state<"wide" | "medium" | "narrow">("wide");
  let railForced = $state<boolean | null>(null); // user override; null = auto
  let showMobileList = $state(false); // mobile drawer for Pages

  // Persisted rail state.
  const RAIL_KEY = "notias.rail.collapsed";

  function classifyViewport(width: number): typeof viewport {
    if (width < 800) return "narrow";
    if (width < 1100) return "medium";
    return "wide";
  }

  function updateViewport() {
    viewport = classifyViewport(window.innerWidth);
  }

  onMount(async () => {
    sections.hydrate();

    // Recovery check is best-effort; safeInvoke swallows both offline and
    // backend errors so the banner only appears when we genuinely need to
    // rebuild the index (DB hash mismatch reported by the backend).
    const rr = await recoveryRequired();
    if (rr.ok) recovery = rr.value;

    if (browser) {
      const saved = localStorage.getItem(RAIL_KEY);
      railForced = saved === null ? null : saved === "true";
      updateViewport();
      window.addEventListener("resize", updateViewport, { passive: true });
    }
  });

  const pathname = $derived(page.url?.pathname ?? "/");
  const showNotesChrome = $derived(pathname === "/notes" || pathname.startsWith("/notes/"));
  const showEditorFullBleed = $derived(
    pathname.startsWith("/notes/") && pathname !== "/notes"
  );

  const railCollapsed = $derived.by(() => {
    if (railForced !== null) return railForced;
    return viewport === "narrow";
  });

  const sectionsVisible = $derived(showNotesChrome && viewport !== "narrow");
  const listVisible = $derived.by(() => {
    if (!showNotesChrome) return false;
    if (viewport === "narrow") return showMobileList;
    if (viewport === "medium") return false;
    return true;
  });

  function toggleRail() {
    const next = !railCollapsed;
    railForced = next;
    if (browser) localStorage.setItem(RAIL_KEY, String(next));
  }

  function toggleMobileList() {
    showMobileList = !showMobileList;
  }

  function closeMobileList() {
    showMobileList = false;
  }

  async function onRebuild() {
    recovery = false;
  }
</script>

<ModeWatcher defaultMode="light" />

<div class="root" data-vp={viewport}>
  <NavRail collapsed={railCollapsed} ontoggle={toggleRail} />

  {#if showNotesChrome}
    <aside class="sections-panel" class:hidden={!sectionsVisible}>
      <SectionTabs />
    </aside>
  {/if}

  <section class="content" class:editor-mode={showEditorFullBleed}>
    <Topbar onopenlist={showMobileList ? closeMobileList : toggleMobileList} />
    {#if recovery}<RecoveryBanner onrebuild={onRebuild} />{/if}

    <div class="view" class:editor-mode={showEditorFullBleed}>
      {#if showNotesChrome}
        <aside class="list-panel" class:hidden={!listVisible}>
          <PageList />
        </aside>
      {/if}
      <div class="page-host">
        {@render children()}
      </div>
    </div>
  </section>
</div>

<CommandPalette />
<ErrorOverlay />
<Toaster richColors position="bottom-right" />

<style>
  .root {
    display: flex;
    min-height: 100vh;
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .sections-panel,
  .list-panel {
    flex-shrink: 0;
    overflow: hidden;
    transition: width 220ms cubic-bezier(0.4, 0, 0.2, 1), opacity 180ms ease;
  }
  .sections-panel {
    width: 200px;
    border-right: 1px solid var(--color-border);
  }
  .list-panel {
    width: 280px;
    border-right: 1px solid var(--color-border);
    background: var(--color-background);
  }

  .sections-panel.hidden,
  .list-panel.hidden {
    width: 0;
    opacity: 0;
    border-right-color: transparent;
  }

  .content {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    background: var(--color-background);
  }
  .content.editor-mode {
    background: var(--color-canvas-bg);
  }

  .view {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .page-host {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    height: 100%;
  }

  /* ─── Media queries ───────────────────────────────────────────────────── */
  @media (max-width: 1099px) {
    .sections-panel {
      width: 0;
      opacity: 0;
      border-right-color: transparent;
    }
  }
  @media (max-width: 799px) {
    .list-panel {
      position: fixed;
      top: 0;
      left: 0;
      bottom: 0;
      width: 84vw;
      max-width: 360px;
      z-index: 40;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
    }
    .list-panel.hidden {
      transform: translateX(-100%);
      opacity: 1;
      width: 84vw;
      max-width: 360px;
    }
    .page-host {
      flex: 1;
      min-width: 0;
    }
  }
</style>