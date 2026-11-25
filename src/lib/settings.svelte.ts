// App-wide preferences, persisted in localStorage (per machine).
import type { Engine, TexFlavor } from "./types";

export type ThemePref = "system" | "light" | "dark";

export interface RecentProject {
  root: string;
  name: string;
  openedAt: number;
}

const KEY = "kerntex.settings.v1";

const defaults = {
  theme: "system" as ThemePref,
  engine: "tectonic" as Engine,
  flavor: "pdflatex" as TexFlavor,
  compileOnSave: true,
  autosave: true,
  autosaveDelayMs: 1500,
  fontSize: 14,
  wrap: true,
  showGenerated: false,
  pdfZoom: "fit-width" as "fit-width" | "fit-page" | number,
  recent: [] as RecentProject[],
  welcomed: false,
  /** MCP server: AI agents may work on the open project. Off until the user turns it on. */
  mcpEnabled: false,
  /** When off, agents may read and compile but not change files. */
  mcpAllowEdits: true,
  layout: { tree: 240, pdf: 0.45, problems: 200, treeOpen: true, pdfOpen: true, problemsOpen: false, side: "files" as "files" | "git" },
};

export type Settings = typeof defaults;

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...defaults, ...parsed, layout: { ...defaults.layout, ...(parsed.layout ?? {}) } };
    }
  } catch {
    /* private mode or corrupt: use defaults */
  }
  return structuredClone(defaults);
}

export const settings: Settings = $state(load());

let timer: ReturnType<typeof setTimeout> | undefined;
$effect.root(() => {
  $effect(() => {
    const snapshot = JSON.stringify(settings);
    clearTimeout(timer);
    timer = setTimeout(() => {
      try {
        localStorage.setItem(KEY, snapshot);
      } catch {
        /* ignore */
      }
    }, 200);
  });
});

export function addRecent(root: string, name: string) {
  settings.recent = [{ root, name, openedAt: Date.now() }, ...settings.recent.filter((r) => r.root !== root)].slice(0, 12);
}

export function removeRecent(root: string) {
  settings.recent = settings.recent.filter((r) => r.root !== root);
}

// --- Theme ---
const media = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
let systemDark = $state(media?.matches ?? false);
media?.addEventListener("change", (e) => (systemDark = e.matches));

export function effectiveTheme(): "light" | "dark" {
  return settings.theme === "system" ? (systemDark ? "dark" : "light") : settings.theme;
}
