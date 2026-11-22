<script lang="ts">
  import Modal from "./Modal.svelte";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import { ui } from "../lib/commands.svelte";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { pickFolder, parentDir } from "../lib/api";
  import type { Template } from "../lib/types";

  const templates: { id: Template; name: string; desc: string }[] = [
    { id: "article", name: "Article", desc: "Paper or short report with abstract" },
    { id: "report", name: "Thesis / report", desc: "Chapters, table of contents" },
    { id: "beamer", name: "Presentation", desc: "Beamer slides" },
    { id: "letter", name: "Letter", desc: "Formal letter" },
    { id: "blank", name: "Blank", desc: "Just the essentials" },
  ];

  let template: Template = $state("article");
  let name = $state("My document");
  let parent = $state(settings.recent[0] ? parentDir(settings.recent[0].root) : "");
  let creating = $state(false);

  async function choose() {
    const dir = await pickFolder("Where should the project folder be created?");
    if (dir) parent = dir;
  }

  async function create() {
    if (!parent) {
      await choose();
      if (!parent) return;
    }
    creating = true;
    const ok = await ws.createProject(parent, name, template);
    creating = false;
    if (ok) ui.newProject = false;
  }
</script>

<Modal title="New project" width={560} onClose={() => (ui.newProject = false)}>
  <form class="body" onsubmit={(e) => { e.preventDefault(); create(); }}>
    <div class="grid" role="radiogroup" aria-label="Template">
      {#each templates as t}
        <button type="button" role="radio" aria-checked={template === t.id} class="tpl" class:on={template === t.id} onclick={() => (template = t.id)}>
          <b>{t.name}</b>
          <small>{t.desc}</small>
        </button>
      {/each}
    </div>
    <label>
      <span>Project name</span>
      <input class="input" bind:value={name} required spellcheck="false" />
    </label>
    <div class="loc">
      <span>Location</span>
      <div class="pick">
        <span class="path" title={parent}>{parent || "Choose a folder…"}{#if parent}<span class="muted">/{name}</span>{/if}</span>
        <button type="button" class="btn" onclick={choose}><FolderOpen size={14} /> Choose…</button>
      </div>
    </div>
    <div class="foot">
      <button type="button" class="btn ghost" onclick={() => (ui.newProject = false)}>Cancel</button>
      <button type="submit" class="btn primary" disabled={creating || !name.trim()}>
        {#if creating}<span class="spinner"></span>{/if} Create project
      </button>
    </div>
  </form>
</Modal>

<style>
  .body {
    padding: 8px 20px 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .tpl {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px;
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    cursor: pointer;
    transition: border-color var(--dur) var(--ease), transform 80ms ease;
  }
  .tpl:hover {
    border-color: var(--border-strong);
  }
  .tpl:active {
    transform: scale(0.98);
  }
  .tpl.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .tpl small {
    color: var(--text-muted);
  }
  label,
  .loc {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label > span,
  .loc > span {
    font-weight: 600;
    font-size: 12px;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
