// Line diff between two texts, for showing what changed on disk. Returns
// display rows with a few lines of context around each change.

import type { DiffLine } from "../components/DiffTable.svelte";

const CONTEXT = 3;
/** Above this many cells the middle part is shown as replaced as a whole. */
const MAX_CELLS = 4_000_000;

type Op = { kind: "add" | "del" | "ctx"; text: string };

export function lineDiff(oldText: string, newText: string): DiffLine[] {
  const a = oldText.split("\n");
  const b = newText.split("\n");
  // Common start and end need no table.
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start++;
  let endA = a.length;
  let endB = b.length;
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA--;
    endB--;
  }
  const ops: Op[] = a.slice(0, start).map((text) => ({ kind: "ctx", text }));
  ops.push(...middle(a.slice(start, endA), b.slice(start, endB)));
  ops.push(...a.slice(endA).map((text): Op => ({ kind: "ctx", text })));
  return withContext(ops);
}

/** Longest common subsequence of lines; falls back to delete-all/add-all when huge. */
function middle(a: string[], b: string[]): Op[] {
  const n = a.length;
  const m = b.length;
  if (n * m > MAX_CELLS) return [...a.map((text): Op => ({ kind: "del", text })), ...b.map((text): Op => ({ kind: "add", text }))];
  const w = m + 1;
  const len = new Uint32Array((n + 1) * w);
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      len[i * w + j] = a[i] === b[j] ? len[(i + 1) * w + j + 1] + 1 : Math.max(len[(i + 1) * w + j], len[i * w + j + 1]);
    }
  }
  const out: Op[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      out.push({ kind: "ctx", text: a[i++] });
      j++;
    } else if (len[(i + 1) * w + j] >= len[i * w + j + 1]) out.push({ kind: "del", text: a[i++] });
    else out.push({ kind: "add", text: b[j++] });
  }
  while (i < n) out.push({ kind: "del", text: a[i++] });
  while (j < m) out.push({ kind: "add", text: b[j++] });
  return out;
}

/** Numbers the lines and keeps only CONTEXT unchanged lines around changes. */
function withContext(ops: Op[]): DiffLine[] {
  const keep = new Array<boolean>(ops.length).fill(false);
  ops.forEach((op, k) => {
    if (op.kind === "ctx") return;
    for (let x = Math.max(0, k - CONTEXT); x <= Math.min(ops.length - 1, k + CONTEXT); x++) keep[x] = true;
  });
  const rows: DiffLine[] = [];
  let o = 1;
  let n = 1;
  let skipped = false;
  ops.forEach((op, k) => {
    const row: DiffLine = { kind: op.kind, text: op.text, old: op.kind === "add" ? null : o, new: op.kind === "del" ? null : n };
    if (op.kind !== "add") o++;
    if (op.kind !== "del") n++;
    if (!keep[k]) {
      skipped = true;
      return;
    }
    if (skipped) rows.push({ kind: "hunk", text: "", old: null, new: null });
    skipped = false;
    rows.push(row);
  });
  return rows;
}
