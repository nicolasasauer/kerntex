<script lang="ts">
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import X from "@lucide/svelte/icons/x";
  let { title, onClose, width = 520, children, top = false }: { title: string; onClose: () => void; width?: number; children: Snippet; top?: boolean } = $props();
  let box: HTMLDivElement | undefined = $state();
  let prev: Element | null = null;

  onMount(() => {
    prev = document.activeElement;
    (box?.querySelector("input, select, button:not(.x)") as HTMLElement | null)?.focus();
    return () => (prev as HTMLElement | null)?.focus?.();
  });

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
    if (e.key === "Tab" && box) {
      // Keep focus inside the dialog.
      const f = [...box.querySelectorAll<HTMLElement>("button, input, select, textarea, [tabindex]:not([tabindex='-1'])")].filter((el) => !el.hasAttribute("disabled"));
      if (f.length === 0) return;
      const i = f.indexOf(document.activeElement as HTMLElement);
      if (e.shiftKey && i <= 0) {
        f[f.length - 1].focus();
        e.preventDefault();
      } else if (!e.shiftKey && i === f.length - 1) {
        f[0].focus();
        e.preventDefault();
      }
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" class:top onpointerdown={(e) => e.target === e.currentTarget && onClose()} onkeydown={key}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} style:width="{width}px" bind:this={box}>
    {#if !top}
      <div class="head">
        <h2>{title}</h2>
        <button class="icon-btn x" aria-label="Close" title="Close (Esc)" onclick={onClose}><X size={16} /></button>
      </div>
    {/if}
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--backdrop);
    animation: fade-in var(--dur) var(--ease);
  }
  .backdrop.top {
    align-items: flex-start;
    padding-top: 12vh;
  }
  .dialog {
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    animation: pop-in 180ms var(--ease);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 14px 6px 20px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
  }
</style>
