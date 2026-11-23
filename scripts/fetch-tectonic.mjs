#!/usr/bin/env node
// Downloads the Tectonic binary for the current (or given) Rust target into
// src-tauri/binaries/tectonic-<target-triple>[.exe], as Tauri's `externalBin`
// expects. Picks the newest `tectonic@x.y.z` GitHub release and finds the
// asset by pattern, so nothing is hard-coded to a version.
//
// Usage: node scripts/fetch-tectonic.mjs [--target <triple>] [--force]
// Env:   GITHUB_TOKEN (optional, avoids API rate limits in CI)

import { execFileSync } from "node:child_process";
import { copyFileSync, createWriteStream, existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, statSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const REPO = "tectonic-typesetting/tectonic";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "src-tauri", "binaries");

/** Rust host triple, e.g. x86_64-pc-windows-msvc. */
export function hostTriple() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const m = out.match(/^host: (.+)$/m);
  if (!m) throw new Error("Couldn't determine the Rust target triple from `rustc -vV`.");
  return m[1].trim();
}

/** Candidate Tectonic target names for a Rust triple, best first. */
export function tectonicTargets(triple) {
  // Prefer the statically linked musl build on Linux: it runs on any distro.
  if (triple.endsWith("-unknown-linux-gnu")) return [triple.replace("-gnu", "-musl"), triple];
  return [triple];
}

/** Newest stable `tectonic@x.y.z` release that has an asset for the triple. */
export function pickAsset(releases, triple) {
  const stable = releases
    .filter((r) => /^tectonic@\d+\.\d+\.\d+$/.test(r.tag_name) && !r.draft && !r.prerelease)
    .sort((a, b) => cmpVersion(b.tag_name.split("@")[1], a.tag_name.split("@")[1]));
  for (const rel of stable) {
    for (const t of tectonicTargets(triple)) {
      const asset = rel.assets.find((a) => a.name.includes(`-${t}.`) && /\.(tar\.gz|zip)$/.test(a.name));
      if (asset) return { version: rel.tag_name.split("@")[1], name: asset.name, url: asset.browser_download_url };
    }
  }
  return null;
}

function cmpVersion(a, b) {
  const pa = a.split(".").map(Number), pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (pa[i] !== pb[i]) return pa[i] - pb[i];
  return 0;
}

async function listReleases() {
  const headers = { Accept: "application/vnd.github+json", "User-Agent": "kerntex-build" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  const all = [];
  for (let page = 1; page <= 5; page++) {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=50&page=${page}`, { headers });
    if (!res.ok) throw new Error(`GitHub API returned ${res.status} ${res.statusText}. Set GITHUB_TOKEN if you hit the rate limit.`);
    const batch = await res.json();
    all.push(...batch);
    if (batch.length < 50 || all.some((r) => /^tectonic@/.test(r.tag_name))) break;
  }
  return all;
}

/**
 * On Windows use the system bsdtar explicitly: in Git Bash, `tar` is GNU tar,
 * which reads `C:\…` as a remote host and can't open .zip files.
 */
function tarCommand() {
  if (process.platform === "win32") {
    const sys = join(process.env.SystemRoot || "C:\\Windows", "System32", "tar.exe");
    if (existsSync(sys)) return sys;
  }
  return "tar";
}

function findFile(dir, name) {
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) {
      const f = findFile(p, name);
      if (f) return f;
    } else if (entry === name) return p;
  }
  return null;
}

async function main() {
  const args = process.argv.slice(2);
  const ti = args.indexOf("--target");
  const triple = ti >= 0 ? args[ti + 1] : process.env.TAURI_TARGET_TRIPLE || hostTriple();
  const windows = triple.includes("windows");
  const dest = join(outDir, `tectonic-${triple}${windows ? ".exe" : ""}`);
  if (existsSync(dest) && !args.includes("--force")) {
    console.log(`✔ Tectonic already present: ${dest} (use --force to re-download)`);
    return;
  }

  console.log(`Looking up the latest Tectonic release for ${triple}…`);
  const asset = pickAsset(await listReleases(), triple);
  if (!asset) throw new Error(`No Tectonic release asset found for ${triple}. Install Tectonic manually and put it at ${dest}.`);

  console.log(`Downloading ${asset.name} (Tectonic ${asset.version})…`);
  const tmp = mkdtempSync(join(tmpdir(), "kerntex-tectonic-"));
  try {
    const archive = join(tmp, asset.name);
    const res = await fetch(asset.url, { headers: { "User-Agent": "kerntex-build" } });
    if (!res.ok || !res.body) throw new Error(`Download failed: ${res.status} ${res.statusText}`);
    await pipeline(Readable.fromWeb(res.body), createWriteStream(archive));

    // bsdtar (Windows 10+) and GNU tar both handle .tar.gz; bsdtar also handles .zip.
    const extract = join(tmp, "x");
    mkdirSync(extract);
    execFileSync(tarCommand(), ["-xf", archive, "-C", extract], { stdio: "inherit" });
    const bin = findFile(extract, windows ? "tectonic.exe" : "tectonic");
    if (!bin) throw new Error(`The archive ${asset.name} doesn't contain a tectonic binary.`);

    mkdirSync(outDir, { recursive: true });
    // Copy, not rename: the temp folder may be on another drive (C: vs D: on Windows CI).
    copyFileSync(bin, dest);
    if (!windows) chmodSync(dest, 0o755);
    console.log(`✔ Tectonic ${asset.version} → ${dest}`);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main().catch((e) => {
    console.error(`✖ ${e.message}`);
    process.exit(1);
  });
}
