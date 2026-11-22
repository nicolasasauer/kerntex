<script lang="ts">
  import Play from "@lucide/svelte/icons/play";
  import Square from "@lucide/svelte/icons/square";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import PanelRight from "@lucide/svelte/icons/panel-right";
  import Search from "@lucide/svelte/icons/search";
  import Settings from "@lucide/svelte/icons/settings";
  import Sun from "@lucide/svelte/icons/sun";
  import Moon from "@lucide/svelte/icons/moon";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Download from "@lucide/svelte/icons/download";
  import { ws } from "../lib/workspace.svelte";
  import { settings, effectiveTheme } from "../lib/settings.svelte";
  import { ui, fmt, modKey } from "../lib/commands.svelte";

  // Elapsed time ticks while compiling.
  let now = $state(performance.now());
  $effect(() => {
    if (ws.status.state !== "running") return;
    const t = setInterval(() => (now = performance.now()), 100);
    return () => clearInterval(t);
  });

  const counts = $derived({
    errors: ws.diagnostics.filter((d) => d.severity === "error").length,
    warnings: ws.diagnostics.filter((d) => d.severity === "warning").length,
  });

  function secs(ms: number) {
    return ms < 10000 ? (ms / 1000).toFixed(1) + " s" : Math.round(ms / 1000) + " s";
  }

  function toggleTheme() {
    settings.theme = effectiveTheme() === "dark" ? "light" : "dark";
  }
</script>

<header class="toolbar">
  <div class="left">
    <div class="brand" title="KernTeX">
      <svg viewBox="0 0 1024 1024" width="20" height="20" aria-hidden="true">
        <rect x="64" y="64" width="896" height="896" rx="200" fill="var(--accent)" />
        <rect x="300" y="250" width="110" height="524" rx="40" fill="#fff" />
        <path d="M430 512 L640 262 Q660 240 690 250 L748 268 Q770 278 755 300 L560 512 L755 724 Q770 746 748 756 L690 774 Q660 784 640 762 Z" fill="#fff" />
        <circle cx="700" cy="512" r="58" fill="#ffd43b" />
      </svg>
    </div>
    {#if ws.project}
      <button class="icon-btn" class:on={settings.layout.treeOpen} title="Toggle file tree ({fmt('Mod+B')})" aria-label="Toggle file tree" onclick={() => (settings.layout.treeOpen = !settings.layout.treeOpen)}>
        <PanelLeft size={17} />
      </button>
      <div class="project" title={ws.project.root}>
        <span class="name">{ws.project.name}</span>
        {#if ws.mainFile}<span class="main muted">· {ws.mainFile}</span>{/if}
      </div>
    {:else}
      <span class="title">KernTeX</span>
    {/if}
  </div>

  {#if ws.project}
    <div class="center">
      {#if ws.status.state === "running"}
        <button class="btn danger compile" onclick={() => ws.cancelCompile()} title="Stop compiling (Esc)">
          <Square size={14} fill="currentColor" /> Stop
        </button>
        <div class="status running" role="status" aria-live="polite">
          <span class="spinner"></span>
          <span class="stage">
            {#if ws.status.firstRun && ws.status.stage.startsWith("Downloading")}
              First run: downloading TeX packages – this happens once
            {:else}
              {ws.status.stage}
            {/if}
          </span>
          {#if ws.status.detail}<span class="detail muted">{ws.status.detail}</span>{/if}
          <span class="time muted">{secs(now - ws.status.startedAt)}</span>
        </div>
      {:else}
        <button class="btn primary compile" onclick={() => ws.compile()} title="Compile ({fmt('Mod+Enter')}, or save with {modKey}+S)">
          <Play size={14} fill="currentColor" /> Compile
        </button>
        {#if ws.status.state === "done"}
          {@const r = ws.status.result}
          <button class="status" onclick={() => (settings.layout.problemsOpen = true)} title="Show problems ({fmt('Mod+J')})">
            {#if r.canceled}
              <span class="muted">Stopped</span>
            {:else if counts.errors > 0}
              <CircleX size={15} color="var(--danger)" />
              <span>{counts.errors} error{counts.errors === 1 ? "" : "s"}</span>
            {:else if counts.warnings > 0}
              <TriangleAlert size={15} color="var(--warning)" />
              <span>{counts.warnings} warning{counts.warnings === 1 ? "" : "s"}</span>
            {:else}
              <CircleCheck size={15} color="var(--success)" />
              <span>Compiled</span>
            {/if}
            {#if !r.canceled}<span class="muted">{r.engineLabel} · {secs(r.durationMs)}</span>{/if}
          </button>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="right">
    {#if ws.project}
      <button class="icon-btn" title="Export PDF" aria-label="Export PDF" onclick={() => ws.exportPdf()}><Download size={17} /></button>
    {/if}
    <button class="palette-btn" onclick={() => (ui.palette = true)} title="Command palette">
      <Search size={14} /> <span>Commands</span> <kbd>{fmt("Mod+K")}</kbd>
    </button>
    <button class="icon-btn" title="Switch to {effectiveTheme() === 'dark' ? 'light' : 'dark'} mode" aria-label="Toggle theme" onclick={toggleTheme}>
      {#if effectiveTheme() === "dark"}<Sun size={17} />{:else}<Moon size={17} />{/if}
    </button>
    <button class="icon-btn" title="Settings ({fmt('Mod+,')})" aria-label="Settings" onclick={() => (ui.settings = true)}><Settings size={17} /></button>
    {#if ws.project}
      <button class="icon-btn" class:on={settings.layout.pdfOpen} title="Toggle PDF preview ({fmt('Mod+Shift+P')})" aria-label="Toggle PDF preview" onclick={() => (settings.layout.pdfOpen = !settings.layout.pdfOpen)}>
        <PanelRight size={17} />
      </button>
    {/if}
  </div>
</header>

<style>
  .toolbar {
    flex: none;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 12px;
    height: 46px;
    padding: 0 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .left,
  .right,
  .center {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .right {
    justify-content: flex-end;
  }
  .brand {
    display: grid;
    place-items: center;
    width: 28px;
  }
  .title {
    font-weight: 600;
  }
  .project {
    display: flex;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }
  .project .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .project .main {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .compile {
    min-width: 98px;
    justify-content: center;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    white-space: nowrap;
    cursor: pointer;
    animation: fade-in var(--dur) var(--ease);
  }
  .status:hover {
    background: var(--hover);
  }
  .status.running {
    cursor: default;
    min-width: 280px;
  }
  .status.running:hover {
    background: transparent;
  }
  .stage {
    font-weight: 500;
  }
  .detail {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .time {
    font-variant-numeric: tabular-nums;
  }
  .palette-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-muted);
    cursor: pointer;
    transition: border-color var(--dur) var(--ease), transform 80ms ease;
  }
  .palette-btn:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .palette-btn:active {
    transform: scale(0.97);
  }
  @media (max-width: 980px) {
    .palette-btn span,
    .project .main,
    .detail {
      display: none;
    }
  }
</style>
