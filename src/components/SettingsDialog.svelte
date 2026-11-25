<script lang="ts">
  import Modal from "./Modal.svelte";
  import { settings } from "../lib/settings.svelte";
  import { ui } from "../lib/commands.svelte";
  import { ws } from "../lib/workspace.svelte";
  import type { Engine, TexFlavor } from "../lib/types";
  import { APP_VERSION } from "../lib/about";
  import { mcp, agentName } from "../lib/mcp.svelte";

  const p = $derived(ws.project);

  function setProjectEngine(v: string) {
    ws.updateProjectSettings({ engine: v === "default" ? undefined : (v as Engine) });
  }
  function setProjectFlavor(v: string) {
    ws.updateProjectSettings({ flavor: v === "default" ? undefined : (v as TexFlavor) });
  }
</script>

<Modal title="Settings" width={560} onClose={() => (ui.settings = false)}>
  <div class="body">
    <section>
      <h3>Appearance</h3>
      <div class="row">
        <label for="theme">Theme</label>
        <div class="seg" id="theme" role="radiogroup" aria-label="Theme">
          {#each [["system", "System"], ["light", "Light"], ["dark", "Dark"]] as [v, l]}
            <button role="radio" aria-checked={settings.theme === v} class:on={settings.theme === v} onclick={() => (settings.theme = v as typeof settings.theme)}>{l}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <label for="fs">Editor font size</label>
        <div class="num">
          <input id="fs" type="range" min="10" max="24" bind:value={settings.fontSize} />
          <span>{settings.fontSize}px</span>
        </div>
      </div>
      <div class="row">
        <label for="wrap">Wrap long lines</label>
        <input id="wrap" type="checkbox" class="switch" bind:checked={settings.wrap} />
      </div>
    </section>

    <section>
      <h3>Compiling</h3>
      <div class="row">
        <label for="cos">Compile when saving <small>{"Ctrl+S saves all files, then compiles"}</small></label>
        <input id="cos" type="checkbox" class="switch" bind:checked={settings.compileOnSave} />
      </div>
      <div class="row">
        <label for="as">Autosave <small>Saves shortly after you stop typing</small></label>
        <input id="as" type="checkbox" class="switch" bind:checked={settings.autosave} />
      </div>
      {#if settings.autosave}
        <div class="row">
          <label for="asd">Autosave delay</label>
          <select id="asd" class="input" bind:value={settings.autosaveDelayMs}>
            <option value={800}>0.8 s</option>
            <option value={1500}>1.5 s</option>
            <option value={3000}>3 s</option>
            <option value={5000}>5 s</option>
          </select>
        </div>
      {/if}
      <div class="row">
        <label for="eng">Default engine <small>For projects without their own choice</small></label>
        <select id="eng" class="input" bind:value={settings.engine}>
          <option value="tectonic">Tectonic (built in, downloads packages)</option>
          <option value="latexmk">latexmk (your TeX Live / MiKTeX)</option>
        </select>
      </div>
      {#if settings.engine === "latexmk"}
        <div class="row">
          <label for="fl">LaTeX flavor</label>
          <select id="fl" class="input" bind:value={settings.flavor}>
            <option value="pdflatex">pdfLaTeX</option>
            <option value="xelatex">XeLaTeX</option>
            <option value="lualatex">LuaLaTeX</option>
          </select>
        </div>
      {/if}
      <p class="engines muted">
        Tectonic: {ws.engines?.tectonic ? "found" : "not found"} · latexmk: {ws.engines?.latexmk ? "found" : "not found"}
      </p>
    </section>

    <section>
      <h3>AI assistants</h3>
      <div class="row">
        <label for="mcp-open">Claude, Cursor, VS Code … <small>{mcp.clients.length > 0 ? `${agentName()} connected` : settings.mcpEnabled ? "Allowed — no assistant connected" : "Off"}</small></label>
        <button id="mcp-open" class="btn" onclick={() => { ui.settings = false; mcp.dialog = true; }}>Set up…</button>
      </div>
    </section>

    {#if p}
      <section>
        <h3>This project <small class="muted">saved in .kerntex/project.json</small></h3>
        <div class="row">
          <label for="pmain">Main file</label>
          <span class="mono">{ws.mainFile ?? "—"}</span>
        </div>
        {#if ws.isTypst}
          <div class="row">
            <span>Engine</span>
            <span>Typst {ws.engines?.typstVersion ?? ""} <small class="muted">built in — Typst documents always use it</small></span>
          </div>
        {:else}
        <div class="row">
          <label for="peng">Engine</label>
          <select id="peng" class="input" value={p.settings.engine ?? "default"} onchange={(e) => setProjectEngine(e.currentTarget.value)}>
            <option value="default">Use default ({p.hints.hasLatexmkrc ? "latexmk — latexmkrc found" : settings.engine})</option>
            <option value="tectonic">Tectonic</option>
            <option value="latexmk">latexmk</option>
          </select>
        </div>
        {#if ws.engine === "latexmk"}
          <div class="row">
            <label for="pfl">Flavor</label>
            <select id="pfl" class="input" value={p.settings.flavor ?? "default"} onchange={(e) => setProjectFlavor(e.currentTarget.value)}>
              <option value="default">Use default ({p.hints.flavor ? `${p.hints.flavor} — from % !TEX program` : settings.flavor})</option>
              <option value="pdflatex">pdfLaTeX</option>
              <option value="xelatex">XeLaTeX</option>
              <option value="lualatex">LuaLaTeX</option>
            </select>
          </div>
        {/if}
        {/if}
      </section>
    {/if}
    <footer>
      <button class="about" onclick={() => { ui.settings = false; ui.about = true; }}>About KernTeX {APP_VERSION}</button>
    </footer>
  </div>
</Modal>

<style>
  .body {
    padding: 4px 20px 20px;
    overflow: auto;
  }
  section {
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }
  section:last-child {
    border-bottom: none;
  }
  h3 {
    margin: 6px 0 8px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 40px;
  }
  label {
    display: flex;
    flex-direction: column;
  }
  label small {
    color: var(--text-muted);
    font-size: 12px;
  }
  select.input {
    max-width: 290px;
  }
  .seg {
    display: flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
  }
  .seg button {
    height: 26px;
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
    color: var(--text-muted);
    transition: background var(--dur) var(--ease);
  }
  .seg button:active {
    transform: scale(0.97);
  }
  .seg button.on {
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }
  .num {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .num span {
    width: 36px;
    font-variant-numeric: tabular-nums;
  }
  .engines {
    font-size: 12px;
    margin: 8px 0 0;
  }
  footer {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .about {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
  }
  .about:hover {
    text-decoration: underline;
  }
  .mono {
    font-family: var(--font-mono);
  }
</style>
