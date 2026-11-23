<script lang="ts">
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Eye from "@lucide/svelte/icons/eye";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import Star from "@lucide/svelte/icons/star";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import FileText from "@lucide/svelte/icons/file-text";
  import Folder from "@lucide/svelte/icons/folder";
  import FileUp from "@lucide/svelte/icons/file-up";
  import Eye2 from "@lucide/svelte/icons/scan-eye";
  import { SvelteSet } from "svelte/reactivity";
  import TreeNode from "./TreeNode.svelte";
  import InlineInput from "./InlineInput.svelte";
  import ContextMenu, { type MenuEntry } from "./ContextMenu.svelte";
  import { ws, isPreviewable } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { ui } from "../lib/commands.svelte";
  import { joinPath, revealInFolder } from "../lib/api";
  import type { FileNode } from "../lib/types";

  const expanded = new SvelteSet<string>();
  let renaming: string | null = $state(null);
  let menu: { x: number; y: number; items: MenuEntry[] } | null = $state(null);
  let spinning = $state(false);

  const nodes = $derived((ws.project?.tree ?? []).filter((n) => settings.showGenerated || !n.generated));

  // Expand the folder that receives a new file.
  $effect(() => {
    if (ui.treeCreate?.dir) expanded.add(ui.treeCreate.dir);
  });

  function toggle(path: string) {
    if (expanded.has(path)) expanded.delete(path);
    else expanded.add(path);
  }

  function openMenu(e: MouseEvent, node: FileNode | null) {
    e.preventDefault();
    e.stopPropagation();
    const dir = node ? (node.isDir ? node.path : node.path.includes("/") ? node.path.slice(0, node.path.lastIndexOf("/")) : "") : "";
    const items: MenuEntry[] = [
      { label: "New file", icon: FilePlus, run: () => (ui.treeCreate = { dir, kind: "file" }) },
      { label: "New folder", icon: FolderPlus, run: () => (ui.treeCreate = { dir, kind: "folder" }) },
      { label: dir ? `Add files to “${dir.split("/").pop()}”…` : "Add files…", icon: FileUp, run: () => ws.addFiles(dir) },
    ];
    if (node) {
      items.push("separator");
      if (!node.isDir && isPreviewable(node.path)) {
        items.push({ label: "Preview", icon: Eye2, run: () => ws.openFile(node.path) });
        if (ws.lastTextTab?.endsWith(".tex")) {
          const target = ws.lastTextTab;
          items.push({ label: `Insert figure into ${target.split("/").pop()}`, icon: FileUp, run: () => ws.insertFigure(node.path, target) });
        }
      }
      if (!node.isDir && /\.tex$/i.test(node.name)) {
        items.push({ label: "Set as main file", icon: Star, disabled: ws.mainFile === node.path, run: () => ws.setMainFile(node.path) });
      }
      items.push({ label: "Rename", icon: Pencil, shortcut: "F2", run: () => (renaming = node.path) });
      items.push({ label: "Show in file manager", icon: FolderSearch, run: () => revealInFolder(joinPath(ws.project!.root, node.path)) });
      items.push("separator");
      items.push({ label: "Move to trash", icon: Trash2, danger: true, run: () => ws.trash(node.path) });
    } else {
      items.push({ label: "Show in file manager", icon: FolderSearch, run: () => revealInFolder(ws.project!.root) });
    }
    menu = { x: e.clientX, y: e.clientY, items };
  }

  async function refresh() {
    spinning = true;
    await ws.refreshTree();
    setTimeout(() => (spinning = false), 300);
  }

  function key(e: KeyboardEvent) {
    if (e.key === "F2" && ws.active) {
      renaming = ws.active;
      e.preventDefault();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="filetree" class:drop-root={ui.dropDir === ""} oncontextmenu={(e) => openMenu(e, null)} onkeydown={key}>
  <div class="head">
    <span class="label">Files</span>
    <div class="tools">
      <button class="icon-btn" title="New file" aria-label="New file" onclick={() => (ui.treeCreate = { dir: "", kind: "file" })}><FilePlus size={15} /></button>
      <button class="icon-btn" title="New folder" aria-label="New folder" onclick={() => (ui.treeCreate = { dir: "", kind: "folder" })}><FolderPlus size={15} /></button>
      <button class="icon-btn" title="Add files (images, .bib, …) — or drag them here" aria-label="Add files" onclick={() => ws.addFiles("")}><FileUp size={15} /></button>
      <button
        class="icon-btn"
        class:on={settings.showGenerated}
        title={settings.showGenerated ? "Hide generated files (.aux, .log …)" : "Show generated files (.aux, .log …)"}
        aria-label="Toggle generated files"
        onclick={() => (settings.showGenerated = !settings.showGenerated)}
      >
        {#if settings.showGenerated}<Eye size={15} />{:else}<EyeOff size={15} />{/if}
      </button>
      <button class="icon-btn" class:spin={spinning} title="Refresh file tree" aria-label="Refresh file tree" onclick={refresh}><RefreshCw size={15} /></button>
    </div>
  </div>
  <div class="list" role="tree" data-dir="">
    {#if ui.treeCreate && ui.treeCreate.dir === ""}
      {@const kind = ui.treeCreate.kind}
      <div class="row-new">
        {#if kind === "folder"}<Folder size={15} />{:else}<FileText size={15} />{/if}
        <InlineInput
          value={kind === "file" ? ".tex" : ""}
          placeholder={kind === "file" ? "name.tex" : "folder name"}
          cursorStart
          onSubmit={(v) => { ui.treeCreate = null; if (kind === "file") ws.newFile("", v); else ws.newFolder("", v); }}
          onCancel={() => (ui.treeCreate = null)}
        />
      </div>
    {/if}
    {#each nodes as node (node.path)}
      <TreeNode {node} depth={0} {expanded} {renaming} onMenu={openMenu} onToggle={toggle} onRenameDone={() => (renaming = null)} />
    {/each}
    <p class="drop-hint">Drag images or other files here to add them</p>
  </div>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => (menu = null)} />
{/if}

<style>
  .filetree {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border);
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .tools {
    display: flex;
  }
  .tools .icon-btn {
    width: 26px;
    height: 26px;
  }
  .filetree.drop-root .list {
    background: var(--accent-soft);
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
    border-radius: 8px;
  }
  .drop-hint {
    margin: 10px 8px;
    padding: 10px;
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
  }
  .spin :global(svg) {
    animation: spin 0.6s linear;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 6px;
  }
  .row-new {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding-left: 27px;
    color: var(--text-muted);
  }
</style>
