// In-memory stand-in for the Rust backend, used only by `npm run dev` in a
// normal browser (never bundled into the app). Lets the UI be developed and
// screenshot without Tauri.

import welcomeTex from "../../../src-tauri/templates/welcome/main.tex?raw";
import welcomeBib from "../../../src-tauri/templates/welcome/references.bib?raw";
import articleTex from "../../../src-tauri/templates/article.tex?raw";
import samplePdfUrl from "./sample.pdf?url";
import exampleImageUrl from "../../../src-tauri/templates/welcome/images/example.png?url";
import sampleDetails from "./details.json";
import type { BuildDetails, CompileRequest, CompileResult, Diagnostic, FileNode, ProjectInfo } from "../types";

const emptyDetails: BuildDetails = { program: null, format: null, documentClass: null, packages: [], passes: 0, enginePath: null, outDir: null, logPath: null };

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
      return { ok: false, canceled: true, pdfPath: null, diagnostics: [], durationMs: Math.round(performance.now() - start), log: "", engineLabel: "Tectonic", details: emptyDetails };
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
  return { ok, canceled: false, pdfPath: `${ROOT}/.kerntex/out/main.pdf`, diagnostics, durationMs: Math.round(performance.now() - start), log: "This is a mock log.\n(./main.tex\nLaTeX Warning: ...\n)", engineLabel: req.engine === "tectonic" ? "Tectonic" : "latexmk (pdfLaTeX)", details: sampleDetails as BuildDetails };
}

export async function invoke<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  await delay(15);
  const r = await handle(cmd, args);
  return r as T;
}

async function handle(cmd: string, a: Record<string, any>): Promise<unknown> {
  switch (cmd) {
    case "detect_engines":
      return { tectonic: "/opt/kerntex/tectonic", latexmk: "/usr/bin/latexmk", tectonicCacheReady: true, tectonicVersion: "Tectonic 0.17.0", latexmkVersion: "Latexmk, John Collins, 31 Jan. 2024. Version 4.83", tectonicCacheDir: "/home/demo/.cache/Tectonic" };
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
    case "import_files": {
      const added: string[] = [];
      for (const src of a.sources as string[]) {
        const name = src.split(/[\\/]/).pop()!;
        let rel = a.dir ? `${a.dir}/${name}` : name;
        const dot = name.lastIndexOf(".");
        for (let n = 2; files.has(rel); n++) rel = (a.dir ? `${a.dir}/` : "") + (dot > 0 ? `${name.slice(0, dot)}-${n}${name.slice(dot)}` : `${name}-${n}`);
        files.set(rel, "<binary>");
        added.push(rel);
      }
      return added;
    }
    case "read_binary":
      if (!files.has(a.path)) throw err(`Couldn't open “${a.path}”`);
      return (await fetch(a.path.endsWith(".pdf") ? samplePdfUrl : exampleImageUrl)).arrayBuffer();
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
    default:
      if (cmd.startsWith("git_")) return gitMock(cmd, a);
  }
  throw { source: "app", title: `Unknown command ${cmd}`, detail: null, hint: null };
}

export async function listen(event: string, cb: (p: unknown) => void) {
  const list = listeners.get(event) ?? [];
  list.push(cb);
  listeners.set(event, list);
  return () => listeners.set(event, (listeners.get(event) ?? []).filter((x) => x !== cb));
}

// --- Git (pretend repository; ?nogit starts without one) ---

const gitRepo = { repo: !new URLSearchParams(location.search).has("nogit"), remoteUrl: "https://github.com/demo/welcome-to-kerntex.git" as string | null, ahead: 0, identity: true };
const committed = new Map(files);
committed.set("main.tex", welcomeTex.replace("\\section{", "\\section{Old ").replace(/\n[^\n]*\n/, "\n"));
committed.delete("chapters/appendix.tex");
let lastCommit = { hash: "3f2c1ab", subject: "Add bibliography", time: Math.floor(Date.now() / 1000) - 7200 };
let gitCancel = false;

function gitChanges() {
  const changes: { path: string; oldPath: string | null; kind: string }[] = [];
  const skip = (p: string) => /\.(aux|log)$/.test(p);
  for (const [p, t] of files) {
    if (skip(p)) continue;
    if (!committed.has(p)) changes.push({ path: p, oldPath: null, kind: "untracked" });
    else if (committed.get(p) !== t) changes.push({ path: p, oldPath: null, kind: "modified" });
  }
  for (const p of committed.keys()) if (!files.has(p) && !skip(p)) changes.push({ path: p, oldPath: null, kind: "deleted" });
  return changes.sort((x, y) => x.path.localeCompare(y.path));
}

function mockDiff(path: string) {
  const a = (committed.get(path) ?? "").split("\n");
  const b = (files.get(path) ?? "").split("\n");
  if (!committed.has(path)) return `@@ -0,0 +1,${b.length} @@\n` + b.map((l) => "+" + l).join("\n");
  let s = 0;
  while (s < a.length && s < b.length && a[s] === b[s]) s++;
  let e = 0;
  while (e < a.length - s && e < b.length - s && a[a.length - 1 - e] === b[b.length - 1 - e]) e++;
  const from = Math.max(0, s - 3);
  const ctx = (l: string) => " " + l;
  const body = [...a.slice(from, s).map(ctx), ...a.slice(s, a.length - e).map((l) => "-" + l), ...b.slice(s, b.length - e).map((l) => "+" + l), ...a.slice(a.length - e, a.length - e + 3).map(ctx)];
  return `@@ -${from + 1} +${from + 1} @@\n` + body.join("\n");
}

async function gitMock(cmd: string, a: Record<string, any>): Promise<unknown> {
  switch (cmd) {
    case "git_status":
      return {
        gitVersion: "git version 2.47.0", repo: gitRepo.repo, branch: gitRepo.repo ? "main" : null,
        upstream: gitRepo.repo && gitRepo.remoteUrl ? "origin/main" : null, ahead: gitRepo.ahead, behind: 0,
        remote: gitRepo.remoteUrl ? "origin" : null, remoteUrl: gitRepo.remoteUrl, hasCommits: true,
        lastCommit: gitRepo.repo ? lastCommit : null, merging: false,
        identity: gitRepo.identity ? { name: "Demo User", email: "demo@example.com" } : null,
        changes: gitRepo.repo ? gitChanges() : [],
      };
    case "git_diff":
      return mockDiff(a.path);
    case "git_suggest_message": {
      const c = gitChanges();
      const mod = c.filter((x) => x.kind === "modified").map((x) => x.path.split("/").pop());
      const add = c.filter((x) => x.kind === "untracked").map((x) => x.path.split("/").pop());
      const parts = [mod.length ? `Edit “Introduction”${mod.length > 1 ? `, update ${mod.slice(1).join(", ")}` : ""}` : "", add.length ? `add ${add.join(", ")}` : ""].filter(Boolean);
      return parts.join(", ");
    }
    case "git_init":
      gitRepo.repo = true;
      return null;
    case "git_commit":
      for (const [p, t] of files) committed.set(p, t);
      for (const p of [...committed.keys()]) if (!files.has(p)) committed.delete(p);
      lastCommit = { hash: Math.random().toString(16).slice(2, 9), subject: String(a.message).split("\n")[0], time: Math.floor(Date.now() / 1000) };
      gitRepo.ahead++;
      await delay(300);
      return lastCommit;
    case "git_pull":
    case "git_push": {
      gitCancel = false;
      const steps = cmd === "git_push" ? ["Enumerating objects: 7, done.", "Counting objects: 100% (7/7), done.", "Writing objects:  40% (2/5)", "Writing objects: 100% (5/5), 1.2 KiB, done."] : ["Fetching origin", "Receiving objects: 100% (3/3), done."];
      for (const line of steps) {
        if (gitCancel) throw { source: "git", title: `${cmd === "git_push" ? "Push" : "Pull"} canceled`, detail: null, hint: "Nothing was lost. Try again whenever you like." };
        for (const cb of listeners.get("git://progress") ?? []) cb({ op: cmd.slice(4), line });
        await delay(500);
      }
      if (cmd === "git_push") gitRepo.ahead = 0;
      return cmd === "git_pull" ? { updated: false, conflicts: [] } : null;
    }
    case "git_cancel":
      gitCancel = true;
      return true;
    case "git_set_remote":
      gitRepo.remoteUrl = a.url;
      return null;
    case "git_set_identity":
      gitRepo.identity = true;
      return null;
    case "git_abort_merge":
      return null;
  }
  throw { source: "app", title: `Unknown command ${cmd}`, detail: null, hint: null };
}
