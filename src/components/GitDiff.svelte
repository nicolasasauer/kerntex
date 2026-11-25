<script lang="ts">
  import FileText from "@lucide/svelte/icons/file-text";
  import Modal from "./Modal.svelte";
  import { api, toAppError } from "../lib/api";
  import { ws } from "../lib/workspace.svelte";
  import type { AppError, GitChange } from "../lib/types";

  let { change, onClose }: { change: GitChange; onClose: () => void } = $props();

  type Line = { kind: "add" | "del" | "ctx" | "hunk"; text: string; old: number | null; new: number | null };

  let lines = $state<Line[] | null>(null);
  let binary = $state(false);
  let error = $state<AppError | null>(null);

  $effect(() => {
    const root = ws.project?.root;
    if (!root) return;
    lines = null;
    error = null;
    api
      .gitDiff(root, change.path, change.oldPath)
      .then((text) => {
        binary = /^Binary files .* differ$/m.test(text);
        lines = parse(text);
      })
      .catch((e) => (error = toAppError(e)));
  });

  /** Unified diff → display lines with old/new line numbers; file headers dropped. */
  function parse(text: string): Line[] {
    const out: Line[] = [];
    let o = 0;
    let n = 0;
    let inHunk = false;
    for (const l of text.split("\n")) {
      const m = l.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/);
      if (m) {
        o = +m[1];
        n = +m[2];
        inHunk = true;
        out.push({ kind: "hunk", text: m[3].trim(), old: null, new: null });
      } else if (!inHunk) {
        continue;
      } else if (l.startsWith("+")) {
        out.push({ kind: "add", text: l.slice(1), old: null, new: n++ });
      } else if (l.startsWith("-")) {
        out.push({ kind: "del", text: l.slice(1), old: o++, new: null });
      } else if (l.startsWith(" ")) {
        out.push({ kind: "ctx", text: l.slice(1), old: o++, new: n++ });
      } else if (l.startsWith("\\")) {
        out.push({ kind: "hunk", text: l.slice(2), old: null, new: null });
      } else if (l.startsWith("diff --git")) {
        inHunk = false;
      }
    }
    return out;
  }

  const stats = $derived({
    add: lines?.filter((l) => l.kind === "add").length ?? 0,
    del: lines?.filter((l) => l.kind === "del").length ?? 0,
  });

  function open() {
    onClose();
    ws.openFile(change.path);
  }
</script>

<Modal title={change.oldPath ? `${change.oldPath} → ${change.path}` : change.path} {onClose} width={900}>
  <div class="bar">
    <span class="muted">Changes since the last commit</span>
    {#if lines}<span class="plus">+{stats.add}</span><span class="minus">−{stats.del}</span>{/if}
    {#if change.kind !== "deleted"}<button class="btn small" onclick={open}><FileText size={14} />Open file</button>{/if}
  </div>
  <div class="diff">
    {#if error}
      <p class="msg"><strong>{error.title}</strong>{#if error.hint}<br /><span class="muted">{error.hint}</span>{/if}</p>
    {:else if !lines}
      <p class="msg muted"><span class="spinner"></span>Loading…</p>
    {:else if binary}
      <p class="msg muted">Binary file (image, PDF …) — no text changes to show.</p>
    {:else if lines.length === 0}
      <p class="msg muted">No text changes (only the file mode or line endings changed).</p>
    {:else}
      <table>
        <tbody>
          {#each lines as l, i (i)}
            {#if l.kind === "hunk"}
              <tr class="hunk"><td class="ln"></td><td class="ln"></td><td class="code">{l.text || "⋯"}</td></tr>
            {:else}
              <tr class={l.kind}>
                <td class="ln">{l.old ?? ""}</td>
                <td class="ln">{l.new ?? ""}</td>
                <td class="code"><span class="sign">{l.kind === "add" ? "+" : l.kind === "del" ? "−" : " "}</span>{l.text}</td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</Modal>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 20px 10px;
    font-size: 12.5px;
  }
  .bar .btn {
    margin-left: auto;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .plus {
    color: var(--success);
    font-weight: 600;
  }
  .minus {
    color: var(--danger);
    font-weight: 600;
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
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 0;
    padding: 16px 20px;
    font-family: var(--font-ui);
    flex-wrap: wrap;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 0 8px;
    vertical-align: top;
    line-height: 1.55;
  }
  .ln {
    width: 1%;
    min-width: 36px;
    text-align: right;
    color: var(--text-faint);
    user-select: none;
    white-space: nowrap;
  }
  .code {
    white-space: pre-wrap;
    word-break: break-word;
    padding-left: 4px;
  }
  .sign {
    display: inline-block;
    width: 14px;
    user-select: none;
    color: var(--text-faint);
  }
  tr.add {
    background: var(--success-soft);
  }
  tr.add .sign {
    color: var(--success);
  }
  tr.del {
    background: var(--danger-soft);
  }
  tr.del .sign {
    color: var(--danger);
  }
  tr.hunk td {
    background: var(--surface-2);
    color: var(--text-muted);
    padding-top: 2px;
    padding-bottom: 2px;
  }
</style>
