#!/usr/bin/env node
// Packs the built Windows app into an MSIX for the Microsoft Store.
//
// Run after `tauri build` (or `cargo build` for a quick check) on Windows:
//   node scripts/build-msix.mjs [--profile release|debug] [--out <dir>]
//
// The package identity must match the app reserved in Partner Center
// (Product management → Product identity). Set these environment variables
// (in CI: repository variables of the same name):
//   MSIX_IDENTITY_NAME            Package/Identity/Name,       e.g. 12345NicolasSauer.KernTeX
//   MSIX_PUBLISHER                Package/Identity/Publisher,  e.g. CN=1A2B3C4D-…
//   MSIX_PUBLISHER_DISPLAY_NAME   Package/Properties/PublisherDisplayName
// Without them, placeholder values are used: the package still builds (useful
// to check it in CI) but can't be submitted to the Store.
//
// The Store signs the package itself, so no certificate is needed for a
// submission. To install the .msix locally for testing, sign it yourself
// (signtool + a trusted test certificate).

import { execFileSync } from "node:child_process";
import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const PLACEHOLDER = {
  MSIX_IDENTITY_NAME: "KernTeX.Development",
  MSIX_PUBLISHER: "CN=KernTeX Development",
  MSIX_PUBLISHER_DISPLAY_NAME: "KernTeX (development build)",
};

/** "0.1.0" → "0.1.0.0" (MSIX needs four parts; the Store requires the last to be 0). */
export function msixVersion(semver) {
  const m = /^(\d+)\.(\d+)\.(\d+)/.exec(semver);
  if (!m) throw new Error(`Can't turn version "${semver}" into an MSIX version.`);
  return `${m[1]}.${m[2]}.${m[3]}.0`;
}

function xmlEscape(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** Fills the {{…}} placeholders of the manifest template. */
export function renderManifest(template, values) {
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    if (!(key in values)) throw new Error(`Manifest placeholder {{${key}}} has no value.`);
    return xmlEscape(values[key]);
  });
}

export function identityFromEnv(env) {
  const get = (k) => (env[k] && env[k].trim()) || null;
  const real = ["MSIX_IDENTITY_NAME", "MSIX_PUBLISHER", "MSIX_PUBLISHER_DISPLAY_NAME"].every(get);
  const pick = (k) => get(k) ?? PLACEHOLDER[k];
  return {
    storeReady: real,
    IDENTITY_NAME: pick("MSIX_IDENTITY_NAME"),
    PUBLISHER: pick("MSIX_PUBLISHER"),
    PUBLISHER_DISPLAY_NAME: pick("MSIX_PUBLISHER_DISPLAY_NAME"),
  };
}

/** Newest makeappx.exe from the Windows 10/11 SDK. */
function findMakeAppx() {
  const kits = join(process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "Windows Kits", "10", "bin");
  if (!existsSync(kits)) return null;
  const versions = readdirSync(kits).filter((d) => /^10\.\d+\.\d+\.\d+$/.test(d)).sort((a, b) => {
    const pa = a.split(".").map(Number), pb = b.split(".").map(Number);
    for (let i = 0; i < 4; i++) if (pa[i] !== pb[i]) return pb[i] - pa[i];
    return 0;
  });
  for (const v of versions) {
    const p = join(kits, v, "x64", "makeappx.exe");
    if (existsSync(p)) return p;
  }
  return null;
}

function main() {
  const args = process.argv.slice(2);
  const arg = (name, def) => {
    const i = args.indexOf(name);
    return i >= 0 ? args[i + 1] : def;
  };
  const profile = arg("--profile", "release");
  const outDir = arg("--out", join(root, "src-tauri", "target", profile, "bundle", "msix"));

  if (process.platform !== "win32") throw new Error("MSIX packages can only be built on Windows (makeappx.exe).");
  const makeappx = findMakeAppx();
  if (!makeappx) throw new Error("makeappx.exe not found. Install the Windows 10/11 SDK.");

  const target = join(root, "src-tauri", "target", profile);
  const exe = join(target, "kerntex.exe");
  if (!existsSync(exe)) throw new Error(`${exe} doesn't exist. Build the app first (npm run tauri build / cargo build).`);

  // Tectonic: copied next to the exe by the Tauri build, else take it from binaries/.
  let tectonic = join(target, "tectonic.exe");
  if (!existsSync(tectonic)) {
    const bins = join(root, "src-tauri", "binaries");
    const found = existsSync(bins) ? readdirSync(bins).find((f) => /^tectonic-.*windows.*\.exe$/.test(f)) : null;
    tectonic = found ? join(bins, found) : null;
  }

  // kerntex-mcp (AI assistants, MCP): next to the exe after a Tauri or workspace build, else from binaries/.
  let mcp = join(target, "kerntex-mcp.exe");
  if (!existsSync(mcp)) {
    const bins = join(root, "src-tauri", "binaries");
    const found = existsSync(bins) ? readdirSync(bins).find((f) => /^kerntex-mcp-.*windows.*\.exe$/.test(f)) : null;
    mcp = found ? join(bins, found) : null;
  }
  if (!mcp) throw new Error("kerntex-mcp.exe not found. Build it first: npm run build:mcp (or cargo build --workspace).");

  const version = msixVersion(JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version);
  const identity = identityFromEnv(process.env);

  const layout = join(target, "msix-layout");
  rmSync(layout, { recursive: true, force: true });
  mkdirSync(layout, { recursive: true });
  copyFileSync(exe, join(layout, "kerntex.exe"));
  copyFileSync(mcp, join(layout, "kerntex-mcp.exe"));
  if (tectonic) copyFileSync(tectonic, join(layout, "tectonic.exe"));
  else console.warn("⚠ Tectonic not found — the package will need a TeX distribution on the user's machine. Run `npm run setup:tectonic` first.");
  cpSync(join(root, "src-tauri", "msix", "Assets"), join(layout, "Assets"), { recursive: true });
  const template = readFileSync(join(root, "src-tauri", "msix", "AppxManifest.xml"), "utf8");
  writeFileSync(join(layout, "AppxManifest.xml"), renderManifest(template, { ...identity, VERSION: version }));

  mkdirSync(outDir, { recursive: true });
  const out = join(outDir, `KernTeX_${version}_x64.msix`);
  rmSync(out, { force: true });
  execFileSync(makeappx, ["pack", "/d", layout, "/p", out, "/o"], { stdio: "inherit" });

  console.log(`✔ ${out}`);
  if (!identity.storeReady) {
    console.log("ℹ Built with placeholder identity. Set MSIX_IDENTITY_NAME, MSIX_PUBLISHER and MSIX_PUBLISHER_DISPLAY_NAME (from Partner Center) for a Store-ready package.");
  }
  if (process.env.GITHUB_OUTPUT) writeFileSync(process.env.GITHUB_OUTPUT, `msix=${out}\nstore_ready=${identity.storeReady}\n`, { flag: "a" });
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    main();
  } catch (e) {
    console.error(`✖ ${e.message}`);
    process.exit(1);
  }
}
