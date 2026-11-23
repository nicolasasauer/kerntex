<script lang="ts">
  import { onMount } from "svelte";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import Copy from "@lucide/svelte/icons/copy";
  import FileInput from "@lucide/svelte/icons/file-input";
  import FileWarning from "@lucide/svelte/icons/file-warning";
  import { ws } from "../lib/workspace.svelte";
  import { api, joinPath, revealInFolder, toAppError } from "../lib/api";
  import { loadPdfjs } from "../lib/pdfjs";
  import { toast } from "../lib/toasts.svelte";

  let { path }: { path: string } = $props();

  const MIME: Record<string, string> = {
    png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", svg: "image/svg+xml", webp: "image/webp", bmp: "image/bmp",
  };
  const ext = $derived(path.split(".").pop()!.toLowerCase());
  const name = $derived(path.split("/").pop()!);

  let url: string | null = $state(null);
  let canvas: HTMLCanvasElement | undefined = $state();
  let size = $state(0);
  let dims: string | null = $state(null);
  let pages = $state(0);
  let error: string | null = $state(null);
  let loading = $state(true);
  let actual = $state(false);

  const snippet = $derived(`\\includegraphics[width=0.8\\linewidth]{${ws.graphicsPath(path)}}`);
  const target = $derived(ws.lastTextTab && ws.tabs.some((t) => t.path === ws.lastTextTab) && ws.lastTextTab.endsWith(".tex") ? ws.lastTextTab : null);

  onMount(() => {
    let revoke: string | null = null;
    (async () => {
      const p = ws.project;
      if (!p) return;
      try {
        const bytes = await api.readBinary(p.root, path);
        size = bytes.byteLength;
        if (ext === "pdf") {
          const lib = await loadPdfjs();
          const doc = await lib.getDocument({ data: new Uint8Array(bytes) }).promise;
          pages = doc.numPages;
          const page = await doc.getPage(1);
          const base = page.getViewport({ scale: 1 });
          dims = `${Math.round((base.width / 72) * 25.4)} × ${Math.round((base.height / 72) * 25.4)} mm`;
          const scale = Math.min(3, 1400 / base.width) * (window.devicePixelRatio || 1);
          const vp = page.getViewport({ scale });
          loading = false;
          await Promise.resolve();
          if (!canvas) return;
          canvas.width = Math.floor(vp.width);
          canvas.height = Math.floor(vp.height);
          canvas.style.aspectRatio = `${vp.width} / ${vp.height}`;
          await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport: vp }).promise;
          doc.destroy();
        } else {
          revoke = url = URL.createObjectURL(new Blob([bytes], { type: MIME[ext] ?? "application/octet-stream" }));
          loading = false;
        }
      } catch (e) {
        error = toAppError(e).title;
        loading = false;
      }
    })();
    return () => {
      if (revoke) URL.revokeObjectURL(revoke);
    };
  });

  function onLoad(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    if (img.naturalWidth) dims = `${img.naturalWidth} × ${img.naturalHeight} px`;
  }

  function fmtSize(n: number) {
    return n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${(n / 1024).toFixed(0)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
  }

  async function copySnippet() {
    try {
      await navigator.clipboard.writeText(snippet);
      toast({ kind: "success", source: "KernTeX", title: "Copied", body: snippet }, 2000);
    } catch {
      toast({ kind: "error", source: "KernTeX", title: "Couldn't copy to the clipboard", body: snippet });
    }
  }
</script>

<div class="preview">
  <div class="bar">
    <div class="meta">
      <b>{name}</b>
      <span class="muted">{[ext.toUpperCase(), size ? fmtSize(size) : null, dims, pages > 1 ? `${pages} pages (showing page 1)` : null].filter(Boolean).join(" · ")}</span>
    </div>
    <div class="actions">
      {#if target}
        <button class="btn primary" onclick={() => ws.insertFigure(path, target)} title="Insert a figure environment at the cursor in {target}">
          <FileInput size={14} /> Insert into {target.split("/").pop()}
        </button>
      {/if}
      <button class="btn" onclick={copySnippet} title="Copy the \includegraphics command"><Copy size={14} /> Copy <code>\includegraphics</code></button>
      <button class="icon-btn" title="Show in file manager" aria-label="Show in file manager" onclick={() => ws.project && revealInFolder(joinPath(ws.project.root, path))}>
        <FolderSearch size={16} />
      </button>
    </div>
  </div>
  <div class="stage" class:actual>
    {#if error}
      <div class="msg"><FileWarning size={30} strokeWidth={1.4} /><p>Couldn't show this file</p><p class="muted">{error}</p></div>
    {:else if loading}
      <span class="spinner"></span>
    {:else if ext === "pdf"}
      <canvas bind:this={canvas} class="pdfpage"></canvas>
    {:else if url}
      <button class="imgbtn" onclick={() => (actual = !actual)} title={actual ? "Fit to window" : "Show actual size"}>
        <img src={url} alt={name} onload={onLoad} />
      </button>
    {/if}
  </div>
</div>

<style>
  .preview {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    animation: fade-in var(--dur) var(--ease);
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-wrap: wrap;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .meta b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta span {
    font-size: 12px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  code {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .stage {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    place-items: center;
    padding: 24px;
    /* Checkerboard so transparent areas are visible. */
    background-color: var(--surface-2);
    background-image: linear-gradient(45deg, var(--hover) 25%, transparent 25%), linear-gradient(-45deg, var(--hover) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, var(--hover) 75%), linear-gradient(-45deg, transparent 75%, var(--hover) 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0;
  }
  .imgbtn {
    border: none;
    padding: 0;
    background: transparent;
    cursor: zoom-in;
    max-width: 100%;
    max-height: 100%;
    display: grid;
  }
  .stage.actual .imgbtn {
    cursor: zoom-out;
    max-width: none;
    max-height: none;
  }
  img {
    max-width: 100%;
    max-height: calc(100vh - 260px);
    object-fit: contain;
    box-shadow: var(--shadow-lg);
  }
  .stage.actual img {
    max-width: none;
    max-height: none;
  }
  .pdfpage {
    max-width: 100%;
    max-height: calc(100vh - 260px);
    background: #fff;
    box-shadow: var(--shadow-lg);
  }
  .msg {
    display: flex;
    flex-direction: column;
    align-items: center;
    color: var(--text-muted);
  }
  .msg p {
    margin: 4px 0 0;
  }
</style>
