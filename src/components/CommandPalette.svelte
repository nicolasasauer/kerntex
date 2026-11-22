<script lang="ts">
  import Modal from "./Modal.svelte";
  import { commands, ui, fmt } from "../lib/commands.svelte";

  let query = $state("");
  let index = $state(0);
  let list: HTMLUListElement | undefined = $state();

  function score(title: string, q: string): number {
    if (!q) return 1;
    const t = title.toLowerCase();
    const s = q.toLowerCase();
    if (t.startsWith(s)) return 3;
    if (t.includes(s)) return 2;
    // fuzzy: all chars in order
    let i = 0;
    for (const c of t) if (c === s[i]) i++;
    return i === s.length ? 1 : 0;
  }

  const results = $derived(
    commands
      .filter((c) => !c.when || c.when())
      .map((c) => ({ c, s: Math.max(score(c.title, query), score(`${c.group} ${c.title}`, query)) }))
      .filter((r) => r.s > 0)
      .sort((a, b) => b.s - a.s)
      .map((r) => r.c),
  );

  $effect(() => {
    void query;
    index = 0;
  });

  $effect(() => {
    list?.querySelector(`[data-i="${index}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function run(i: number) {
    const c = results[i];
    if (!c) return;
    ui.palette = false;
    // Let the palette close before running (focus returns to the app).
    setTimeout(() => c.run(), 0);
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown") index = Math.min(results.length - 1, index + 1);
    else if (e.key === "ArrowUp") index = Math.max(0, index - 1);
    else if (e.key === "Enter") run(index);
    else return;
    e.preventDefault();
  }
</script>

<Modal title="Command palette" top width={580} onClose={() => (ui.palette = false)}>
  <input class="q" placeholder="Type a command…" bind:value={query} onkeydown={key} spellcheck="false" aria-label="Search commands" />
  <ul bind:this={list} role="listbox">
    {#each results as c, i (c.id)}
      <li role="option" aria-selected={i === index}>
        <button class:sel={i === index} data-i={i} onclick={() => run(i)} onpointermove={() => (index = i)}>
          <span class="group">{c.group}</span>
          <span class="t">{c.title}</span>
          {#if c.shortcut}<kbd>{fmt(c.shortcut)}</kbd>{/if}
        </button>
      </li>
    {:else}
      <li class="none muted">No matching command</li>
    {/each}
  </ul>
</Modal>

<style>
  .q {
    width: 100%;
    height: 48px;
    padding: 0 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    font-size: 15px;
    outline: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: 50vh;
    overflow: auto;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: none;
    border-radius: 7px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  button.sel {
    background: var(--accent-soft);
  }
  button:active {
    background: var(--pressed);
  }
  .group {
    width: 86px;
    font-size: 11px;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex: none;
  }
  .t {
    flex: 1;
  }
  .none {
    padding: 12px;
  }
</style>
