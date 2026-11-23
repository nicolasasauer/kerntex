<script lang="ts">
  import { onMount } from "svelte";
  import X from "@lucide/svelte/icons/x";
  import FileText from "@lucide/svelte/icons/file-text";
  import { createEditor } from "../lib/latex/editor";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { fmt } from "../lib/commands.svelte";
  import FilePreview from "./FilePreview.svelte";
  import Image from "@lucide/svelte/icons/image";

  let host: HTMLDivElement | undefined = $state();
  const activeTab = $derived(ws.tabs.find((t) => t.path === ws.active));

  onMount(() => {
    const editor = createEditor(
      host!,
      {
        onChange: (path, text) => ws.onEdit(path, text),
        getRefs: () => ws.refs,
        getDiagnostics: (path) => ws.diagnostics.filter((d) => d.file === path),
        getPreamble: (path) => ws.preambleFor(path),
        addPackage: (pkg) => ws.addPackage(pkg),
      },
      { fontSize: settings.fontSize, wrap: settings.wrap, onSave: () => ws.save() },
    );
    ws.editor = editor;
    // Tabs may have been opened before the editor mounted.
    for (const t of ws.tabs) if (t.kind === "text") editor.open(t.path, t.saved);
    if (ws.active && activeTab?.kind === "text") editor.open(ws.active, "");
    return () => {
      ws.editor = null;
      editor.destroy();
    };
  });

  $effect(() => {
    ws.editor?.setOptions({ fontSize: settings.fontSize, wrap: settings.wrap });
  });

  function name(path: string) {
    return path.split("/").pop();
  }
</script>

<div class="pane">
  <div class="tabs" role="tablist">
    {#each ws.tabs as tab (tab.path)}
      <div class="tab" class:active={ws.active === tab.path} role="tab" aria-selected={ws.active === tab.path}>
        <button
          class="tab-label"
          title={tab.path}
          onclick={() => ws.activate(tab.path)}
          onauxclick={(e) => e.button === 1 && ws.closeTab(tab.path)}
        >
          {#if tab.kind === "preview"}<Image size={13} />{/if}
          {name(tab.path)}
          {#if ws.mainFile === tab.path}<span class="main-tag">main</span>{/if}
        </button>
        <button class="close" class:dirty={tab.dirty} title={tab.dirty ? "Unsaved changes — click to save and close" : "Close"} aria-label="Close {name(tab.path)}" onclick={() => ws.closeTab(tab.path)}>
          <span class="dot"></span>
          <X size={13} />
        </button>
      </div>
    {/each}
  </div>
  <div class="host" class:hidden={!ws.active || activeTab?.kind !== "text"} bind:this={host}></div>
  {#if activeTab?.kind === "preview"}
    {#key activeTab.path}<FilePreview path={activeTab.path} />{/key}
  {:else if !ws.active}
    <div class="empty">
      <FileText size={34} strokeWidth={1.4} />
      <p>Open a file from the tree on the left.</p>
      <p class="muted"><kbd>{fmt("Mod+K")}</kbd> shows all commands</p>
    </div>
  {/if}
</div>

<style>
  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--editor-bg);
  }
  .tabs {
    flex: none;
    display: flex;
    height: 36px;
    overflow-x: auto;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    scrollbar-width: none;
  }
  .tab {
    display: flex;
    align-items: center;
    border-right: 1px solid var(--border);
    color: var(--text-muted);
    position: relative;
    flex: none;
  }
  .tab.active {
    background: var(--editor-bg);
    color: var(--text);
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 2px;
    background: var(--accent);
  }
  .tab-label {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 4px 0 12px;
    border: none;
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
  }
  .tab:not(.active):hover {
    background: var(--hover);
  }
  .main-tag {
    font-size: 10px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-right: 6px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  .close > :global(*) {
    grid-area: 1 / 1;
  }
  .close:hover {
    background: var(--hover);
    color: var(--text);
  }
  .close:active {
    background: var(--pressed);
  }
  .close .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--warning);
    display: none;
  }
  .close.dirty .dot {
    display: block;
  }
  .close.dirty :global(svg) {
    display: none;
  }
  .close.dirty:hover .dot {
    display: none;
  }
  .close.dirty:hover :global(svg) {
    display: block;
  }
  .host {
    flex: 1;
    min-height: 0;
    user-select: text;
  }
  .host.hidden {
    display: none;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--text-faint);
  }
  .empty p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }
</style>
