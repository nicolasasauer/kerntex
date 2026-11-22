<script lang="ts">
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Info from "@lucide/svelte/icons/info";
  import Package from "@lucide/svelte/icons/package";
  import Copy from "@lucide/svelte/icons/copy";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import { ws } from "../lib/workspace.svelte";
  import { settings } from "../lib/settings.svelte";
  import { ui, fmt } from "../lib/commands.svelte";
  import { openUrl } from "../lib/api";
  import { toast } from "../lib/toasts.svelte";
  import { PACKAGE_INFO } from "../lib/latex/packages";
  import type { Diagnostic, Severity } from "../lib/types";

  let filter: Severity | "all" = $state("all");
  const L = settings.layout;

  const groups = $derived(
    (["error", "warning", "info"] as Severity[]).map((s) => ({ severity: s, items: ws.diagnostics.filter((d) => d.severity === s) })),
  );
  const counts = $derived(Object.fromEntries(groups.map((g) => [g.severity, g.items.length])) as Record<Severity, number>);
  const shown = $derived(groups.filter((g) => g.items.length > 0 && (filter === "all" || filter === g.severity)));

  /** `File 'foo.sty' not found` → foo (only for packages/classes). */
  function missingPackage(d: Diagnostic): { file: string; pkg: string } | null {
    const m = d.message.match(/File [`'"]([^'`"]+\.(sty|cls))['`"] not found/);
    return m ? { file: m[1], pkg: m[1].replace(/\.(sty|cls)$/, "") } : null;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast({ kind: "success", source: "KernTeX", title: "Copied to clipboard", body: text }, 2000);
    } catch {
      toast({ kind: "error", source: "KernTeX", title: "Couldn't copy", body: text });
    }
  }

  async function switchToTectonic() {
    await ws.updateProjectSettings({ engine: "tectonic" });
    toast({ kind: "success", source: "KernTeX", title: "This project now uses Tectonic", body: "Missing packages are downloaded automatically." });
    ws.compile();
  }

  function toggle() {
    L.problemsOpen = !L.problemsOpen;
  }
</script>

<section class="problems" class:open={L.problemsOpen}>
  <div class="head">
    <button class="title" onclick={toggle} aria-expanded={L.problemsOpen} title="Toggle problems panel ({fmt('Mod+J')})">
      <span class="chev" class:open={L.problemsOpen}><ChevronUp size={15} /></span>
      <span>Problems</span>
      <span class="count err" class:zero={counts.error === 0}><CircleX size={13} /> {counts.error}</span>
      <span class="count warn" class:zero={counts.warning === 0}><TriangleAlert size={13} /> {counts.warning}</span>
      <span class="count inf" class:zero={counts.info === 0}><Info size={13} /> {counts.info}</span>
    </button>
    {#if L.problemsOpen}
      <div class="tabs">
        {#if !ui.rawLog}
          {#each [["all", "All"], ["error", "Errors"], ["warning", "Warnings"], ["info", "Info"]] as [v, label]}
            <button class="chip" class:on={filter === v} onclick={() => (filter = v as Severity | "all")}>{label}</button>
          {/each}
          <span class="sep"></span>
        {/if}
        <button class="chip" class:on={!ui.rawLog} onclick={() => (ui.rawLog = false)}>Problems</button>
        <button class="chip" class:on={ui.rawLog} onclick={() => (ui.rawLog = true)}>Raw log</button>
      </div>
    {/if}
  </div>

  {#if L.problemsOpen}
    <div class="body" style:height="{L.problems}px">
      {#if ui.rawLog}
        <pre class="log">{ws.log || "No log yet — compile first."}</pre>
      {:else if ws.diagnostics.length === 0}
        <p class="empty muted">
          {#if ws.status.state === "done"}No problems. Nice!{:else}Compile to check your document for problems.{/if}
        </p>
      {:else}
        {#each shown as g (g.severity)}
          <ul>
            {#each g.items as d, i (i)}
              {@const pkg = missingPackage(d)}
              <li class={d.severity}>
                <button class="item" onclick={() => ws.gotoDiagnostic(d)} disabled={!d.file} title={d.file ? `Go to ${d.file}${d.line ? `:${d.line}` : ""}` : undefined}>
                  <span class="sev">
                    {#if d.severity === "error"}<CircleX size={15} />{:else if d.severity === "warning"}<TriangleAlert size={15} />{:else}<Info size={15} />{/if}
                  </span>
                  <span class="main">
                    <span class="msg">{d.message}</span>
                    {#if d.hint && !pkg}<span class="hint">{d.hint}</span>{/if}
                  </span>
                  <span class="src">{d.source}</span>
                  {#if d.file}<span class="loc">{d.file}{#if d.line}:{d.line}{/if}</span>{/if}
                </button>
                {#if pkg}
                  <div class="pkg-card">
                    <div class="pkg-head"><Package size={15} /> <b>{pkg.file}</b> is missing</div>
                    {#if PACKAGE_INFO[pkg.pkg]}<p class="muted">{PACKAGE_INFO[pkg.pkg]}</p>{/if}
                    {#if ws.engine === "latexmk"}
                      <p>Your TeX distribution doesn't have this package yet. Fix it one of these ways:</p>
                      <div class="pkg-actions">
                        <button class="btn primary" onclick={switchToTectonic}>Switch this project to Tectonic</button>
                        <button class="btn" onclick={() => copy(`tlmgr install ${pkg.pkg}`)} title="For TeX Live (run in a terminal, may need sudo)">
                          <Copy size={13} /> <code>tlmgr install {pkg.pkg}</code>
                        </button>
                        <button class="btn ghost" onclick={() => openUrl(`https://ctan.org/pkg/${pkg.pkg}`)}><ExternalLink size={13} /> CTAN</button>
                      </div>
                      <p class="muted small">MiKTeX: open the MiKTeX Console → Packages, search “{pkg.pkg}” and install it — or enable “install missing packages on the fly”.</p>
                    {:else}
                      <p>Tectonic couldn't download it. Check the package name for typos{#if d.hint?.includes("internet")} and your internet connection{/if}.</p>
                      <div class="pkg-actions">
                        <button class="btn ghost" onclick={() => openUrl(`https://ctan.org/pkg/${pkg.pkg}`)}><ExternalLink size={13} /> Look up on CTAN</button>
                      </div>
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {/each}
        {#if ws.diagnostics.some((d) => d.hint?.startsWith("KernTeX couldn't find"))}
          <button class="btn link" onclick={() => (ui.rawLog = true)}>Open raw log</button>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .problems {
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 6px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 8px 0 4px;
    border: none;
    border-radius: 6px;
    background: transparent;
    font-weight: 600;
    font-size: 12px;
    cursor: pointer;
  }
  .title:hover {
    background: var(--hover);
  }
  .title:active {
    background: var(--pressed);
  }
  .chev {
    display: grid;
    color: var(--text-muted);
    transition: transform var(--dur) var(--ease);
  }
  .chev.open {
    transform: rotate(180deg);
  }
  .count {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .count.err {
    color: var(--danger);
  }
  .count.warn {
    color: var(--warning);
  }
  .count.inf {
    color: var(--info);
  }
  .count.zero {
    color: var(--text-faint);
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .sep {
    width: 1px;
    height: 16px;
    background: var(--border);
    margin: 0 6px;
  }
  .chip {
    height: 24px;
    padding: 0 9px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }
  .chip:hover {
    background: var(--hover);
  }
  .chip.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .body {
    overflow: auto;
    border-top: 1px solid var(--border);
    animation: fade-in var(--dur) var(--ease);
    user-select: text;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 4px;
  }
  .item {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: start;
    gap: 10px;
    width: 100%;
    padding: 7px 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
    user-select: text;
  }
  .item:hover:not(:disabled) {
    background: var(--hover);
  }
  .item:active:not(:disabled) {
    background: var(--pressed);
  }
  .item:disabled {
    cursor: default;
  }
  .sev {
    display: grid;
    padding-top: 1px;
  }
  .error .sev {
    color: var(--danger);
  }
  .warning .sev {
    color: var(--warning);
  }
  .info .sev {
    color: var(--info);
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .msg {
    font-family: var(--font-mono);
    font-size: 12px;
    word-break: break-word;
  }
  .hint {
    color: var(--text-muted);
    line-height: 1.45;
  }
  .src {
    font-size: 11px;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .loc {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent);
    white-space: nowrap;
  }
  .pkg-card {
    margin: 0 8px 8px 33px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-left: 3px solid var(--danger);
    border-radius: 8px;
    background: var(--surface-2);
  }
  .pkg-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .pkg-card p {
    margin: 6px 0;
    line-height: 1.45;
  }
  .pkg-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0;
  }
  code {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .small {
    font-size: 12px;
  }
  .log {
    margin: 0;
    padding: 10px 14px;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--text-muted);
  }
  .empty {
    padding: 14px 16px;
    margin: 0;
  }
  .btn.link {
    margin: 0 0 10px 12px;
  }
</style>
