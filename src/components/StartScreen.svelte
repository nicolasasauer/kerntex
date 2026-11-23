<script lang="ts">
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FileArchive from "@lucide/svelte/icons/file-archive";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import X from "@lucide/svelte/icons/x";
  import Folder from "@lucide/svelte/icons/folder";
  import { ws } from "../lib/workspace.svelte";
  import { settings, removeRecent } from "../lib/settings.svelte";
  import { ui, fmt } from "../lib/commands.svelte";
  import { openUrl } from "../lib/api";
  import { APP_VERSION, LINKS, reportProblemUrl } from "../lib/about";

  function ago(t: number) {
    const s = (Date.now() - t) / 1000;
    if (s < 60) return "just now";
    if (s < 3600) return `${Math.floor(s / 60)} min ago`;
    if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
    const d = Math.floor(s / 86400);
    return d === 1 ? "yesterday" : `${d} days ago`;
  }
</script>

<main class="start">
  <div class="inner">
    <header>
      <h1>KernTeX</h1>
      <p class="muted">Write LaTeX locally — with a live PDF, clear errors and nothing else to install.</p>
    </header>

    <div class="actions">
      <button class="action" onclick={() => (ui.newProject = true)}>
        <FilePlus size={22} />
        <span><b>New project</b><small>Blank, article, thesis, slides, letter</small></span>
        <kbd>{fmt("Mod+Shift+N")}</kbd>
      </button>
      <button class="action" onclick={() => ws.openFolderDialog()}>
        <FolderOpen size={22} />
        <span><b>Open folder</b><small>Any folder with .tex files</small></span>
        <kbd>{fmt("Mod+O")}</kbd>
      </button>
      <button class="action" onclick={() => ws.importZip()}>
        <FileArchive size={22} />
        <span><b>Import ZIP</b><small>e.g. Overleaf: Menu → Download → Source</small></span>
      </button>
      <button class="action" onclick={() => ws.openWelcome()}>
        <Sparkles size={22} />
        <span><b>Example project</b><small>A guided tour of LaTeX</small></span>
      </button>
    </div>

    <section class="recent">
      <h2>Recent projects</h2>
      {#if settings.recent.length === 0}
        <p class="muted empty">No projects yet. Create one, open a folder — or drag a folder or ZIP onto this window.</p>
      {:else}
        <ul>
          {#each settings.recent as r (r.root)}
            <li>
              <button class="recent-item" onclick={() => ws.openProject(r.root)} title={r.root}>
                <Folder size={16} />
                <span class="rname">{r.name}</span>
                <span class="rpath muted"><bdi>{r.root}</bdi></span>
                <span class="rtime muted">{ago(r.openedAt)}</span>
              </button>
              <button class="icon-btn remove" title="Remove from list (the folder is not touched)" aria-label="Remove {r.name} from recent projects" onclick={() => removeRecent(r.root)}>
                <X size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <p class="hint muted">Tip: drop a project folder or an Overleaf ZIP anywhere on this window. Press <kbd>{fmt("Mod+K")}</kbd> for all commands.</p>

    <footer class="foot muted">
      <span>KernTeX {APP_VERSION}</span>
      <span aria-hidden="true">·</span>
      <button onclick={() => (ui.about = true)}>About</button>
      <span aria-hidden="true">·</span>
      <button onclick={() => openUrl(LINKS.repo)}>GitHub</button>
      <span aria-hidden="true">·</span>
      <button onclick={() => openUrl(reportProblemUrl())}>Report a problem</button>
    </footer>
  </div>
</main>

<style>
  .start {
    flex: 1;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 48px 24px;
  }
  .inner {
    width: 100%;
    max-width: 760px;
    animation: pop-in 250ms var(--ease);
  }
  h1 {
    margin: 0;
    font-size: 30px;
    letter-spacing: -0.02em;
  }
  header p {
    margin: 6px 0 28px;
    font-size: 15px;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .action {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    text-align: left;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--accent);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition: border-color var(--dur) var(--ease), transform 80ms ease, background var(--dur) var(--ease);
  }
  .action:hover {
    border-color: var(--accent);
    background: var(--surface-raised);
  }
  .action:active {
    transform: scale(0.985);
  }
  .action span {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--text);
  }
  .action b {
    font-size: 14px;
  }
  .action small {
    color: var(--text-muted);
    font-size: 12px;
  }
  .recent {
    margin-top: 34px;
  }
  h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    margin: 0 0 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    overflow: hidden;
  }
  li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }
  li:last-child {
    border-bottom: none;
  }
  .recent-item {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: auto auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--text-muted);
  }
  .recent-item:hover {
    background: var(--hover);
  }
  .recent-item:active {
    background: var(--pressed);
  }
  .rname {
    color: var(--text);
    font-weight: 600;
  }
  .rpath {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .remove {
    margin-right: 6px;
    opacity: 0;
  }
  li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .empty {
    padding: 18px;
    border: 1px dashed var(--border-strong);
    border-radius: 12px;
    margin: 0;
  }
  .foot {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 28px;
    font-size: 12px;
  }
  .foot button {
    border: none;
    background: none;
    padding: 0;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
  }
  .foot button:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .hint {
    margin-top: 24px;
    font-size: 12px;
  }
  @media (max-width: 640px) {
    .actions {
      grid-template-columns: 1fr;
    }
  }
</style>
