<script lang="ts">
  import Modal from "./Modal.svelte";
  import Bot from "@lucide/svelte/icons/bot";
  import Copy from "@lucide/svelte/icons/copy";
  import Check from "@lucide/svelte/icons/check";
  import Plug from "@lucide/svelte/icons/plug";
  import Unplug from "@lucide/svelte/icons/unplug";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { onMount } from "svelte";
  import { api, copyText, toAppError } from "../lib/api";
  import { settings } from "../lib/settings.svelte";
  import { toast, toastError } from "../lib/toasts.svelte";
  import { mcp, loadSetup, disconnectAgents, describeActivity, agentName } from "../lib/mcp.svelte";

  onMount(() => {
    loadSetup();
  });

  const command = $derived(mcp.setup?.command ?? null);
  const quoted = $derived(command && /\s/.test(command) ? `"${command}"` : command);
  const claudeCodeCmd = $derived(quoted ? `claude mcp add --scope user kerntex -- ${quoted}` : "");
  const jsonConfig = $derived(command ? JSON.stringify({ mcpServers: { kerntex: { command, args: [] } } }, null, 2) : "");

  let copied: string | null = $state(null);
  let busy: string | null = $state(null);

  async function copy(id: string, text: string) {
    if (await copyText(text)) {
      copied = id;
      setTimeout(() => copied === id && (copied = null), 1800);
    } else {
      toast({ kind: "error", source: "KernTeX", title: "Couldn't copy to the clipboard", body: text });
    }
  }

  /** Connecting only makes sense with MCP on — turn it on with the first connect. */
  function ensureEnabled() {
    if (!settings.mcpEnabled) settings.mcpEnabled = true;
  }

  async function connectClaudeCode() {
    ensureEnabled();
    busy = "claude";
    try {
      await api.mcpAddToClaudeCode();
      toast({ kind: "success", source: "Claude Code", title: "KernTeX added to Claude Code", body: "Start a new Claude Code session and ask it to work on your document." }, 6000);
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      busy = null;
    }
  }

  async function installLink(target: "cursor" | "vscode") {
    ensureEnabled();
    busy = target;
    try {
      await api.mcpInstallLink(target);
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      busy = null;
    }
  }

  function time(at?: number) {
    return at ? new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }) : "";
  }
</script>

<Modal title="AI assistants" width={620} onClose={() => (mcp.dialog = false)}>
  <div class="body">
    <p class="intro">
      Let an AI assistant such as Claude, Cursor or VS Code work on the project that is open in KernTeX: read files, make changes you see live and can undo
      with <kbd>Ctrl</kbd>+<kbd>Z</kbd>, compile and fix errors. KernTeX shares nothing over the network itself — but the assistant may send what it reads to its AI
      service.
    </p>

    <section>
      <div class="row">
        <label for="mcp-on">Allow AI assistants <small>Off: assistants are told to ask you to turn this on, and see no project data</small></label>
        <input id="mcp-on" type="checkbox" class="switch" bind:checked={settings.mcpEnabled} />
      </div>
      <div class="row">
        <label for="mcp-edit">Allow changes to files <small>Off: assistants can read and compile, but not edit</small></label>
        <input id="mcp-edit" type="checkbox" class="switch" bind:checked={settings.mcpAllowEdits} disabled={!settings.mcpEnabled} />
      </div>
    </section>

    <section>
      <h3>Connect an assistant <small class="muted">once per app — it then works with whatever project is open</small></h3>
      {#if mcp.setup && !command}
        <p class="problem"><TriangleAlert size={15} /> {mcp.setup.problem}</p>
      {:else if !mcp.setup}
        <p class="muted">Looking for kerntex-mcp…</p>
      {:else}
        <ul class="clients">
          <li>
            <div class="who"><b>Claude Code</b><small class="muted">{mcp.setup.claudeCode ? "Found on this computer" : "Not found — run this command in a terminal after installing it"}</small></div>
            <div class="actions">
              {#if mcp.setup.claudeCode}
                <button class="btn primary" onclick={connectClaudeCode} disabled={busy === "claude"}><Plug size={14} /> {busy === "claude" ? "Connecting…" : "Connect"}</button>
              {/if}
              <button class="btn" onclick={() => { ensureEnabled(); copy("cc", claudeCodeCmd); }} title={claudeCodeCmd}>
                {#if copied === "cc"}<Check size={14} /> Copied{:else}<Copy size={14} /> Copy command{/if}
              </button>
            </div>
          </li>
          <li>
            <div class="who"><b>Cursor</b><small class="muted">Opens Cursor's “Install MCP server” dialog</small></div>
            <div class="actions">
              <button class="btn" onclick={() => installLink("cursor")} disabled={busy === "cursor"}><Plug size={14} /> Add to Cursor</button>
            </div>
          </li>
          <li>
            <div class="who"><b>VS Code</b><small class="muted">Opens VS Code's “Install MCP server” dialog (needs Copilot Chat)</small></div>
            <div class="actions">
              <button class="btn" onclick={() => installLink("vscode")} disabled={busy === "vscode"}><Plug size={14} /> Add to VS Code</button>
            </div>
          </li>
          <li>
            <div class="who">
              <b>Claude Desktop and others</b>
              <small class="muted">Claude: Settings → Developer → Edit config, paste, restart Claude. Other apps: add a local (stdio) MCP server with this configuration.</small>
            </div>
            <div class="actions">
              <button class="btn" onclick={() => { ensureEnabled(); copy("json", jsonConfig); }}>
                {#if copied === "json"}<Check size={14} /> Copied{:else}<Copy size={14} /> Copy configuration{/if}
              </button>
            </div>
          </li>
        </ul>
        <p class="cmd muted">Command: <code>{command}</code></p>
      {/if}
    </section>

    <section>
      <h3>Status</h3>
      {#if mcp.clients.length > 0}
        <div class="row">
          <span class="connected" class:blocked={!settings.mcpEnabled}>
            <Bot size={16} /> {agentName()} connected{#if !settings.mcpEnabled}<small> — but sees nothing while “Allow AI assistants” is off</small>{/if}
          </span>
          <button class="btn danger" onclick={disconnectAgents}><Unplug size={14} /> Disconnect</button>
        </div>
      {:else}
        <p class="muted">
          No assistant connected. After connecting, open your assistant and ask, for example: “Compile my document in KernTeX and fix the errors.”
        </p>
      {/if}
      {#if mcp.activity.length > 0}
        <ol class="activity" aria-label="What assistants did">
          {#each mcp.activity.slice(0, 30) as a (a.at + a.tool + a.detail)}
            <li class:failed={!a.ok}>
              <span class="t muted">{time(a.at)}</span>
              <span class="c">{a.client}</span>
              <span class="d">{describeActivity(a)}{#if a.error}<small title={a.error}> — refused: {a.error}</small>{/if}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  </div>
</Modal>

<style>
  .body {
    padding: 0 20px 20px;
    overflow: auto;
    max-height: min(78vh, 760px);
  }
  .intro {
    margin: 0 0 4px;
    line-height: 1.5;
    color: var(--text-muted);
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
  h3 small {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    margin-left: 6px;
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
  label small,
  .who small {
    color: var(--text-muted);
    font-size: 12px;
  }
  .switch:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .clients {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .clients li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
  .cmd {
    font-size: 12px;
    margin: 8px 0 0;
    word-break: break-all;
  }
  code {
    font-family: var(--font-mono);
    user-select: all;
  }
  .problem {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    color: var(--warning);
    margin: 0;
    line-height: 1.45;
  }
  .connected {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    font-weight: 600;
    color: var(--success);
  }
  .connected.blocked {
    color: var(--text-muted);
  }
  .connected small {
    font-weight: 400;
  }
  .activity {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    font-size: 12.5px;
    max-height: 180px;
    overflow: auto;
  }
  .activity li {
    display: grid;
    grid-template-columns: 70px 110px 1fr;
    gap: 8px;
    padding: 3px 0;
    border-top: 1px solid var(--border);
  }
  .activity .c {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .activity .d {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .activity li.failed .d {
    color: var(--danger);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 11px;
  }
</style>
