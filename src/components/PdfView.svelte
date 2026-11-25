<script lang="ts">
  import { onMount, untrack } from "svelte";
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import ZoomOut from "@lucide/svelte/icons/zoom-out";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import FileWarning from "@lucide/svelte/icons/file-warning";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { api, joinPath, revealInFolder, toAppError } from "../lib/api";
  import type { PDFDocumentProxy } from "pdfjs-dist";
  import { loadPdfjs } from "../lib/pdfjs";

  /** A document point (page + fraction of that page) and where it sits in the viewport. */
  type Anchor = { index: number; fx: number; fy: number; px: number; py: number };
  /** CSS transform on the current layer: scale `k` around (ox, oy), then shift by (dx, dy). */
  type Transform = { ox: number; oy: number; k: number; dx: number; dy: number };

  let scroller: HTMLDivElement | undefined = $state();
  let doc: PDFDocumentProxy | null = null;
  let pageCount = $state(0);
  let currentPage = $state(1);
  let error: string | null = $state(null);
  let loading = $state(false);
  let sizes: { w: number; h: number }[] = [];
  let layer: HTMLDivElement | null = null;
  let layerScale = 1;
  let observer: IntersectionObserver | null = null;
  let generation = 0;

  // Zoom gesture state, kept until the re-rendered layer is swapped in.
  let anchor: Anchor | null = null;
  let pendingZoom: number | null = null;
  let xf: Transform = { ox: 0, oy: 0, k: 1, dx: 0, dy: 0 };
  let sizer: HTMLDivElement | null = null;
  let buildTimer: ReturnType<typeof setTimeout> | undefined;

  const GAP = 14;
  const PAD = 16;
  const MIN_ZOOM = 0.25;
  const MAX_ZOOM = 5;
  /** Wait this long after the last zoom step before re-rendering sharply. */
  const ZOOM_SETTLE_MS = 140;

  function computeScale(): number {
    if (!scroller || sizes.length === 0) return 1;
    const z = settings.pdfZoom;
    const first = sizes[0];
    const availW = scroller.clientWidth - PAD * 2;
    if (z === "fit-width") return Math.max(0.2, availW / first.w);
    if (z === "fit-page") return Math.max(0.2, Math.min(availW / first.w, (scroller.clientHeight - PAD * 2) / first.h));
    return z;
  }

  async function renderPage(canvas: HTMLCanvasElement, index: number, gen: number, s: number) {
    if (!doc || gen !== generation) return;
    const page = await doc.getPage(index + 1);
    if (gen !== generation) return;
    const dpr = window.devicePixelRatio || 1;
    const viewport = page.getViewport({ scale: s * dpr });
    canvas.width = Math.floor(viewport.width);
    canvas.height = Math.floor(viewport.height);
    await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport }).promise.catch(() => {});
    canvas.dataset.rendered = "1";
  }

  function applyTransform() {
    if (!layer) return;
    layer.style.transformOrigin = `${xf.ox}px ${xf.oy}px`;
    layer.style.transform = `translate(${xf.dx}px, ${xf.dy}px) scale(${xf.k})`;
  }

  /** Maps a viewport point to untransformed coordinates of the current layer. */
  function toLayer(px: number, py: number): { x: number; y: number } {
    const vx = scroller!.scrollLeft + px;
    const vy = scroller!.scrollTop + py;
    return { x: xf.ox + (vx - xf.dx - xf.ox) / xf.k, y: xf.oy + (vy - xf.dy - xf.oy) / xf.k };
  }

  /** Remembers which spot of which page is under viewport point (px, py). */
  function captureAnchor(px: number, py: number): Anchor | null {
    if (!scroller || !layer) return null;
    const pages = layer.querySelectorAll<HTMLCanvasElement>("canvas.page");
    if (pages.length === 0) return null;
    const u = toLayer(px, py);
    let c = pages[0];
    for (const p of pages) {
      if (p.offsetTop <= u.y) c = p;
      else break;
    }
    return {
      index: Number(c.dataset.index),
      fx: (u.x - c.offsetLeft) / c.offsetWidth,
      fy: (u.y - c.offsetTop) / c.offsetHeight,
      px,
      py,
    };
  }

  /** Builds a fresh page layer; swaps it in once visible pages are drawn (no flicker). */
  async function buildLayer() {
    if (!scroller || !doc) return;
    clearTimeout(buildTimer);
    const gen = ++generation;
    const s = computeScale();
    // Without a zoom gesture (reload, resize) keep the top-left spot of the view in place.
    const a = anchor ?? captureAnchor(0, 0);
    const next = document.createElement("div");
    next.className = "layer incoming";
    next.style.padding = `${PAD}px`;
    next.style.gap = `${GAP}px`;
    const canvases: HTMLCanvasElement[] = [];
    for (let i = 0; i < sizes.length; i++) {
      const c = document.createElement("canvas");
      c.className = "page";
      c.style.width = `${Math.floor(sizes[i].w * s)}px`;
      c.style.height = `${Math.floor(sizes[i].h * s)}px`;
      c.dataset.index = String(i);
      next.appendChild(c);
      canvases.push(c);
    }
    scroller.appendChild(next);

    // Scroll so the anchored spot lands where it was on screen; shift the old
    // layer by the same amount so it stays visually still until the swap.
    if (a && canvases.length > 0) {
      const sameIndex = a.index < canvases.length;
      const c = canvases[sameIndex ? a.index : canvases.length - 1];
      const left0 = scroller.scrollLeft;
      const top0 = scroller.scrollTop;
      scroller.scrollLeft = c.offsetLeft + a.fx * c.offsetWidth - a.px;
      scroller.scrollTop = c.offsetTop + (sameIndex ? a.fy : 1) * c.offsetHeight - a.py;
      const dx = scroller.scrollLeft - left0;
      const dy = scroller.scrollTop - top0;
      if (layer && (dx || dy)) {
        xf.dx += dx;
        xf.dy += dy;
        applyTransform();
      }
    }

    observer?.disconnect();
    observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const c = e.target as HTMLCanvasElement;
          if (e.isIntersecting && !c.dataset.rendered) renderPage(c, Number(c.dataset.index), gen, s);
        }
        updateCurrentPage();
      },
      { root: scroller, rootMargin: "600px 0px" },
    );
    // Render what's visible now, then fade the new layer in.
    const top = scroller.scrollTop, bottom = top + scroller.clientHeight;
    const visible = canvases.filter((c) => c.offsetTop + c.offsetHeight >= top - 200 && c.offsetTop <= bottom + 200);
    await Promise.all(visible.map((c) => renderPage(c, Number(c.dataset.index), gen, s)));
    if (gen !== generation) {
      next.remove();
      return;
    }
    canvases.forEach((c) => observer!.observe(c));
    const old = layer;
    layer = next;
    layerScale = s;
    xf = { ox: 0, oy: 0, k: 1, dx: 0, dy: 0 };
    anchor = null;
    pendingZoom = null;
    sizer?.remove();
    sizer = null;
    next.classList.remove("incoming");
    if (old) {
      old.classList.add("outgoing");
      setTimeout(() => old.remove(), 260);
    }
    updateCurrentPage();
  }

  function scheduleBuild(delay: number) {
    clearTimeout(buildTimer);
    buildTimer = setTimeout(() => void buildLayer(), delay);
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
      const old = doc;
      doc = next;
      sizes = nextSizes;
      pageCount = next.numPages;
      error = null;
      await buildLayer();
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
    untrack(() => {
      if (!doc || !scroller) return;
      // Zoom changed outside a gesture (e.g. the dropdown): keep the viewport centre in place.
      if (!anchor) anchor = captureAnchor(scroller.clientWidth / 2, scroller.clientHeight / 2);
      generation++;
      scheduleBuild(pendingZoom != null ? ZOOM_SETTLE_MS : 0);
    });
  });

  onMount(() => {
    let t: ReturnType<typeof setTimeout>;
    const ro = new ResizeObserver(() => {
      clearTimeout(t);
      t = setTimeout(() => {
        if (doc && typeof settings.pdfZoom === "string" && Math.abs(computeScale() - layerScale) > 0.01) {
          void buildLayer();
        }
      }, 150);
    });
    ro.observe(scroller!);
    return () => {
      ro.disconnect();
      clearTimeout(buildTimer);
      observer?.disconnect();
      doc?.destroy();
    };
  });

  /**
   * Zooms around a viewport point (the cursor, or the centre for the buttons).
   * The current layer is scaled with CSS right away; the sharp re-render
   * follows once the gesture settles.
   */
  function zoom(factor: number, clientX?: number, clientY?: number) {
    if (!scroller || !layer) return;
    const base = pendingZoom ?? computeScale();
    const target = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, base * factor));
    if (Math.abs(target - base) < 1e-4) return;
    pendingZoom = target;
    generation++; // a rebuild already in flight would snap back to the old zoom

    if (!anchor) {
      const r = scroller.getBoundingClientRect();
      const px = clientX != null ? clientX - r.left : scroller.clientWidth / 2;
      const py = clientY != null ? clientY - r.top : scroller.clientHeight / 2;
      anchor = captureAnchor(px, py);
      // Move the transform origin under the anchor without moving anything on screen.
      const u = toLayer(px, py);
      xf.dx += (1 - xf.k) * (xf.ox - u.x);
      xf.dy += (1 - xf.k) * (xf.oy - u.y);
      xf.ox = u.x;
      xf.oy = u.y;
      // Hold the scroll extent while the layer shrinks visually, so scrollTop isn't clamped.
      if (!sizer) {
        sizer = document.createElement("div");
        sizer.className = "sizer";
        sizer.style.width = `${layer.offsetWidth}px`;
        sizer.style.height = `${layer.offsetHeight}px`;
        scroller.appendChild(sizer);
      }
    }
    xf.k = target / layerScale;
    applyTransform();

    settings.pdfZoom = Math.round(target * 100) / 100;
    scheduleBuild(ZOOM_SETTLE_MS);
  }

  function onZoomSelect(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    settings.pdfZoom = v === "fit-width" || v === "fit-page" ? v : Number(v);
  }

  function wheel(e: WheelEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    // Proportional to the wheel delta: a mouse notch is ~16 %, trackpad pinches stay fine-grained.
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * 100 : e.deltaY;
    zoom(Math.max(0.5, Math.min(2, Math.exp(-dy * 0.0015))), e.clientX, e.clientY);
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
  .scroller :global(.sizer) {
    position: absolute;
    top: 0;
    left: 0;
    visibility: hidden;
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
