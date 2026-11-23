<script lang="ts">
  import { onMount } from "svelte";
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import ZoomOut from "@lucide/svelte/icons/zoom-out";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import FileWarning from "@lucide/svelte/icons/file-warning";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { api, joinPath, revealInFolder, toAppError } from "../lib/api";
  import type { PDFDocumentProxy } from "pdfjs-dist";
  import { loadPdfjs } from "../lib/pdfjs";

  let scroller: HTMLDivElement | undefined = $state();
  let doc: PDFDocumentProxy | null = null;
  let pageCount = $state(0);
  let currentPage = $state(1);
  let scale = $state(1);
  let error: string | null = $state(null);
  let loading = $state(false);
  let sizes: { w: number; h: number }[] = [];
  let layer: HTMLDivElement | null = null;
  let observer: IntersectionObserver | null = null;
  let generation = 0;

  const GAP = 14;
  const PAD = 16;

  function computeScale(): number {
    if (!scroller || sizes.length === 0) return 1;
    const z = settings.pdfZoom;
    const first = sizes[0];
    const availW = scroller.clientWidth - PAD * 2;
    if (z === "fit-width") return Math.max(0.2, availW / first.w);
    if (z === "fit-page") return Math.max(0.2, Math.min(availW / first.w, (scroller.clientHeight - PAD * 2) / first.h));
    return z;
  }

  async function renderPage(canvas: HTMLCanvasElement, index: number, gen: number) {
    if (!doc || gen !== generation) return;
    const page = await doc.getPage(index + 1);
    if (gen !== generation) return;
    const dpr = window.devicePixelRatio || 1;
    const viewport = page.getViewport({ scale: scale * dpr });
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport }).promise.catch(() => {});
    canvas.dataset.rendered = "1";
  }

  /** Builds a fresh page layer; swaps it in once visible pages are drawn (no flicker). */
  async function buildLayer(keepRatio: number | null) {
    if (!scroller || !doc) return;
    const gen = ++generation;
    scale = computeScale();
    const next = document.createElement("div");
    next.className = "layer incoming";
    next.style.padding = `${PAD}px`;
    next.style.gap = `${GAP}px`;
    const canvases: HTMLCanvasElement[] = [];
    for (let i = 0; i < sizes.length; i++) {
      const c = document.createElement("canvas");
      c.className = "page";
      c.style.width = `${Math.floor(sizes[i].w * scale)}px`;
      c.style.height = `${Math.floor(sizes[i].h * scale)}px`;
      c.dataset.index = String(i);
      next.appendChild(c);
      canvases.push(c);
    }
    scroller.appendChild(next);
    if (keepRatio != null) scroller.scrollTop = keepRatio * scroller.scrollHeight;

    observer?.disconnect();
    observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const c = e.target as HTMLCanvasElement;
          if (e.isIntersecting && !c.dataset.rendered) renderPage(c, Number(c.dataset.index), gen);
        }
        updateCurrentPage();
      },
      { root: scroller, rootMargin: "600px 0px" },
    );
    // Render what's visible now, then fade the new layer in.
    const top = scroller.scrollTop, bottom = top + scroller.clientHeight;
    const visible = canvases.filter((c) => c.offsetTop + c.offsetHeight >= top - 200 && c.offsetTop <= bottom + 200);
    await Promise.all(visible.map((c) => renderPage(c, Number(c.dataset.index), gen)));
    if (gen !== generation) {
      next.remove();
      return;
    }
    canvases.forEach((c) => observer!.observe(c));
    const old = layer;
    layer = next;
    next.classList.remove("incoming");
    if (old) {
      old.classList.add("outgoing");
      setTimeout(() => old.remove(), 260);
    }
  }

  function updateCurrentPage() {
    if (!scroller || !layer) return;
    const mid = scroller.scrollTop + scroller.clientHeight / 3;
    const pages = layer.querySelectorAll<HTMLCanvasElement>("canvas.page");
    let n = 1;
    for (const p of pages) {
      if (p.offsetTop <= mid) n = Number(p.dataset.index) + 1;
      else break;
    }
    currentPage = n;
  }

  async function load() {
    const p = ws.project;
    const main = ws.mainFile;
    if (!p || !main) return;
    loading = true;
    try {
      const [lib, bytes] = await Promise.all([loadPdfjs(), api.readPdf(p.root, main)]);
      const data = new Uint8Array(bytes);
      const next = await lib.getDocument({ data }).promise;
      const nextSizes: { w: number; h: number }[] = [];
      for (let i = 1; i <= next.numPages; i++) {
        const vp = (await next.getPage(i)).getViewport({ scale: 1 });
        nextSizes.push({ w: vp.width, h: vp.height });
      }
      const ratio = scroller && layer ? scroller.scrollTop / Math.max(1, scroller.scrollHeight) : null;
      const old = doc;
      doc = next;
      sizes = nextSizes;
      pageCount = next.numPages;
      error = null;
      await buildLayer(ratio);
      old?.destroy();
    } catch (e) {
      const err = toAppError(e);
      // No PDF yet is normal before the first successful compile.
      error = err.source === "fileSystem" ? null : err.title;
      if (err.source !== "fileSystem") console.error(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void ws.pdfVersion;
    load();
  });

  $effect(() => {
    void settings.pdfZoom;
    if (doc) buildLayer(scroller ? scroller.scrollTop / Math.max(1, scroller.scrollHeight) : null);
  });

  onMount(() => {
    let t: ReturnType<typeof setTimeout>;
    const ro = new ResizeObserver(() => {
      clearTimeout(t);
      t = setTimeout(() => {
        if (doc && typeof settings.pdfZoom === "string" && Math.abs(computeScale() - scale) > 0.01) {
          buildLayer(scroller!.scrollTop / Math.max(1, scroller!.scrollHeight));
        }
      }, 150);
    });
    ro.observe(scroller!);
    return () => {
      ro.disconnect();
      observer?.disconnect();
      doc?.destroy();
    };
  });

  function zoom(factor: number) {
    settings.pdfZoom = Math.round(Math.max(0.25, Math.min(5, scale * factor)) * 100) / 100;
  }

  function onZoomSelect(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    settings.pdfZoom = v === "fit-width" || v === "fit-page" ? v : Number(v);
  }

  function wheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoom(e.deltaY < 0 ? 1.1 : 1 / 1.1);
    }
  }

  function reveal() {
    const p = ws.project;
    const main = ws.mainFile;
    if (!p || !main) return;
    const stem = main.split("/").pop()!.replace(/\.tex$/i, "");
    revealInFolder(joinPath(p.root, `.kerntex/out/${stem}.pdf`));
  }

  const zoomLabel = $derived(typeof settings.pdfZoom === "number" ? `${Math.round(settings.pdfZoom * 100)}%` : null);
</script>

<div class="pdfview">
  <div class="bar">
    <div class="group">
      <button class="icon-btn" title="Zoom out (Ctrl+scroll)" aria-label="Zoom out" onclick={() => zoom(1 / 1.15)}><ZoomOut size={16} /></button>
      <select class="zoom" value={typeof settings.pdfZoom === "number" ? String(settings.pdfZoom) : settings.pdfZoom} onchange={onZoomSelect} aria-label="Zoom">
        <option value="fit-width">Fit width</option>
        <option value="fit-page">Fit page</option>
        {#if zoomLabel && ![0.5, 0.75, 1, 1.25, 1.5, 2].includes(settings.pdfZoom as number)}
          <option value={String(settings.pdfZoom)}>{zoomLabel}</option>
        {/if}
        {#each [0.5, 0.75, 1, 1.25, 1.5, 2] as z}<option value={String(z)}>{z * 100}%</option>{/each}
      </select>
      <button class="icon-btn" title="Zoom in (Ctrl+scroll)" aria-label="Zoom in" onclick={() => zoom(1.15)}><ZoomIn size={16} /></button>
    </div>
    <div class="group">
      {#if pageCount > 0}<span class="pages muted">Page {currentPage} / {pageCount}</span>{/if}
      {#if loading}<span class="spinner" title="Loading PDF"></span>{/if}
      <button class="icon-btn" title="Show PDF in file manager" aria-label="Show PDF in file manager" onclick={reveal} disabled={pageCount === 0}><FolderSearch size={16} /></button>
    </div>
  </div>
  <div class="scroller" bind:this={scroller} onscroll={updateCurrentPage} onwheel={wheel}>
    {#if pageCount === 0 && !loading}
      <div class="empty">
        {#if error}
          <FileWarning size={30} strokeWidth={1.4} />
          <p>Couldn't display the PDF</p>
          <p class="muted">{error}</p>
        {:else if ws.status.state === "running"}
          <span class="spinner big"></span>
          <p>Creating your PDF…</p>
        {:else}
          <p>No PDF yet</p>
          <p class="muted">Compile to see your document here.</p>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .pdfview {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--pdf-bg);
  }
  .bar {
    flex: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 36px;
    padding: 0 6px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .zoom {
    height: 26px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--input-bg);
    cursor: pointer;
  }
  .pages {
    font-variant-numeric: tabular-nums;
    padding: 0 6px;
  }
  .scroller {
    flex: 1;
    position: relative;
    overflow: auto;
    min-height: 0;
  }
  .scroller :global(.layer) {
    position: absolute;
    top: 0;
    left: 0;
    min-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    transition: opacity 220ms var(--ease);
  }
  .scroller :global(.layer.incoming) {
    opacity: 0;
    pointer-events: none;
  }
  .scroller :global(.layer.outgoing) {
    opacity: 0;
    pointer-events: none;
  }
  .scroller :global(canvas.page) {
    flex: none;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18), 0 4px 14px rgba(0, 0, 0, 0.08);
    border-radius: 2px;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    color: var(--text-muted);
    text-align: center;
    padding: 20px;
  }
  .empty p {
    margin: 4px 0 0;
  }
  .spinner.big {
    width: 22px;
    height: 22px;
  }
</style>
