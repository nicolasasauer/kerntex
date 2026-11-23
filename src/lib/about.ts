// Version, project links and the "Report a problem" issue template.
import { ws } from "./workspace.svelte";

export const APP_VERSION = __APP_VERSION__;
export const REPO_URL = "https://github.com/nicolasasauer/kerntex";

export const LINKS = {
  repo: REPO_URL,
  issues: `${REPO_URL}/issues`,
  releases: `${REPO_URL}/releases`,
  privacy: `${REPO_URL}/blob/main/PRIVACY.md`,
  license: `${REPO_URL}/blob/main/LICENSE`,
};

/** Third-party building blocks shown in the About dialog. */
export const CREDITS: { name: string; what: string; license: string; url: string }[] = [
  { name: "Tectonic", what: "TeX engine", license: "MIT", url: "https://tectonic-typesetting.github.io" },
  { name: "Tauri", what: "App shell", license: "MIT / Apache-2.0", url: "https://tauri.app" },
  { name: "Svelte", what: "Interface", license: "MIT", url: "https://svelte.dev" },
  { name: "CodeMirror", what: "Editor", license: "MIT", url: "https://codemirror.net" },
  { name: "pdf.js", what: "PDF preview", license: "Apache-2.0", url: "https://mozilla.github.io/pdf.js/" },
  { name: "Lucide", what: "Icons", license: "ISC", url: "https://lucide.dev" },
  { name: "Latin Modern", what: "Logo lettering", license: "GUST Font License", url: "https://www.gust.org.pl/projects/e-foundry/latin-modern" },
];

/** Best-effort OS name for bug reports, from the WebView's user agent. */
export function osName(ua = navigator.userAgent): string {
  const win = ua.match(/Windows NT ([\d.]+)/);
  if (win) return `Windows (NT ${win[1]})`;
  if (/Mac OS X/.test(ua)) return "macOS";
  if (/Linux/.test(ua)) return "Linux";
  return "unknown";
}

/** Pre-filled GitHub issue with the facts we'd otherwise have to ask for. */
export function reportProblemUrl(): string {
  const e = ws.engines;
  const latexmk = e?.latexmkVersion?.match(/Version ([\d.]+)/)?.[1];
  const engine = ws.project
    ? ws.engine === "tectonic"
      ? (e?.tectonicVersion ?? "Tectonic (version unknown)")
      : `latexmk${latexmk ? ` ${latexmk}` : ""} (${ws.flavor})`
    : "(no project open)";
  const body = [
    "**What happened?**",
    "",
    "",
    "**What did you expect instead?**",
    "",
    "",
    "**Steps to reproduce** (a small .tex file helps a lot)",
    "1. ",
    "",
    "---",
    `- KernTeX: ${APP_VERSION}`,
    `- OS: ${osName()}`,
    `- Engine: ${engine}`,
  ].join("\n");
  return `${LINKS.issues}/new?body=${encodeURIComponent(body)}`;
}
