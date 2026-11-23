<script lang="ts">
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import { ws } from "../lib/workspace.svelte";
  import { revealInFolder, openUrl } from "../lib/api";
  import { loadedPackages, PACKAGE_INFO } from "../lib/latex/packages";
  import type { PackageRef } from "../lib/types";

  let showDeps = $state(false);
  let filter = $state("");

  const result = $derived(ws.status.state === "done" && !ws.status.result.canceled ? ws.status.result : null);
  const d = $derived(result?.details ?? null);

  /** Packages named in the main file's preamble — the ones the author chose. */
  const direct = $derived.by(() => {
    const main = ws.mainFile;
    const text = main ? ws.editor?.text(main) : null;
    return text ? loadedPackages(text).packages : new Set<string>();
  });

  const engineVersion = $derived(
    ws.engine === "tectonic" ? ws.engines?.tectonicVersion : ws.engines?.latexmkVersion,
  );

  function matches(p: PackageRef) {
    const q = filter.trim().toLowerCase();
    return !q || p.name.toLowerCase().includes(q) || (p.description ?? "").toLowerCase().includes(q);
  }

  const chosen = $derived((d?.packages ?? []).filter((p) => direct.has(p.name)));
  const deps = $derived((d?.packages ?? []).filter((p) => !direct.has(p.name)));
  const shown = $derived([...chosen, ...(showDeps || filter ? deps : [])].filter(matches));

  function ver(p: PackageRef) {
    return [p.version, p.date].filter(Boolean).join(" · ") || "—";
  }
</script>

<div class="details">
  {#if !d}
    <p class="muted empty">Compile once to see which engine and packages your document uses.</p>
  {:else}
    <section class="facts">
      <div class="fact">
        <span class="k">Engine</span>
        <span class="v">
          {#if engineVersion && result && engineVersion.toLowerCase().startsWith(result.engineLabel.split(" ")[0].toLowerCase())}
            {engineVersion}{#if result.engineLabel.includes("(")}<span class="muted"> · {result.engineLabel}</span>{/if}
          {:else}
            {result?.engineLabel}{#if engineVersion}<span class="muted"> · {engineVersion}</span>{/if}
          {/if}
        </span>
      </div>
      {#if d.program}<div class="fact"><span class="k">TeX program</span><span class="v mono">{d.program}</span></div>{/if}
      {#if d.format}<div class="fact"><span class="k">LaTeX format</span><span class="v mono">{d.format}</span></div>{/if}
      {#if d.documentClass}
        <div class="fact">
          <span class="k">Document class</span>
          <span class="v"><b class="mono">{d.documentClass.name}</b> <span class="muted">{ver(d.documentClass)}</span></span>
        </div>
      {/if}
      <div class="fact">
        <span class="k">Last build</span>
        <span class="v">{d.passes || 1} TeX pass{(d.passes || 1) === 1 ? "" : "es"} · {((result?.durationMs ?? 0) / 1000).toFixed(1)} s</span>
      </div>
      <div class="fact">
        <span class="k">Files</span>
        <span class="v links">
          {#if d.outDir}{@const out = d.outDir}<button class="link" onclick={() => revealInFolder(out)}><FolderSearch size={13} /> Output folder</button>{/if}
          {#if d.logPath}{@const lp = d.logPath}<button class="link" onclick={() => revealInFolder(lp)}><FolderSearch size={13} /> Full .log file</button>{/if}
          {#if ws.engine === "tectonic" && ws.engines?.tectonicCacheDir}{@const cd = ws.engines.tectonicCacheDir}<button class="link" onclick={() => revealInFolder(cd)} title={cd}><FolderSearch size={13} /> Tectonic package cache</button>{/if}
        </span>
      </div>
      {#if d.enginePath}<div class="fact"><span class="k">Engine binary</span><span class="v mono small">{d.enginePath}</span></div>{/if}
    </section>

    <section class="pkgs">
      <div class="pkg-head">
        <h4>Packages <span class="muted">{chosen.length} loaded by your document · {deps.length} pulled in automatically</span></h4>
        <div class="tools">
          <input class="input search" placeholder="Filter packages…" bind:value={filter} aria-label="Filter packages" />
          <label class="toggle"><input type="checkbox" bind:checked={showDeps} /> Show dependencies</label>
        </div>
      </div>
      {#if shown.length === 0}
        <p class="muted empty">{filter ? "No package matches." : "The document doesn't load any packages."}</p>
      {:else}
        <table>
          <thead><tr><th>Package</th><th>Version</th><th>What it does</th><th></th></tr></thead>
          <tbody>
            {#each shown as p (p.name)}
              <tr class:dep={!direct.has(p.name)}>
                <td class="mono name">{p.name}{#if !direct.has(p.name)}<span class="tag">dependency</span>{/if}</td>
                <td class="mono muted">{ver(p)}</td>
                <td>{PACKAGE_INFO[p.name] ?? p.description ?? ""}</td>
                <td><button class="icon-btn" title="Documentation on CTAN" aria-label="{p.name} on CTAN" onclick={() => openUrl(`https://ctan.org/pkg/${p.name}`)}><ExternalLink size={13} /></button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </section>
  {/if}
</div>

<style>
  .details {
    padding: 10px 16px 16px;
    user-select: text;
  }
  .empty {
    margin: 4px 0;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 5px 18px;
    margin-bottom: 14px;
  }
  .fact {
    display: contents;
  }
  .k {
    color: var(--text-muted);
  }
  .v {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .small {
    color: var(--text-muted);
  }
  .links {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .pkg-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 6px;
  }
  h4 {
    margin: 0;
    font-size: 13px;
  }
  h4 span {
    font-weight: 400;
    margin-left: 6px;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .search {
    height: 26px;
    width: 180px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--text-muted);
    cursor: pointer;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  tr.dep td {
    color: var(--text-muted);
  }
  .name {
    white-space: nowrap;
    color: var(--text);
  }
  .tag {
    margin-left: 6px;
    font-family: var(--font-ui);
    font-size: 10px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  td .icon-btn {
    width: 24px;
    height: 24px;
  }
</style>
