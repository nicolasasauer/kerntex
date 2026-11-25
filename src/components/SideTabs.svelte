<script lang="ts">
  import { settings } from "../lib/settings.svelte";
  import { git } from "../lib/git.svelte";
  import { fmt } from "../lib/commands.svelte";

  const L = settings.layout;
</script>

<div class="tabs" role="tablist" aria-label="Sidebar">
  <button role="tab" class="tab" class:active={L.side === "files"} aria-selected={L.side === "files"} title="Files ({fmt('Mod+B')} toggles the sidebar)" onclick={() => (L.side = "files")}>
    Files
  </button>
  <button role="tab" class="tab" class:active={L.side === "git"} aria-selected={L.side === "git"} title="Git: commit, push, pull ({fmt('Mod+Shift+G')})" onclick={() => (L.side = "git")}>
    Git
    {#if git.changeCount > 0}<span class="badge" aria-label="{git.changeCount} changes">{git.changeCount > 99 ? "99+" : git.changeCount}</span>{/if}
  </button>
</div>

<style>
  .tabs {
    display: flex;
    gap: 2px;
    height: 100%;
    align-items: stretch;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: color var(--dur) var(--ease), border-color var(--dur) var(--ease);
  }
  .tab:hover {
    color: var(--text);
  }
  .tab:active {
    color: var(--text);
    background: var(--pressed);
  }
  .tab.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .badge {
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 10px;
    line-height: 16px;
    text-align: center;
    letter-spacing: 0;
  }
</style>
