<script lang="ts" module>
  export interface MenuItem {
    label: string;
    icon?: any;
    shortcut?: string;
    danger?: boolean;
    disabled?: boolean;
    /** Tooltip. */
    title?: string;
    run: () => void;
  }
  export type MenuEntry = MenuItem | "separator" | { heading: string };
</script>

<script lang="ts">
  import { onMount } from "svelte";
  let { x, y, items, onClose }: { x: number; y: number; items: MenuEntry[]; onClose: () => void } = $props();
  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  onMount(() => {
    const r = el!.getBoundingClientRect();
    pos = { left: Math.min(x, innerWidth - r.width - 8), top: Math.min(y, innerHeight - r.height - 8) };
    (el!.querySelector("button:not(:disabled)") as HTMLElement | null)?.focus();
  });

  function key(e: KeyboardEvent) {
    const buttons = [...el!.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") buttons[(i + 1) % buttons.length]?.focus();
    else if (e.key === "ArrowUp") buttons[(i - 1 + buttons.length) % buttons.length]?.focus();
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onpointerdown={onClose} oncontextmenu={(e) => { e.preventDefault(); onClose(); }}></div>
<div class="menu" role="menu" tabindex="-1" bind:this={el} style:left="{pos.left}px" style:top="{pos.top}px" onkeydown={key}>
  {#each items as item}
    {#if item === "separator"}
      <div class="sep" role="separator"></div>
    {:else if "heading" in item}
      <div class="heading" role="presentation">{item.heading}</div>
    {:else}
      <button role="menuitem" title={item.title} class:danger={item.danger} disabled={item.disabled} onclick={() => { onClose(); item.run(); }}>
        <span class="icon">{#if item.icon}<item.icon size={15} />{/if}</span>
        <span class="label">{item.label}</span>
        {#if item.shortcut}<span class="sc">{item.shortcut}</span>{/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
  }
  .menu {
    position: fixed;
    z-index: 61;
    min-width: 210px;
    max-width: 340px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 5px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow-lg);
    animation: pop-in 120ms var(--ease);
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  button:hover,
  button:focus-visible {
    background: var(--accent-soft);
    outline: none;
  }
  button:active {
    background: var(--pressed);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
    background: transparent;
  }
  .danger {
    color: var(--danger);
  }
  .icon {
    width: 16px;
    display: grid;
    place-items: center;
    color: var(--text-muted);
  }
  .danger .icon {
    color: var(--danger);
  }
  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .heading {
    padding: 4px 8px 2px;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .sc {
    color: var(--text-faint);
    font-size: 12px;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
