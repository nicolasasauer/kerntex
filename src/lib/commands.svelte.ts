// Every user action in one place: the command palette lists them, global
// shortcuts trigger them, and tooltips show their shortcut.

import { ws } from "./workspace.svelte";
import { settings } from "./settings.svelte";
import { openUrl } from "./api";
import { LINKS, reportProblemUrl } from "./about";
import { openSearchPanel } from "@codemirror/search";

export const ui = $state({
  palette: false,
  settings: false,
  newProject: false,
  about: false,
  /** Bottom panel tab. */
  bottomTab: "problems" as "problems" | "details" | "log",
  /** Folder under the pointer while files are dragged over the tree ("" = root, null = none). */
  dropDir: null as string | null,
  /** Set by the file tree to start an inline "new file" input. */
  treeCreate: null as null | { dir: string; kind: "file" | "folder" },
});

export interface Command {
  id: string;
  title: string;
  group: string;
  /** Display form, e.g. "Ctrl+S". `Mod` = Ctrl (Cmd on macOS). */
  shortcut?: string;
  when?: () => boolean;
  run: () => unknown;
}

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);
export const modKey = isMac ? "⌘" : "Ctrl";
export function fmt(shortcut?: string) {
  return shortcut?.replace("Mod", modKey);
}

const hasProject = () => ws.project != null;

export const commands: Command[] = [
  { id: "compile", title: "Compile", group: "Build", shortcut: "Mod+Enter", when: hasProject, run: () => ws.compile() },
  { id: "save", title: "Save (and compile)", group: "File", shortcut: "Mod+S", when: hasProject, run: () => ws.save() },
  { id: "cancel", title: "Stop compiling", group: "Build", shortcut: "Escape", when: () => ws.status.state === "running", run: () => ws.cancelCompile() },
  { id: "palette", title: "Command palette", group: "View", shortcut: "Mod+K", run: () => (ui.palette = true) },
  { id: "newProject", title: "New project…", group: "Project", shortcut: "Mod+Shift+N", run: () => (ui.newProject = true) },
  { id: "openFolder", title: "Open folder…", group: "Project", shortcut: "Mod+O", run: () => ws.openFolderDialog() },
  { id: "importZip", title: "Import ZIP (e.g. from Overleaf)…", group: "Project", run: () => ws.importZip() },
  { id: "example", title: "Open example project", group: "Help", run: () => ws.openWelcome() },
  { id: "exampleTypst", title: "Open Typst example project", group: "Help", run: () => ws.openWelcome(false, "typst") },
  { id: "exampleFresh", title: "Open a fresh copy of the example project", group: "Help", run: () => ws.openWelcome(true) },
  { id: "closeProject", title: "Close project", group: "Project", when: hasProject, run: () => ws.closeProject() },
  { id: "newFile", title: "New file…", group: "File", when: hasProject, run: () => { settings.layout.treeOpen = true; ui.treeCreate = { dir: "", kind: "file" }; } },
  { id: "newFolder", title: "New folder…", group: "File", when: hasProject, run: () => { settings.layout.treeOpen = true; ui.treeCreate = { dir: "", kind: "folder" }; } },
  { id: "exportPdf", title: "Export PDF…", group: "Export", when: hasProject, run: () => ws.exportPdf() },
  { id: "exportZip", title: "Export project as ZIP…", group: "Export", when: hasProject, run: () => ws.exportZip(false) },
  { id: "exportCleanZip", title: "Export clean ZIP (for arXiv / journals)…", group: "Export", when: hasProject, run: () => ws.exportZip(true) },
  { id: "toggleTree", title: "Toggle file tree", group: "View", shortcut: "Mod+B", when: hasProject, run: () => (settings.layout.treeOpen = !settings.layout.treeOpen) },
  { id: "togglePdf", title: "Toggle PDF preview", group: "View", shortcut: "Mod+Shift+P", when: hasProject, run: () => (settings.layout.pdfOpen = !settings.layout.pdfOpen) },
  { id: "toggleProblems", title: "Toggle problems panel", group: "View", shortcut: "Mod+J", when: hasProject, run: () => (settings.layout.problemsOpen = !settings.layout.problemsOpen) },
  { id: "rawLog", title: "Show raw log", group: "Build", when: hasProject, run: () => { settings.layout.problemsOpen = true; ui.bottomTab = "log"; } },
  { id: "buildDetails", title: "Show build details (engine, packages, versions)", group: "Build", when: hasProject, run: () => { settings.layout.problemsOpen = true; settings.layout.problems = Math.max(settings.layout.problems, 320); ui.bottomTab = "details"; } },
  { id: "addFiles", title: "Add files to project…", group: "File", when: hasProject, run: () => ws.addFiles("") },
  { id: "find", title: "Find / replace in file", group: "Edit", shortcut: "Mod+F", when: () => ws.active != null, run: () => { if (ws.editor) openSearchPanel(ws.editor.view); } },
  { id: "settings", title: "Settings", group: "Preferences", shortcut: "Mod+,", run: () => (ui.settings = true) },
  { id: "themeLight", title: "Theme: Light", group: "Preferences", run: () => (settings.theme = "light") },
  { id: "themeDark", title: "Theme: Dark", group: "Preferences", run: () => (settings.theme = "dark") },
  { id: "themeSystem", title: "Theme: Follow system", group: "Preferences", run: () => (settings.theme = "system") },
  { id: "toggleCompileOnSave", title: "Toggle compile on save", group: "Preferences", run: () => (settings.compileOnSave = !settings.compileOnSave) },
  { id: "toggleWrap", title: "Toggle line wrapping", group: "Preferences", run: () => (settings.wrap = !settings.wrap) },
  { id: "zoomIn", title: "Editor: larger font", group: "View", shortcut: "Mod+=", run: () => (settings.fontSize = Math.min(28, settings.fontSize + 1)) },
  { id: "zoomOut", title: "Editor: smaller font", group: "View", shortcut: "Mod+-", run: () => (settings.fontSize = Math.max(9, settings.fontSize - 1)) },
  { id: "docs", title: "LaTeX help on CTAN", group: "Help", run: () => openUrl("https://www.ctan.org/") },
  { id: "github", title: "KernTeX on GitHub", group: "Help", run: () => openUrl(LINKS.repo) },
  { id: "about", title: "About KernTeX", group: "Help", run: () => (ui.about = true) },
  { id: "report", title: "Report a problem…", group: "Help", run: () => openUrl(reportProblemUrl()) },
];

export function commandById(id: string) {
  return commands.find((c) => c.id === id);
}

/** Matches a KeyboardEvent against "Mod+Shift+K" style shortcuts. */
function matches(e: KeyboardEvent, shortcut: string) {
  const parts = shortcut.split("+");
  const key = parts.pop()!;
  const mod = parts.includes("Mod");
  const shift = parts.includes("Shift");
  const ctrlOrMeta = isMac ? e.metaKey : e.ctrlKey;
  if (mod !== ctrlOrMeta || shift !== e.shiftKey || e.altKey) return false;
  const k = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (key === "=") return e.key === "=" || e.key === "+";
  return k === key.toUpperCase() || e.key === key;
}

export function handleGlobalKey(e: KeyboardEvent) {
  if (e.defaultPrevented) return; // already handled (e.g. Mod+S inside the editor)
  if (e.key === "Escape" && (ui.palette || ui.settings || ui.newProject)) return; // dialogs handle it
  for (const c of commands) {
    if (!c.shortcut || (c.when && !c.when())) continue;
    if (c.id === "find") continue; // CodeMirror handles Mod+F itself
    if (c.id === "cancel" && document.activeElement?.closest(".cm-editor")) continue;
    if (matches(e, c.shortcut)) {
      e.preventDefault();
      c.run();
      return;
    }
  }
}
