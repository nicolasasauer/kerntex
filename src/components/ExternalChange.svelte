<script lang="ts">
  import Modal from "./Modal.svelte";
  import DiffTable from "./DiffTable.svelte";
  import { ws } from "../lib/workspace.svelte";
  import { lineDiff } from "../lib/linediff";

  let { path, onClose }: { path: string; onClose: () => void } = $props();

  const tab = $derived(ws.tabs.find((t) => t.path === path));
  // Red: only in the file on disk. Green: only in the editor.
  const lines = $derived(tab?.external?.text != null ? lineDiff(tab.external.text, ws.editor?.text(path) ?? tab.saved) : []);

  // Act before closing: once the dialog is gone, its props are too.
  function keepMine() {
    ws.keepMine(path);
    onClose();
  }
  function loadTheirs() {
    ws.loadTheirs(path);
    onClose();
  }
</script>

<Modal title="{path}: your version and the file on disk" {onClose} width={900}>
  <div class="bar">
    <span class="legend"><span class="sw del"></span>Only on disk</span>
    <span class="legend"><span class="sw add"></span>Only in your version</span>
    <span class="spacer"></span>
    <button class="btn" onclick={loadTheirs}>Load the file on disk</button>
    <button class="btn primary" onclick={keepMine}>Keep my version</button>
  </div>
  <div class="diff">
    {#if lines.length === 0}
      <p class="msg muted">No differences left.</p>
    {:else}
      <DiffTable {lines} />
    {/if}
  </div>
</Modal>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 4px 20px 10px;
    font-size: 12.5px;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }
  .sw {
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }
  .sw.del {
    background: var(--danger-soft);
    box-shadow: inset 0 0 0 1px var(--danger);
  }
  .sw.add {
    background: var(--success-soft);
    box-shadow: inset 0 0 0 1px var(--success);
  }
  .diff {
    flex: 1;
    min-height: 120px;
    max-height: 70vh;
    overflow: auto;
    border-top: 1px solid var(--border);
    background: var(--editor-bg);
    font-family: var(--font-mono);
    font-size: 12.5px;
    user-select: text;
  }
  .msg {
    margin: 0;
    padding: 16px 20px;
    font-family: var(--font-ui);
  }
</style>
