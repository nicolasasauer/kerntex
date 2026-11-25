import type { AppError } from "./types";

export type ToastKind = "success" | "error" | "info" | "warning";

export interface Toast {
  id: number;
  kind: ToastKind;
  /** Where it came from: "LaTeX", "Tectonic", "File system", "KernTeX" … */
  source: string;
  title: string;
  body?: string | null;
  action?: { label: string; run: () => void };
}

let next = 1;
export const toasts: Toast[] = $state([]);

export function toast(t: Omit<Toast, "id">, ms = t.kind === "error" ? 9000 : 3500) {
  const id = next++;
  toasts.push({ ...t, id });
  if (toasts.length > 4) toasts.splice(0, toasts.length - 4);
  if (ms > 0) setTimeout(() => dismiss(id), ms);
  return id;
}

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

const SOURCE_LABEL: Record<AppError["source"], string> = {
  fileSystem: "File system",
  engine: "TeX engine",
  archive: "ZIP",
  git: "Git",
  app: "KernTeX",
};

export function toastError(e: AppError) {
  return toast({ kind: "error", source: SOURCE_LABEL[e.source] ?? "KernTeX", title: e.title, body: [e.hint, e.detail].filter(Boolean).join("\n\n") });
}
