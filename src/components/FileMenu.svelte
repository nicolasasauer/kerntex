<script lang="ts">
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FileArchive from "@lucide/svelte/icons/file-archive";
  import Folder from "@lucide/svelte/icons/folder";
  import Download from "@lucide/svelte/icons/download";
  import X from "@lucide/svelte/icons/x";
  import ContextMenu, { type MenuEntry } from "./ContextMenu.svelte";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { commandById, fmt } from "../lib/commands.svelte";

  const MAX_RECENT = 6;

  let button: HTMLButtonElement | undefined = $state();
  let menu: { x: number; y: number; items: MenuEntry[] } | null = $state(null);

  /** A menu entry for a command, so shortcut and action have one source. */
  function command(id: string, icon?: any, label?: string): MenuEntry {
    const c = commandById(id)!;
    return { label: label ?? c.title, icon, shortcut: fmt(c.shortcut), run: () => void c.run() };
  }

  function items(): MenuEntry[] {
    const list: MenuEntry[] = [
      command("newProject", FilePlus),
      command("openFolder", FolderOpen),
      command("importZip", FileArchive, "Import ZIP…"),
    ];
    const recent = settings.recent.filter((r) => r.root !== ws.project?.root).slice(0, MAX_RECENT);
    if (recent.length > 0) {
      list.push("separator", { heading: "Recent projects" });
      for (const r of recent) list.push({ label: r.name, icon: Folder, title: r.root, run: () => void ws.openProject(r.root) });
    }
    if (ws.project) {
      list.push(
        "separator",
        command("exportPdf", Download),
        command("exportZip"),
        command("exportCleanZip"),
        "separator",
        command("closeProject", X),
      );
    }
    return list;
  }

  function open() {
    const r = button!.getBoundingClientRect();
    menu = { x: r.left, y: r.bottom + 4, items: items() };
  }
</script>

<button class="file" class:open={menu} bind:this={button} onclick={open} aria-haspopup="menu" aria-expanded={menu != null}>
  File <ChevronDown size={14} />
</button>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => (menu = null)} />
{/if}

<style>
  .file {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 28px;
    padding: 0 6px 0 9px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }
  .file:hover,
  .file.open {
    background: var(--hover);
  }
  .file:active {
    background: var(--pressed);
  }
</style>
