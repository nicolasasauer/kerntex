// MCP server, UI side: answers the backend's requests that need the editor
// (unsaved text, undoable edits, compiling with the usual UI), reports what
// the user allows, and keeps the list of connected agents and their actions.
// The protocol itself lives in src-tauri/src/mcp.

import { api, toAppError } from "./api";
import { settings } from "./settings.svelte";
import { toast } from "./toasts.svelte";
import { ws } from "./workspace.svelte";
import type { McpActivity, McpClient, McpRequest, McpSetup } from "./types";

/** Most entries kept in the activity list. */
const MAX_ACTIVITY = 100;

export const mcp = $state({
  clients: [] as McpClient[],
  activity: [] as McpActivity[],
  /** Where `kerntex-mcp` is; loaded when the setup dialog opens. */
  setup: null as McpSetup | null,
  /** The "AI assistants" dialog is open. */
  dialog: false,
});

const toolLabels: Record<string, string> = {
  list_files: "Listed the files",
  read_file: "Read",
  edit_file: "Edited",
  create_file: "Created",
  search: "Searched for",
  compile: "Compiled",
  get_diagnostics: "Checked the problems",
};

export function describeActivity(a: McpActivity): string {
  const label = toolLabels[a.tool] ?? a.tool;
  if (!a.detail) return label;
  return a.tool === "compile" ? `${label} (${a.detail})` : `${label} ${a.detail}`;
}

/** Name of the first connected agent, e.g. "Claude Code". */
export function agentName(): string {
  const names = [...new Set(mcp.clients.map((c) => c.name))];
  if (names.length === 0) return "";
  return names.length === 1 ? names[0] : `${names.length} AI assistants`;
}

let started = false;

export async function initMcp() {
  if (started) return;
  started = true;
  // Tell the backend what the user allows and which project is open, whenever that changes.
  $effect.root(() => {
    $effect(() => {
      const p = ws.project;
      const config = {
        enabled: settings.mcpEnabled,
        allowEdits: settings.mcpAllowEdits,
        root: p?.root ?? null,
        name: p?.name ?? null,
        mainFile: ws.mainFile,
      };
      api.mcpConfigure(config).catch(() => {});
    });
  });
  await Promise.all([
    api.onMcpRequest((r) => void handle(r)),
    api.onMcpClients((c) => (mcp.clients = c)),
    api.onMcpActivity((a) => {
      mcp.activity = [{ ...a, at: Date.now() }, ...mcp.activity].slice(0, MAX_ACTIVITY);
      if (a.ok && (a.tool === "edit_file" || a.tool === "create_file")) {
        toast({ kind: "info", source: a.client, title: describeActivity(a), body: a.tool === "edit_file" ? "Undo with Ctrl+Z in that file." : null }, 3500);
      }
    }),
  ]);
  api.mcpClients().then((c) => (mcp.clients = c)).catch(() => {});
}

export async function loadSetup() {
  try {
    mcp.setup = await api.mcpSetup();
  } catch (e) {
    mcp.setup = { command: null, problem: toAppError(e).title, endpoint: "", claudeCode: null };
  }
}

export async function disconnectAgents() {
  await api.mcpDisconnect();
  toast({ kind: "success", source: "KernTeX", title: "AI assistants disconnected", body: "They can connect again while “Allow AI assistants” is on." }, 4000);
}

async function handle(r: McpRequest) {
  try {
    const value = await answer(r);
    await api.mcpRespond(r.id, true, value, null);
  } catch (e) {
    const err = toAppError(e);
    await api.mcpRespond(r.id, false, null, [err.title, err.detail, err.hint].filter(Boolean).join(". ")).catch(() => {});
  }
}

function textTab(path: string) {
  return ws.tabs.find((t) => t.path === path && t.kind === "text");
}

async function answer(r: McpRequest): Promise<unknown> {
  const path = typeof r.args.path === "string" ? r.args.path : "";
  switch (r.op) {
    // What the editor shows for `path`, or null if it isn't open.
    case "read": {
      const tab = textTab(path);
      const text = tab ? ws.editor?.text(path) : null;
      return { text: text ?? null, dirty: tab?.dirty ?? false };
    }

    // Replace the text of `path` with `text`, if it still equals `base`.
    case "apply": {
      const base = String(r.args.base);
      const text = String(r.args.text);
      await ws.openInBackground(path);
      const current = ws.editor?.text(path);
      if (current == null) throw new Error(`${path} couldn't be opened in the editor.`);
      if (current !== base) {
        throw new Error(`${path} changed in the editor just now (the user is typing), so nothing was changed. Read it again and redo the edit.`);
      }
      if (!ws.editor?.applyText(path, text)) throw new Error(`${path} couldn't be changed in the editor.`);
      ws.onEdit(path, text);
      return {};
    }

    // Unsaved editor text, so search sees what the user sees.
    case "buffers": {
      const buffers: Record<string, string> = {};
      for (const t of ws.tabs) {
        if (t.kind !== "text" || !t.dirty) continue;
        const text = ws.editor?.text(t.path);
        if (text != null) buffers[t.path] = text;
      }
      return { buffers };
    }

    case "created":
      await ws.refreshTree();
      return {};

    // Like the Compile button: the toolbar shows progress and the PDF refreshes.
    case "compile": {
      if (!ws.mainFile) return { message: "The project has no main file. Ask the user to right-click the root .tex or .typ file in KernTeX's file tree → “Set as main file”." };
      while (ws.status.state === "running") await new Promise((res) => setTimeout(res, 100));
      const result = await ws.compile();
      return result ? {} : { message: "KernTeX didn't compile. See the problems panel in KernTeX." };
    }
  }
  throw new Error(`Unknown request ${(r as McpRequest).op}`);
}
