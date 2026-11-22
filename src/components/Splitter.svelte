<script lang="ts">
  let { dir, onDrag, label }: { dir: "x" | "y"; onDrag: (delta: number) => void; label: string } = $props();
  let active = $state(false);
  let last = 0;

  function down(e: PointerEvent) {
    active = true;
    last = dir === "x" ? e.clientX : e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!active) return;
    const pos = dir === "x" ? e.clientX : e.clientY;
    onDrag(pos - last);
    last = pos;
  }
  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 40 : 10;
    if ((dir === "x" && e.key === "ArrowLeft") || (dir === "y" && e.key === "ArrowUp")) onDrag(-step);
    else if ((dir === "x" && e.key === "ArrowRight") || (dir === "y" && e.key === "ArrowDown")) onDrag(step);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter {dir}"
  class:active
  role="separator"
  aria-orientation={dir === "x" ? "vertical" : "horizontal"}
  aria-label={label}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={() => (active = false)}
  onkeydown={key}
></div>

<style>
  .splitter {
    flex: none;
    position: relative;
    z-index: 5;
    background: var(--border);
    transition: background var(--dur) var(--ease);
  }
  .splitter.x {
    width: 1px;
    cursor: col-resize;
  }
  .splitter.y {
    height: 1px;
    cursor: row-resize;
  }
  .splitter::after {
    content: "";
    position: absolute;
    inset: -3px;
  }
  .splitter.x::after {
    inset: 0 -3px;
  }
  .splitter.y::after {
    inset: -3px 0;
  }
  .splitter:hover,
  .splitter.active,
  .splitter:focus-visible {
    background: var(--accent);
    outline: none;
  }
</style>
