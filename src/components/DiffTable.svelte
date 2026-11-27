<script module lang="ts">
  /** One row of a diff: added, deleted, unchanged context, or a hunk separator. */
  export type DiffLine = { kind: "add" | "del" | "ctx" | "hunk"; text: string; old: number | null; new: number | null };
</script>

<script lang="ts">
  let { lines }: { lines: DiffLine[] } = $props();
</script>

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

<style>
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
