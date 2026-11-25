#!/usr/bin/env node
// Builds `kerntex-mcp` (src-tauri/mcp-proxy) in release mode and puts it at
// src-tauri/binaries/kerntex-mcp-<target-triple>[.exe], where Tauri's
// `externalBin` (src-tauri/tauri.tectonic.conf.json) picks it up and installs
// it next to the app. AI assistants start this command to reach KernTeX.
//
// Usage: node scripts/build-mcp-proxy.mjs

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Rust host triple, e.g. x86_64-pc-windows-msvc. */
export function hostTriple(rustcVersion) {
  const m = /^host: (\S+)$/m.exec(rustcVersion);
  if (!m) throw new Error("Couldn't determine the Rust target triple from `rustc -vV`.");
  return m[1];
}

/** File name Tauri expects for an `externalBin` sidecar. */
export function sidecarName(triple) {
  return `kerntex-mcp-${triple}${triple.includes("windows") ? ".exe" : ""}`;
}

function main() {
  const triple = hostTriple(execFileSync("rustc", ["-vV"], { encoding: "utf8" }));
  const manifest = join(root, "src-tauri", "Cargo.toml");
  execFileSync("cargo", ["build", "--release", "--manifest-path", manifest, "-p", "kerntex-mcp"], { stdio: "inherit" });
  const exe = triple.includes("windows") ? "kerntex-mcp.exe" : "kerntex-mcp";
  const out = join(root, "src-tauri", "binaries");
  mkdirSync(out, { recursive: true });
  copyFileSync(join(root, "src-tauri", "target", "release", exe), join(out, sidecarName(triple)));
  console.log(`✔ src-tauri/binaries/${sidecarName(triple)}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    main();
  } catch (e) {
    console.error(`✖ ${e.message}`);
    process.exit(1);
  }
}
