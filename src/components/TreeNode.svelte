<script lang="ts">
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import FileText from "@lucide/svelte/icons/file-text";
  import FileImage from "@lucide/svelte/icons/file-image";
  import File from "@lucide/svelte/icons/file";
  import BookOpen from "@lucide/svelte/icons/book-open";
  import Folder from "@lucide/svelte/icons/folder";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import Star from "@lucide/svelte/icons/star";
  import TreeNode from "./TreeNode.svelte";
  import InlineInput from "./InlineInput.svelte";
  import type { FileNode } from "../lib/types";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { ui } from "../lib/commands.svelte";

  let { node, depth, expanded, renaming, onMenu, onToggle, onRenameDone }: {
    node: FileNode;
    depth: number;
    expanded: Set<string>;
    renaming: string | null;
    onMenu: (e: MouseEvent, node: FileNode) => void;
    onToggle: (path: string) => void;
    onRenameDone: () => void;
  } = $props();

  const open = $derived(expanded.has(node.path));
  const isMain = $derived(ws.mainFile === node.path);
  const dirty = $derived(ws.tabs.some((t) => t.path === node.path && t.dirty));
  const children = $derived(node.children.filter((c) => settings.showGenerated || !c.generated));
  const creatingHere = $derived(ui.treeCreate && ui.treeCreate.dir === node.path && node.isDir);

  function icon(n: FileNode) {
    if (n.isDir) return open ? FolderOpen : Folder;
    if (/\.(tex|sty|cls|ltx)$/i.test(n.name)) return FileText;
    if (/\.bib$/i.test(n.name)) return BookOpen;
    if (/\.(png|jpe?g|gif|svg|pdf|eps)$/i.test(n.name)) return FileImage;
    return File;
  }
  const Icon = $derived(icon(node));

  function click() {
    if (node.isDir) onToggle(node.path);
    else ws.openFile(node.path);
  }
</script>

{#if renaming === node.path}
  <div class="row" style:padding-left="{8 + depth * 14}px">
    <span class="chev"></span>
    <Icon size={15} />
    <InlineInput
      value={node.name}
      selectStem={!node.isDir}
      onSubmit={(v) => { onRenameDone(); ws.rename(node.path, v); }}
      onCancel={onRenameDone}
    />
  </div>
{:else}
  <button
    class="row"
    class:active={ws.active === node.path}
    class:generated={node.generated}
    style:padding-left="{8 + depth * 14}px"
    onclick={click}
    oncontextmenu={(e) => onMenu(e, node)}
    title={node.path}
    aria-expanded={node.isDir ? open : undefined}
  >
    <span class="chev" class:open>{#if node.isDir}<ChevronRight size={14} />{/if}</span>
    <span class="icon" class:main={isMain}><Icon size={15} /></span>
    <span class="name">{node.name}</span>
    {#if dirty}<span class="dot" title="Unsaved changes"></span>{/if}
    {#if isMain}<span class="main-badge" title="Main file — this document is compiled"><Star size={12} fill="currentColor" /></span>{/if}
  </button>
{/if}

{#if node.isDir && open}
  <div class="children">
    {#if creatingHere && ui.treeCreate}
      {@const kind = ui.treeCreate.kind}
      <div class="row" style:padding-left="{8 + (depth + 1) * 14}px">
        <span class="chev"></span>
        {#if kind === "folder"}<Folder size={15} />{:else}<FileText size={15} />{/if}
        <InlineInput
          value={kind === "file" ? ".tex" : ""}
          placeholder={kind === "file" ? "name.tex" : "folder name"}
          cursorStart
          onSubmit={(v) => { ui.treeCreate = null; if (kind === "file") ws.newFile(node.path, v); else ws.newFolder(node.path, v); }}
          onCancel={() => (ui.treeCreate = null)}
        />
      </div>
    {/if}
    {#each children as child (child.path)}
      <TreeNode node={child} depth={depth + 1} {expanded} {renaming} {onMenu} {onToggle} {onRenameDone} />
    {/each}
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    height: 26px;
    padding-right: 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }
  button.row:hover {
    background: var(--hover);
  }
  button.row:active {
    background: var(--pressed);
  }
  .row.active {
    background: var(--accent-soft);
  }
  .row.generated {
    opacity: 0.5;
  }
  .chev {
    width: 14px;
    display: grid;
    place-items: center;
    color: var(--text-faint);
    transition: transform var(--dur) var(--ease);
    flex: none;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .icon {
    display: grid;
    color: var(--text-muted);
    flex: none;
  }
  .icon.main {
    color: var(--accent);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warning);
    flex: none;
  }
  .main-badge {
    display: grid;
    color: var(--accent);
    flex: none;
  }
  .children {
    animation: fade-in var(--dur) var(--ease);
  }
</style>
