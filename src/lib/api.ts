// Typed access to the Rust backend. In a plain browser (no Tauri), a mock
// backend is loaded in dev builds so the UI can be developed and screenshot.

import type {
  AppError, CompileRequest, CompileResult, EngineStatus, FileNode, ProjectInfo,
  ProjectRefs, ProjectSettings, Progress, Template,
} from "./types";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = (event: string, cb: (payload: unknown) => void) => Promise<() => void>;

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let backend: Promise<{ invoke: Invoke; listen: Listen }> | null = null;

function load() {
  backend ??= (async () => {
    if (isTauri) {
      const [{ invoke }, { listen }] = await Promise.all([
        import("@tauri-apps/api/core"),
        import("@tauri-apps/api/event"),
      ]);
      return {
        invoke: invoke as Invoke,
        listen: (async (event, cb) => listen(event, (e) => cb(e.payload))) as Listen,
      };
    }
    if (import.meta.env.DEV) {
      const mock = await import("./mock/backend");
      return { invoke: mock.invoke as Invoke, listen: mock.listen as Listen };
    }
    throw new Error("KernTeX must run inside the desktop app.");
  })();
  return backend;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const b = await load();
  try {
    return await b.invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "title" in e && "source" in e) return e as AppError;
  return { source: "app", title: "Something went wrong inside KernTeX", detail: String(e), hint: "Please try again. If it keeps happening, report it on GitHub with the details above." };
}

export const api = {
  detectEngines: () => call<EngineStatus>("detect_engines"),
  openProject: (root: string) => call<ProjectInfo>("open_project", { root }),
  readTree: (root: string) => call<FileNode[]>("read_tree", { root }),
  projectExists: (root: string) => call<boolean>("project_exists", { root }),
  readFile: (root: string, path: string) => call<string>("read_file", { root, path }),
  writeFile: (root: string, path: string, content: string) => call<void>("write_file", { root, path, content }),
  createFile: (root: string, path: string) => call<void>("create_file", { root, path }),
  createDir: (root: string, path: string) => call<void>("create_dir", { root, path }),
  renamePath: (root: string, from: string, to: string) => call<void>("rename_path", { root, from, to }),
  trashPath: (root: string, path: string) => call<void>("trash_path", { root, path }),
  importFiles: (root: string, dir: string, sources: string[]) => call<string[]>("import_files", { root, dir, sources }),
  readBinary: (root: string, path: string) => call<ArrayBuffer>("read_binary", { root, path }),
  scanRefs: (root: string) => call<ProjectRefs>("scan_refs", { root }),
  saveProjectSettings: (root: string, settings: ProjectSettings) => call<void>("save_project_settings", { root, settings }),
  texRoot: (root: string, path: string) => call<string | null>("tex_root", { root, path }),
  newProject: (parent: string, name: string, template: Template) => call<string>("new_project", { parent, name, template }),
  welcomeProject: (fresh: boolean) => call<string>("welcome_project", { fresh }),
  importZip: (zip: string, destParent: string) => call<string>("import_zip", { zip, destParent }),
  exportZip: (root: string, dest: string, clean: boolean) => call<number>("export_zip", { root, dest, clean }),
  readPdf: (root: string, mainFile: string) => call<ArrayBuffer>("read_pdf", { root, mainFile }),
  exportPdf: (root: string, mainFile: string, dest: string) => call<void>("export_pdf", { root, mainFile, dest }),
  compile: (request: CompileRequest) => call<CompileResult>("compile", { request }),
  cancelCompile: () => call<boolean>("cancel_compile"),
  onProgress: async (cb: (p: Progress) => void) => (await load()).listen("compile://progress", (p) => cb(p as Progress)),
};

// --- Native dialogs & shell (no-ops / prompts in the browser mock) ---

export async function pickFolder(title: string): Promise<string | null> {
  if (!isTauri) return window.prompt(title, "/home/demo/Documents") || null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, title });
  return typeof r === "string" ? r : null;
}

export async function pickZip(): Promise<string | null> {
  if (!isTauri) return window.prompt("ZIP file", "/home/demo/Downloads/overleaf.zip") || null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ title: "Import ZIP", filters: [{ name: "ZIP archive", extensions: ["zip"] }] });
  return typeof r === "string" ? r : null;
}

export async function pickFiles(title: string): Promise<string[]> {
  if (!isTauri) {
    const r = window.prompt(title, "/home/demo/Pictures/photo.png");
    return r ? [r] : [];
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ title, multiple: true });
  return Array.isArray(r) ? r : r ? [r] : [];
}

export async function pickSavePath(title: string, defaultPath: string, ext: string, label: string): Promise<string | null> {
  if (!isTauri) return window.prompt(title, defaultPath) || null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return (await save({ title, defaultPath, filters: [{ name: label, extensions: [ext] }] })) ?? null;
}

export async function confirmDialog(message: string, title = "KernTeX"): Promise<boolean> {
  if (!isTauri) return window.confirm(message);
  const { ask } = await import("@tauri-apps/plugin-dialog");
  return ask(message, { title, kind: "warning" });
}

export async function revealInFolder(path: string) {
  if (!isTauri) return;
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}

export async function openUrl(url: string) {
  if (!isTauri) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

export function joinPath(root: string, rel: string) {
  const sep = root.includes("\\") && !root.includes("/") ? "\\" : "/";
  return root.replace(/[\\/]+$/, "") + sep + rel.split("/").join(sep);
}

export function parentDir(path: string) {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}
