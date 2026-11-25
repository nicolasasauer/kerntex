<script lang="ts">
  import { onMount } from "svelte";
  import { ws } from "../lib/workspace.svelte";
  import { settings, effectiveTheme } from "../lib/settings.svelte";
  import { ui, handleGlobalKey } from "../lib/commands.svelte";
  import { isTauri } from "../lib/api";
  import { toast } from "../lib/toasts.svelte";
  import Toolbar from "./Toolbar.svelte";
  import StartScreen from "./StartScreen.svelte";
  import FileTree from "./FileTree.svelte";
  import EditorPane from "./EditorPane.svelte";
  import PdfView from "./PdfView.svelte";
  import ProblemsPanel from "./ProblemsPanel.svelte";
  import CommandPalette from "./CommandPalette.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import NewProjectDialog from "./NewProjectDialog.svelte";
  import AboutDialog from "./AboutDialog.svelte";
  import McpDialog from "./McpDialog.svelte";
  import { mcp, initMcp } from "../lib/mcp.svelte";
  import Toasts from "./Toasts.svelte";
  import Splitter from "./Splitter.svelte";

  let dragging = $state(false);
  let main: HTMLDivElement | undefined = $state();

  $effect(() => {
    document.documentElement.dataset.theme = effectiveTheme();
  });

  async function handleDrop(paths: string[]) {
    const p = paths[0];
    if (!p) return;
    if (/\.zip$/i.test(p)) await ws.importZip(p);
    else if (/\.(tex|bib|pdf|png|jpe?g)$/i.test(p)) {
      toast({ kind: "info", source: "KernTeX", title: "Drop a folder or a .zip", body: "To open a project, drop its folder (or an Overleaf ZIP) here." });
    } else await ws.openProject(p);
  }

  /** Project folder under a (physical-pixel) drop position, or null if not over the tree. */
  function treeDirAt(px: number, py: number): string | null {
    if (!ws.project) return null;
    const dpr = window.devicePixelRatio || 1;
    const el = document.elementFromPoint(px / dpr, py / dpr);
    if (!el?.closest(".filetree")) return null;
    const row = el.closest<HTMLElement>("[data-dir]");
    return row?.dataset.dir ?? "";
  }

  onMount(() => {
    ws.init();
    initMcp();
    // Dev-only handle for automated UI checks (stripped from production builds).
    if (import.meta.env.DEV) (window as unknown as { __kerntex: unknown }).__kerntex = ws;
    let unlisten: (() => void) | undefined;
    if (isTauri) {
      import("@tauri-apps/api/webview").then(async ({ getCurrentWebview }) => {
        unlisten = await getCurrentWebview().onDragDropEvent((e) => {
          const t = e.payload.type;
          if (t === "leave") {
            dragging = false;
            ui.dropDir = null;
            return;
          }
          // Over the file tree of an open project → add files to the folder under the pointer.
          const dir = "position" in e.payload ? treeDirAt(e.payload.position.x, e.payload.position.y) : null;
          if (t === "enter" || t === "over") {
            ui.dropDir = dir;
            dragging = dir === null;
          } else if (t === "drop") {
            dragging = false;
            ui.dropDir = null;
            if (dir !== null) ws.addFiles(dir, e.payload.paths);
            else handleDrop(e.payload.paths);
          }
        });
      });
    }
    // First start: open the example project, compiled.
    if (!settings.welcomed && settings.recent.length === 0) ws.openWelcome();
    const params = new URLSearchParams(location.search);
    if (!isTauri && params.has("demo")) ws.openWelcome();

    const beforeUnload = () => {
      if (ws.dirtyCount > 0) ws.saveAll();
    };
    window.addEventListener("beforeunload", beforeUnload);
    return () => {
      unlisten?.();
      ws.dispose();
      window.removeEventListener("beforeunload", beforeUnload);
    };
  });

  const L = settings.layout;
</script>

<svelte:window onkeydown={handleGlobalKey} />

<div class="app">
  <Toolbar />

  {#if ws.project}
    <div class="workspace" bind:this={main}>
      {#if L.treeOpen}
        <aside class="tree" style:width="{L.tree}px">
          <FileTree />
        </aside>
        <Splitter
          dir="x"
          onDrag={(dx) => (L.tree = Math.max(160, Math.min(520, L.tree + dx)))}
          label="Resize file tree"
        />
      {/if}

      <section class="center">
        <div class="editor-pdf">
          <div class="editor" style:flex-basis={L.pdfOpen ? `${(1 - L.pdf) * 100}%` : "100%"}>
            <EditorPane />
          </div>
          {#if L.pdfOpen}
            <Splitter
              dir="x"
              onDrag={(dx) => {
                const w = main?.querySelector(".editor-pdf")?.clientWidth ?? 1000;
                L.pdf = Math.max(0.2, Math.min(0.8, L.pdf - dx / w));
              }}
              label="Resize PDF preview"
            />
            <div class="pdf" style:flex-basis="{L.pdf * 100}%">
              <PdfView />
            </div>
          {/if}
        </div>
        {#if L.problemsOpen}
          <Splitter dir="y" onDrag={(dy) => (L.problems = Math.max(90, Math.min(600, L.problems - dy)))} label="Resize problems panel" />
        {/if}
        <ProblemsPanel />
      </section>
    </div>
  {:else}
    <StartScreen />
  {/if}

  {#if dragging}
    <div class="dropzone">
      <div>Drop a project folder or an Overleaf ZIP</div>
    </div>
  {/if}

  {#if ws.busy}
    <div class="busy" role="status"><span class="spinner"></span>{ws.busy}</div>
  {/if}

  {#if ui.palette}<CommandPalette />{/if}
  {#if ui.settings}<SettingsDialog />{/if}
  {#if ui.newProject}<NewProjectDialog />{/if}
  {#if ui.about}<AboutDialog />{/if}
  {#if mcp.dialog}<McpDialog />{/if}
  <Toasts />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .workspace {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .tree {
    flex: none;
    background: var(--surface);
    border-right: 1px solid var(--border);
    min-width: 0;
    animation: slide-in var(--dur) var(--ease);
  }
  .center {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .editor-pdf {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .editor,
  .pdf {
    min-width: 0;
    flex-grow: 1;
    flex-shrink: 1;
    display: flex;
    flex-direction: column;
  }
  .pdf {
    animation: fade-in var(--dur) var(--ease);
  }
  .dropzone {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: var(--backdrop);
    z-index: 50;
    pointer-events: none;
  }
  .dropzone div {
    padding: 28px 40px;
    border: 2px dashed var(--accent);
    border-radius: 16px;
    background: var(--surface);
    font-size: 16px;
    font-weight: 600;
    color: var(--accent);
  }
  .busy {
    position: fixed;
    left: 50%;
    top: 60px;
    transform: translateX(-50%);
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 14px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: var(--shadow-lg);
    z-index: 40;
    animation: fade-in var(--dur) var(--ease);
  }
  :global(.spinner) {
    width: 14px;
    height: 14px;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex: none;
  }
  @keyframes -global-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes -global-fade-in {
    from {
      opacity: 0;
    }
  }
  @keyframes -global-slide-in {
    from {
      opacity: 0;
      transform: translateX(-8px);
    }
  }
  @keyframes -global-pop-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }
</style>
