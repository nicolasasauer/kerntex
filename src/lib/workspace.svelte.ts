// The open project: files, tabs, compile state, diagnostics. UI components
// read this state and call its methods; all backend access goes through `api`.

import { api, joinPath, parentDir, pickFiles, pickFolder, pickSavePath, pickZip, confirmDialog, toAppError } from "./api";
import { settings, addRecent, removeRecent } from "./settings.svelte";
import { toast, toastError } from "./toasts.svelte";
import { usepackageInsertPos } from "./latex/packages";
import type { EditorController } from "./latex/editor";
import type {
  CompileResult, Diagnostic, EngineStatus, FileNode, ProjectInfo, ProjectRefs, ProjectSettings, Progress, Template, TexFlavor, Engine,
} from "./types";

export interface Tab {
  path: string;
  /** "preview" = image or PDF figure, shown read-only. */
  kind: "text" | "preview";
  /** Text as last saved to disk. */
  saved: string;
  dirty: boolean;
}

export type CompileStatus =
  | { state: "idle" }
  | { state: "running"; stage: string; detail: string | null; startedAt: number; firstRun: boolean }
  | { state: "done"; result: CompileResult };

/** Builds faster than this keep compiling on every autosave (setting "fast"). */
const FAST_BUILD_MS = 5000;
const TEXT_EXT = /\.(tex|typ|bib|sty|cls|txt|md|bbx|cbx|dtx|ins|ltx|cfg|def|bst|latexmkrc|tikz|csv|dat|json|yaml|yml|lua|py|r|m|gitignore)$/i;

const PREVIEW_EXT = /\.(png|jpe?g|gif|svg|webp|bmp|pdf)$/i;

export function isPreviewable(path: string) {
  return PREVIEW_EXT.test(path);
}

export function isImage(path: string) {
  return /\.(png|jpe?g|gif|svg|webp|bmp|pdf|eps)$/i.test(path);
}

export function isTextFile(path: string) {
  const name = path.split("/").pop() ?? path;
  return TEXT_EXT.test(name) || name === "latexmkrc" || name === ".latexmkrc" || !name.includes(".");
}

class Workspace {
  project: ProjectInfo | null = $state(null);
  tabs: Tab[] = $state([]);
  active: string | null = $state(null);
  refs: ProjectRefs = $state({ labels: [], citations: [] });
  status: CompileStatus = $state({ state: "idle" });
  diagnostics: Diagnostic[] = $state([]);
  log = $state("");
  /** Bumped whenever a new PDF is available. */
  pdfVersion = $state(0);
  engines: EngineStatus | null = $state(null);
  busy: string | null = $state(null);
  /** Bumped after KernTeX writes to the project folder (the Git panel refreshes on it). */
  fsVersion = $state(0);
  editor: EditorController | null = null;
  /** Most recent text tab: where "Insert figure" goes. */
  lastTextTab: string | null = null;

  #pendingCompile = false;
  /** Duration of the last successful and of the last finished build (ms), for auto-compile. */
  #lastOkBuildMs: number | null = null;
  #lastBuildMs: number | null = null;
  #toldAutoCompilePaused = false;
  #autosaveTimer: ReturnType<typeof setTimeout> | undefined;
  #unlisten: (() => void) | null = null;

  get mainFile(): string | null {
    return this.project?.settings.mainFile ?? this.project?.mainFile ?? null;
  }

  get engine(): Engine {
    return this.project?.settings.engine ?? (this.project?.hints.hasLatexmkrc ? "latexmk" : settings.engine);
  }

  get flavor(): TexFlavor {
    return this.project?.settings.flavor ?? this.project?.hints.flavor ?? settings.flavor;
  }

  get dirtyCount() {
    return this.tabs.filter((t) => t.dirty).length;
  }

  async init() {
    this.#unlisten = await api.onProgress((p: Progress) => {
      if (this.status.state === "running") {
        this.status = { ...this.status, stage: p.stage, detail: p.detail };
      }
    });
    api.detectEngines().then((e) => (this.engines = e)).catch(() => {});
  }

  dispose() {
    this.#unlisten?.();
  }

  // ---------- Projects ----------

  async openProject(root: string, opts: { compile?: boolean } = {}) {
    if (this.project && !(await this.confirmDiscard())) return;
    this.busy = "Opening project…";
    try {
      const info = await api.openProject(root);
      this.closeAllTabs();
      this.project = info;
      this.#lastOkBuildMs = this.#lastBuildMs = null;
      this.#toldAutoCompilePaused = false;
      this.diagnostics = [];
      this.log = "";
      this.status = { state: "idle" };
      addRecent(info.root, info.name);
      await this.loadRefs();
      const main = this.mainFile;
      if (main) await this.openFile(main);
      else toast({ kind: "info", source: "KernTeX", title: "No main file found", body: "Right-click your root .tex file in the file tree and choose “Set as main file”." });
      this.pdfVersion++;
      if (opts.compile !== false && main) this.compile();
    } catch (e) {
      const err = toAppError(e);
      toastError(err);
      if (err.source === "fileSystem") removeRecent(root);
    } finally {
      this.busy = null;
    }
  }

  async closeProject() {
    if (!(await this.confirmDiscard())) return;
    this.closeAllTabs();
    this.project = null;
    this.diagnostics = [];
    this.status = { state: "idle" };
  }

  async openFolderDialog() {
    const dir = await pickFolder("Open a LaTeX project folder");
    if (dir) await this.openProject(dir);
  }

  /** True when the main file is a Typst document (compiled by the built-in Typst). */
  get isTypst(): boolean {
    return /\.typ$/i.test(this.mainFile ?? "");
  }

  async openWelcome(fresh = false, kind: "latex" | "typst" = "latex") {
    try {
      this.busy = "Preparing example project…";
      const root = kind === "typst" ? await api.typstWelcomeProject(fresh) : await api.welcomeProject(fresh);
      settings.welcomed = true;
      await this.openProject(root);
      toast({ kind: "info", source: "KernTeX", title: "Example project", body: `Saved in ${root}` }, 6000);
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      this.busy = null;
    }
  }

  async createProject(parent: string, name: string, template: Template) {
    try {
      const root = await api.newProject(parent, name, template);
      await this.openProject(root);
      return true;
    } catch (e) {
      toastError(toAppError(e));
      return false;
    }
  }

  async importZip(zipPath?: string) {
    const zip = zipPath ?? (await pickZip());
    if (!zip) return;
    const dest = await pickFolder("Where should the project be extracted?") ;
    if (!dest) return;
    try {
      this.busy = "Importing ZIP…";
      const root = await api.importZip(zip, dest);
      toast({ kind: "success", source: "ZIP", title: "Project imported", body: root });
      await this.openProject(root);
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      this.busy = null;
    }
  }

  async exportZip(clean: boolean) {
    const p = this.project;
    if (!p) return;
    await this.saveAll();
    const dest = await pickSavePath(clean ? "Export clean ZIP" : "Export project as ZIP", joinPath(parentDir(p.root), `${p.name}${clean ? "-clean" : ""}.zip`), "zip", "ZIP archive");
    if (!dest) return;
    try {
      const n = await api.exportZip(p.root, dest, clean);
      toast({ kind: "success", source: "ZIP", title: `Exported ${n} files`, body: dest });
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async exportPdf() {
    const p = this.project;
    const main = this.mainFile;
    if (!p || !main) return;
    const stem = main.split("/").pop()!.replace(/\.tex$/i, "");
    const dest = await pickSavePath("Export PDF", joinPath(parentDir(p.root), `${stem}.pdf`), "pdf", "PDF document");
    if (!dest) return;
    try {
      await api.exportPdf(p.root, main, dest);
      toast({ kind: "success", source: "KernTeX", title: "PDF exported", body: dest });
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async updateProjectSettings(patch: Partial<ProjectSettings>) {
    const p = this.project;
    if (!p) return;
    const next = { ...p.settings, ...patch };
    try {
      await api.saveProjectSettings(p.root, next);
      p.settings = next;
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async setMainFile(path: string) {
    await this.updateProjectSettings({ mainFile: path });
    toast({ kind: "success", source: "KernTeX", title: `Main file: ${path}` });
    this.compile();
  }

  async refreshTree() {
    const p = this.project;
    if (!p) return;
    try {
      p.tree = await api.readTree(p.root);
      this.fsVersion++;
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  /** After files changed on disk (Git pull, merge abort): update the tree and open tabs. */
  async reloadFromDisk() {
    const p = this.project;
    if (!p) return;
    await this.refreshTree();
    let reopened = false;
    for (const t of [...this.tabs]) {
      if (t.kind !== "text" || t.dirty) continue;
      let text: string;
      try {
        text = await api.readFile(p.root, t.path);
      } catch {
        // Gone (deleted by the pull): close the tab without saving it back.
        this.tabs.splice(this.tabs.indexOf(t), 1);
        this.editor?.close(t.path);
        if (this.active === t.path) this.active = null;
        continue;
      }
      if (text === t.saved) continue;
      t.saved = text;
      this.editor?.replaceAll(t.path, text);
      // Tabs in the background lose their editor state on replaceAll; recreate it.
      if (this.editor && !this.editor.has(t.path)) {
        this.editor.open(t.path, text);
        reopened = true;
      }
    }
    if (!this.active && this.tabs[0]) this.activate(this.tabs[0].path);
    else if (reopened) {
      const show = this.tabs.find((t) => t.path === this.active)?.kind === "text" ? this.active : this.lastTextTab;
      if (show && this.tabs.some((t) => t.path === show)) this.editor?.open(show, "");
    }
    this.loadRefs();
  }

  async loadRefs() {
    const p = this.project;
    if (!p) return;
    try {
      this.refs = await api.scanRefs(p.root);
    } catch {
      /* completion just won't know labels */
    }
  }

  // ---------- Files & tabs ----------

  async openFile(path: string, line?: number, column?: number, endColumn?: number) {
    const p = this.project;
    if (!p) return;
    if (isPreviewable(path)) {
      if (!this.tabs.some((t) => t.path === path)) this.tabs.push({ path, kind: "preview", saved: "", dirty: false });
      this.active = path;
      return;
    }
    if (!isTextFile(path)) {
      toast({ kind: "info", source: "KernTeX", title: `“${path.split("/").pop()}” can't be opened here`, body: "KernTeX edits text files and previews images and PDFs. Use “Show in file manager” to open it with another program." });
      return;
    }
    if (!this.tabs.some((t) => t.path === path)) {
      try {
        const text = await api.readFile(p.root, path);
        // Opened meanwhile (e.g. by an AI assistant's edit): just switch to it.
        if (this.tabs.some((t) => t.path === path)) this.editor?.open(path, "");
        else {
          this.tabs.push({ path, kind: "text", saved: text, dirty: false });
          this.editor?.open(path, text);
        }
      } catch (e) {
        toastError(toAppError(e));
        return;
      }
    } else if (this.active !== path) {
      this.editor?.open(path, "");
    }
    this.lastTextTab = path;
    this.active = path;
    if (line) queueMicrotask(() => this.editor?.gotoLine(line, column, endColumn));
  }

  activate(path: string) {
    if (this.active === path) return;
    if (this.tabs.find((t) => t.path === path)?.kind === "text") {
      this.editor?.open(path, "");
      this.lastTextTab = path;
    }
    this.active = path;
  }

  async closeTab(path: string) {
    const tab = this.tabs.find((t) => t.path === path);
    if (!tab) return;
    if (tab.kind === "text" && tab.dirty) {
      await this.saveFile(path);
    }
    const i = this.tabs.indexOf(tab);
    this.tabs.splice(i, 1);
    this.editor?.close(path);
    if (this.active === path) {
      const next = this.tabs[i] ?? this.tabs[i - 1];
      this.active = null;
      if (next) this.activate(next.path);
    }
  }

  closeAllTabs() {
    for (const t of this.tabs) this.editor?.close(t.path);
    this.tabs = [];
    this.active = null;
  }

  /** Called by the editor on every change. */
  onEdit(path: string, text: string) {
    const tab = this.tabs.find((t) => t.path === path);
    if (!tab) return;
    tab.dirty = text !== tab.saved;
    if (settings.autosave && tab.dirty) {
      clearTimeout(this.#autosaveTimer);
      this.#autosaveTimer = setTimeout(() => this.save({ compile: this.#autosaveShouldCompile(), quiet: true }), settings.autosaveDelayMs);
    }
  }

  /** Quick documents compile on every autosave (feels live); long builds only on
   *  Ctrl+S or Compile, so the engine doesn't run all the time. */
  #autosaveShouldCompile(): boolean {
    if (!settings.compileOnSave || settings.autoCompile === "off") return false;
    if (settings.autoCompile === "always") return true;
    const last = this.#lastOkBuildMs ?? this.#lastBuildMs;
    if (last == null || last < FAST_BUILD_MS) return true;
    if (!this.#toldAutoCompilePaused) {
      this.#toldAutoCompilePaused = true;
      toast({
        kind: "info",
        source: "KernTeX",
        title: "Saved without compiling",
        body: `This document takes ${Math.round(last / 1000)} s to build, so autosave doesn't compile it. Press Ctrl+S or Compile to update the PDF (Settings → Compiling).`,
      });
    }
    return false;
  }

  async saveFile(path: string): Promise<boolean> {
    const p = this.project;
    const tab = this.tabs.find((t) => t.path === path);
    if (!p || !tab || tab.kind !== "text") return true;
    const text = this.editor?.text(path);
    if (text == null || text === tab.saved) {
      tab.dirty = false;
      return true;
    }
    try {
      await api.writeFile(p.root, path, text);
      this.fsVersion++;
      tab.saved = text;
      tab.dirty = this.editor?.text(path) !== text;
      return true;
    } catch (e) {
      toastError(toAppError(e));
      return false;
    }
  }

  async saveAll() {
    clearTimeout(this.#autosaveTimer);
    const results = await Promise.all(this.tabs.filter((t) => t.dirty).map((t) => this.saveFile(t.path)));
    return results.every(Boolean);
  }

  /** Ctrl+S: save everything, then compile (if enabled). */
  async save(opts: { compile?: boolean; quiet?: boolean } = {}) {
    const hadChanges = this.dirtyCount > 0;
    const ok = await this.saveAll();
    if (!ok) return;
    if (hadChanges) this.loadRefs();
    if (opts.compile ?? settings.compileOnSave) this.compile();
    else if (!opts.quiet) toast({ kind: "success", source: "KernTeX", title: "Saved" }, 1500);
  }

  async confirmDiscard(): Promise<boolean> {
    if (this.dirtyCount === 0) return true;
    return this.saveAll();
  }

  async newFile(dir: string, name: string) {
    const p = this.project;
    if (!p) return;
    const path = dir ? `${dir}/${name}` : name;
    try {
      await api.createFile(p.root, path);
      await this.refreshTree();
      await this.openFile(path);
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async newFolder(dir: string, name: string) {
    const p = this.project;
    if (!p) return;
    try {
      await api.createDir(p.root, dir ? `${dir}/${name}` : name);
      await this.refreshTree();
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async rename(from: string, newName: string) {
    const p = this.project;
    if (!p || !newName.trim()) return;
    const dir = from.includes("/") ? from.slice(0, from.lastIndexOf("/")) : "";
    const to = dir ? `${dir}/${newName.trim()}` : newName.trim();
    if (to === from) return;
    await this.saveAll();
    try {
      await api.renamePath(p.root, from, to);
      for (const t of this.tabs) {
        if (t.path === from || t.path.startsWith(from + "/")) {
          const np = to + t.path.slice(from.length);
          this.editor?.rename(t.path, np);
          if (this.active === t.path) this.active = np;
          t.path = np;
        }
      }
      if (this.mainFile === from) await this.updateProjectSettings({ mainFile: to });
      await this.refreshTree();
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  async trash(path: string) {
    const p = this.project;
    if (!p) return;
    if (!(await confirmDialog(`Move “${path}” to the trash?\n\nYou can restore it from your system's trash.`))) return;
    try {
      await api.trashPath(p.root, path);
      for (const t of [...this.tabs]) {
        if (t.path === path || t.path.startsWith(path + "/")) {
          this.tabs.splice(this.tabs.indexOf(t), 1);
          this.editor?.close(t.path);
          if (this.active === t.path) this.active = null;
        }
      }
      if (!this.active && this.tabs[0]) this.activate(this.tabs[0].path);
      await this.refreshTree();
      toast({ kind: "success", source: "File system", title: `Moved “${path}” to the trash` });
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  /** Copies files from outside into `dir` ("" = project root). */
  async addFiles(dir: string, sources?: string[]) {
    const p = this.project;
    if (!p) return;
    const files = sources ?? (await pickFiles(dir ? `Add files to “${dir}”` : "Add files to the project"));
    if (files.length === 0) return;
    try {
      const added = await api.importFiles(p.root, dir, files);
      await this.refreshTree();
      const first = added[0];
      const target = this.lastTextTab && this.tabs.some((t) => t.path === this.lastTextTab) ? this.lastTextTab : null;
      const renamed = added.filter((a, i) => a.split("/").pop() !== files[i].split(/[\\/]/).pop());
      toast(
        {
          kind: "success",
          source: "File system",
          title: added.length === 1 ? `Added “${first}”` : `Added ${added.length} items to ${dir || "the project"}`,
          body: renamed.length ? `A file with the same name already existed, so the copy was renamed: ${renamed.join(", ")}` : null,
          action:
            added.length === 1 && isImage(first) && target?.endsWith(".tex")
              ? { label: `Insert figure into ${target.split("/").pop()}`, run: () => this.insertFigure(first, target) }
              : added.length === 1 && isPreviewable(first)
                ? { label: "Preview", run: () => this.openFile(first) }
                : undefined,
        },
        8000,
      );
    } catch (e) {
      toastError(toAppError(e));
    }
  }

  /** `\includegraphics` path for `file`, relative to the main file's folder (Overleaf convention). */
  graphicsPath(file: string): string {
    const main = this.mainFile ?? "";
    const base = main.includes("/") ? main.slice(0, main.lastIndexOf("/") + 1) : "";
    return base && file.startsWith(base) ? file.slice(base.length) : file;
  }

  figureSnippet(file: string): string {
    const label = (file.split("/").pop() ?? "").replace(/\.[^.]+$/, "").replace(/[^a-zA-Z0-9-]+/g, "-");
    return [
      "\\begin{figure}[htbp]",
      "  \\centering",
      `  \\includegraphics[width=0.8\\linewidth]{${this.graphicsPath(file)}}`,
      "  \\caption{}",
      `  \\label{fig:${label}}`,
      "\\end{figure}",
      "",
    ].join("\n");
  }

  async insertFigure(file: string, into?: string | null) {
    const target = into ?? this.lastTextTab;
    if (!target) return;
    this.activate(target);
    await Promise.resolve();
    this.editor?.insertAtCursor(this.figureSnippet(file));
  }

  /** Opens `path` as a tab without switching to it (for edits the user didn't make by typing). */
  async openInBackground(path: string) {
    const p = this.project;
    if (!p || this.tabs.some((t) => t.path === path)) return;
    const text = await api.readFile(p.root, path);
    if (this.tabs.some((t) => t.path === path)) return;
    this.tabs.push({ path, kind: "text", saved: text, dirty: false });
    const cur = this.active;
    this.editor?.open(path, text);
    if (cur && cur !== path && this.tabs.find((t) => t.path === cur)?.kind === "text") this.editor?.open(cur, "");
    else if (!cur) this.active = path;
  }

  /** Quick fix: add \usepackage{pkg} to the main file's preamble. */
  async addPackage(pkg: string) {
    const main = this.mainFile;
    const p = this.project;
    if (!main || !p) return;
    await this.openInBackground(main);
    const text = this.editor?.text(main) ?? "";
    const pos = usepackageInsertPos(text);
    this.editor?.insertAt(main, pos, `${pos > 0 ? "\n" : ""}\\usepackage{${pkg}}`);
    const tab = this.tabs.find((t) => t.path === main);
    if (tab) tab.dirty = true;
    toast({ kind: "success", source: "KernTeX", title: `Added \\usepackage{${pkg}} to ${main}`, action: { label: "Show", run: () => this.openFile(main, text.slice(0, pos).split("\n").length + 1) } });
  }

  preambleFor(path: string): string | null {
    const main = this.mainFile;
    if (!main || main === path) return null;
    return this.editor?.text(main) ?? null;
  }

  // ---------- Compile ----------

  /** Saves all files and compiles the main file. Resolves with the result (null if nothing ran). */
  async compile(): Promise<CompileResult | null> {
    const p = this.project;
    const main = this.mainFile;
    if (!p) return null;
    if (!main) {
      toast({ kind: "warning", source: "KernTeX", title: "No main file", body: "Right-click your root .tex file in the file tree → “Set as main file”." });
      return null;
    }
    if (this.status.state === "running") {
      this.#pendingCompile = true;
      return null;
    }
    await this.saveAll();
    const firstRun = this.engine === "tectonic" && this.engines != null && !this.engines.tectonicCacheReady;
    this.status = { state: "running", stage: "Starting", detail: null, startedAt: performance.now(), firstRun };
    try {
      const result = await api.compile({ root: p.root, mainFile: main, engine: this.engine, flavor: this.flavor });
      if (this.project !== p) return null;
      this.status = { state: "done", result };
      this.log = result.log;
      if (!result.canceled) {
        this.#lastBuildMs = result.durationMs;
        if (result.ok) this.#lastOkBuildMs = result.durationMs;
        this.diagnostics = result.diagnostics;
        if (result.pdfPath) this.pdfVersion++;
        const errors = result.diagnostics.filter((d) => d.severity === "error").length;
        if (errors > 0) {
          settings.layout.problemsOpen = true;
        }
        if (firstRun && this.engines) this.engines.tectonicCacheReady = true;
      }
      this.editor?.refreshLint();
      this.refreshTree();
      return result;
    } catch (e) {
      const err = toAppError(e);
      this.status = { state: "idle" };
      this.diagnostics = [{ severity: "error", source: err.source === "engine" ? "TeX engine" : "KernTeX", file: null, line: null, message: err.title + (err.detail ? ` (${err.detail})` : ""), hint: err.hint }];
      settings.layout.problemsOpen = true;
      toastError(err);
      return null;
    } finally {
      if (this.#pendingCompile) {
        this.#pendingCompile = false;
        this.compile();
      }
    }
  }

  async cancelCompile() {
    this.#pendingCompile = false;
    await api.cancelCompile();
  }

  async gotoDiagnostic(d: Diagnostic) {
    if (!d.file) return;
    // Select the exact range when it's on one line (e.g. a missing package's name).
    const sameLine = !d.endLine || d.endLine === d.line;
    await this.openFile(d.file, d.line ?? undefined, d.column, sameLine ? d.endColumn : undefined);
  }
}

export const ws = new Workspace();
