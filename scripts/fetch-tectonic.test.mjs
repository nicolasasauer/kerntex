// node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import { pickAsset, tectonicTargets } from "./fetch-tectonic.mjs";

const rel = (tag, names, extra = {}) => ({ tag_name: tag, draft: false, prerelease: false, assets: names.map((n) => ({ name: n, browser_download_url: `https://x/${n}` })), ...extra });

const releases = [
  rel("continuous", ["tectonic-0.99.0-x86_64-pc-windows-msvc.zip"]),
  rel("tectonic@0.14.1", ["tectonic-0.14.1-x86_64-pc-windows-msvc.zip", "tectonic-0.14.1-x86_64-unknown-linux-musl.tar.gz"]),
  rel("tectonic@0.15.0", ["tectonic-0.15.0-x86_64-pc-windows-msvc.zip", "tectonic-0.15.0-x86_64-unknown-linux-gnu.tar.gz", "tectonic-0.15.0-x86_64-unknown-linux-musl.tar.gz", "tectonic-0.15.0-x86_64-pc-windows-msvc.zip.sha256"]),
  rel("cargo-cache@1.0.0", []),
  rel("tectonic@0.16.0", ["tectonic-0.16.0-x86_64-pc-windows-msvc.zip"], { prerelease: true }),
];

test("picks newest stable release for Windows", () => {
  assert.deepEqual(pickAsset(releases, "x86_64-pc-windows-msvc"), { version: "0.15.0", name: "tectonic-0.15.0-x86_64-pc-windows-msvc.zip", url: "https://x/tectonic-0.15.0-x86_64-pc-windows-msvc.zip" });
});

test("prefers static musl build on Linux", () => {
  assert.equal(pickAsset(releases, "x86_64-unknown-linux-gnu").name, "tectonic-0.15.0-x86_64-unknown-linux-musl.tar.gz");
  assert.deepEqual(tectonicTargets("aarch64-apple-darwin"), ["aarch64-apple-darwin"]);
});

test("returns null when no asset matches", () => {
  assert.equal(pickAsset(releases, "riscv64gc-unknown-linux-gnu"), null);
});
