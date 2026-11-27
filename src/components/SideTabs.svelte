<script lang="ts">
  import Files from "@lucide/svelte/icons/files";
  import { settings } from "../lib/settings.svelte";
  import { git } from "../lib/git.svelte";
  import { fmt } from "../lib/commands.svelte";

  const L = settings.layout;
  /** Tab names only when the sidebar has room next to the panel's buttons; otherwise icons with tooltips. */
  const labels = $derived(L.tree >= 290);

  // Push / pull in progress: dots flow along the branch (up = out, down = in).
  const flow = $derived(git.op?.direction ?? null);

  // More changed files than before: the badge hops once and the branch tip lights up.
  // Not on the first status of a project, and not when the number goes down.
  let pop = $state(0);
  let prev: number | null = null;
  $effect(() => {
    const repo = git.status?.repo ?? false;
    const n = git.changeCount;
    if (!repo) {
      prev = null;
      return;
    }
    if (prev !== null && n > prev) pop++;
    prev = n;
  });
</script>

<div class="tabs" role="tablist" aria-label="Sidebar">
  <button role="tab" class="tab" class:active={L.side === "files"} aria-selected={L.side === "files"} title="Files ({fmt('Mod+B')} toggles the sidebar)" aria-label={labels ? undefined : "Files"} onclick={() => (L.side = "files")}>
    <Files size={14} aria-hidden="true" />
    {#if labels}Files{/if}
  </button>
  <button role="tab" class="tab" class:active={L.side === "git"} aria-selected={L.side === "git"} title="Git: commit, push, pull ({fmt('Mod+Shift+G')})" aria-label={labels ? undefined : `Git${git.changeCount > 0 ? `, ${git.changeCount} changes` : ""}`} onclick={() => (L.side = "git")}>
    {#key pop}
      <!-- Lucide's git-branch, inline so the tip and the branch line can be animated. -->
      <svg class="branch" class:pop={pop > 0} class:flow={flow} class:pull={flow === "pull"} viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M15 6a9 9 0 0 0-9 9V3" />
        <circle class="tip" cx="18" cy="6" r="3" />
        <circle cx="6" cy="18" r="3" />
      </svg>
    {/key}
    {#if labels}Git{/if}
    {#if git.changeCount > 0}
      {#key pop}<span class="badge" class:pop={pop > 0} aria-label="{git.changeCount} changes">{git.changeCount > 99 ? "99+" : git.changeCount}</span>{/key}
    {/if}
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
  .tab :global(svg) {
    flex: none;
  }
  /* Push / pull running */
  .tab:has(.branch.flow) {
    color: var(--accent);
  }
  .branch.flow path {
    stroke-dasharray: 2.5 3.5;
    animation: git-flow 0.7s linear infinite;
  }
  .branch.flow.pull path {
    animation-direction: reverse;
  }
  @keyframes git-flow {
    to {
      stroke-dashoffset: -12;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    /* Only the colour shows that Git is busy. */
    .branch.flow path {
      stroke-dasharray: none;
    }
  }
  /* New changes */
  .branch.pop .tip {
    animation: git-tip 1s ease-out;
  }
  @keyframes git-tip {
    from {
      fill: var(--accent);
      stroke: var(--accent);
    }
    to {
      fill: transparent;
    }
  }
  .badge.pop {
    animation: git-pop 0.45s cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  @keyframes git-pop {
    40% {
      transform: scale(1.35);
    }
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
