// In-memory stand-in for the Rust backend, used only by `npm run dev` in a
// normal browser (never bundled into the app). Lets the UI be developed and
// screenshot without Tauri.

import welcomeTex from "../../../src-tauri/templates/welcome/main.tex?raw";
import welcomeBib from "../../../src-tauri/templates/welcome/references.bib?raw";
import articleTex from "../../../src-tauri/templates/article.tex?raw";
import samplePdfUrl from "./sample.pdf?url";
import type { CompileRequest, CompileResult, Diagnostic, FileNode, ProjectInfo } from "../types";

const ROOT = "/home/demo/Documents/Welcome to KernTeX";
const files = new Map<string, string>([
  ["main.tex", welcomeTex],
  ["references.bib", welcomeBib],
  ["images/example.png", "<binary>"],
  ["chapters/appendix.tex", "% !TEX root = ../main.tex\n\\section{Appendix}\nMore content.\n"],
  ["main.aux", ""],
  ["main.log", ""],
]);
const dirs = new Set(["images", "chapters"]);
let settings: Record<string, unknown> = { version: 1 };
const listeners = new Map<string, ((p: unknown) => void)[]>();
let cancelFlag = false;

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));
const err = (title: string, hint?: string) => ({ source: "fileSystem", title, detail: null, hint: hint ?? null });

function tree(): FileNode[] {
  const rootNodes: FileNode[] = [];
  const byPath = new Map<string, FileNode>();
  const ensureDir = (p: string): FileNode[] => {
    if (!p) return rootNodes;
    let n = byPath.get(p);
    if (!n) {
      const i = p.lastIndexOf("/");
      n = { name: p.slice(i + 1), path: p, isDir: true, generated: false, children: [] };
      byPath.set(p, n);
      ensureDir(i > 0 ? p.slice(0, i) : "").push(n);
    }
    return n.children;
  };
  for (const d of dirs) ensureDir(d);
  for (const f of files.keys()) {
    const i = f.lastIndexOf("/");
    const name = f.slice(i + 1);
    ensureDir(i > 0 ? f.slice(0, i) : "").push({
      name, path: f, isDir: false, generated: /\.(aux|log|out|toc|synctex\.gz|bbl|blg|bcf)$/.test(name) && !name.endsWith(".bbl"), children: [],
    });
  }
  const sort = (ns: FileNode[]) => {
    ns.sort((a, b) => Number(b.isDir) - Number(a.isDir) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    ns.forEach((n) => sort(n.children));
  };
  sort(rootNodes);
  return rootNodes;
}

function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((cb) => cb(payload));
}

async function compile(req: CompileRequest): Promise<CompileResult> {
  cancelFlag = false;
  const start = performance.now();
  const stages: [string, string | null, number][] = req.engine === "tectonic"
    ? [["Starting engine", "Tectonic", 80], ["Typesetting (pass 1)", null, 500], ["Processing bibliography", null, 250], ["Typesetting (pass 2)", "because of changed references", 400], ["Writing PDF", null, 200]]
    : [["Starting engine", "latexmk (pdfLaTeX)", 80], ["Typesetting (pass 1)", "pdflatex", 500], ["Processing bibliography", "bibtex", 250], ["Typesetting (pass 2)", "pdflatex", 400]];
  for (const [stage, detail, ms] of stages) {
    emit("compile://progress", { stage, detail, elapsedMs: Math.round(performance.now() - start) });
    await delay(ms);
    if (cancelFlag) {
      return { ok: false, canceled: true, pdfPath: null, diagnostics: [], durationMs: Math.round(performance.now() - start), log: "", engineLabel: "Tectonic" };
    }
  }
  const diagnostics: Diagnostic[] = [];
  for (const [path, text] of files) {
    if (!path.endsWith(".tex")) continue;
    text.split("\n").forEach((line, i) => {
      const m = line.match(/\\(foo|sectoin|textbff)\b/);
      if (m) diagnostics.push({ severity: "error", source: "LaTeX", file: path, line: i + 1, message: `Undefined control sequence — near “\\${m[1]}”`, hint: "This command isn't defined. Check its spelling, or load the package that provides it with \\usepackage{…} in the preamble." });
      const pkg = line.match(/\\usepackage\{(nonexistentpkg)\}/);
      if (pkg) diagnostics.push({ severity: "error", source: "LaTeX", file: path, line: i + 1, message: `LaTeX Error: File \`${pkg[1]}.sty' not found`, hint: `LaTeX can't find “${pkg[1]}.sty”.` });
    });
  }
  diagnostics.push({ severity: "warning", source: "LaTeX", file: "main.tex", line: 137, message: "Overfull \\hbox (27.6pt too wide) in paragraph at lines 135--139", hint: "A line sticks out into the margin. Rephrase it, allow hyphenation (e.g. long URLs with \\url{…}), or load \\usepackage{microtype}." });
  diagnostics.push({ severity: "info", source: "hyperref", file: "main.tex", line: 31, message: "Token not allowed in a PDF string (Unicode): removing `\\textsf'", hint: null });
  const ok = !diagnostics.some((d) => d.severity === "error");
  return { ok, canceled: false, pdfPath: `${ROOT}/.kerntex/out/main.pdf`, diagnostics, durationMs: Math.round(performance.now() - start), log: "This is a mock log.\n(./main.tex\nLaTeX Warning: ...\n)", engineLabel: req.engine === "tectonic" ? "Tectonic" : "latexmk (pdfLaTeX)" };
}

export async function invoke<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  await delay(15);
  const r = await handle(cmd, args);
  return r as T;
}

async function handle(cmd: string, a: Record<string, any>): Promise<unknown> {
  switch (cmd) {
    case "detect_engines":
      return { tectonic: "/opt/kerntex/tectonic", latexmk: "/usr/bin/latexmk", tectonicCacheReady: true };
    case "project_exists":
      return true;
    case "welcome_project":
    case "new_project":
    case "import_zip":
      if (cmd === "new_project") files.set("main.tex", articleTex);
      return ROOT;
    case "open_project": {
      const info: ProjectInfo = { root: ROOT, name: "Welcome to KernTeX", tree: tree(), mainFile: (settings.mainFile as string) ?? "main.tex", settings: settings as any, hints: { flavor: null, hasLatexmkrc: false } };
      return info;
    }
    case "read_tree":
      return tree();
    case "read_file":
      if (!files.has(a.path)) throw err(`Couldn't open “${a.path}”`);
      if (a.path.endsWith(".png")) throw err(`“${a.path}” isn't a text file`, "KernTeX can only edit UTF-8 text.");
      return files.get(a.path);
    case "write_file":
      files.set(a.path, a.content);
      return null;
    case "create_file":
      if (files.has(a.path)) throw err(`Couldn't create “${a.path}”`, "Something with this name already exists. Choose a different name.");
      files.set(a.path, "");
      return null;
    case "create_dir":
      dirs.add(a.path);
      return null;
    case "rename_path": {
      for (const [k, v] of [...files]) {
        if (k === a.from || k.startsWith(a.from + "/")) {
          files.delete(k);
          files.set(a.to + k.slice(a.from.length), v);
        }
      }
      if (dirs.delete(a.from)) dirs.add(a.to);
      return null;
    }
    case "trash_path":
      for (const k of [...files.keys()]) if (k === a.path || k.startsWith(a.path + "/")) files.delete(k);
      dirs.delete(a.path);
      return null;
    case "scan_refs": {
      const labels: { name: string; file: string; line: number }[] = [];
      const citations: { key: string; kind: string; title: string | null; file: string }[] = [];
      for (const [f, t] of files) {
        if (f.endsWith(".tex")) t.split("\n").forEach((l, i) => { for (const m of l.matchAll(/\\label\{([^}]+)\}/g)) labels.push({ name: m[1], file: f, line: i + 1 }); });
        if (f.endsWith(".bib")) for (const m of t.matchAll(/@(\w+)\{([^,]+),[\s\S]*?title\s*=\s*\{(.+?)\},/g)) citations.push({ kind: m[1].toLowerCase(), key: m[2].trim(), title: m[3].replace(/[{}]/g, ""), file: f });
      }
      return { labels, citations };
    }
    case "save_project_settings":
      settings = a.settings;
      return null;
    case "tex_root":
      return a.path.startsWith("chapters/") ? "main.tex" : null;
    case "export_zip":
      return files.size;
    case "export_pdf":
      return null;
    case "read_pdf":
      return (await fetch(samplePdfUrl)).arrayBuffer();
    case "compile":
      return compile(a.request);
    case "cancel_compile":
      cancelFlag = true;
      return true;
  }
  throw { source: "app", title: `Unknown command ${cmd}`, detail: null, hint: null };
}

export async function listen(event: string, cb: (p: unknown) => void) {
  const list = listeners.get(event) ?? [];
  list.push(cb);
  listeners.set(event, list);
  return () => listeners.set(event, (listeners.get(event) ?? []).filter((x) => x !== cb));
}
