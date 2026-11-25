import { test } from "node:test";
import assert from "node:assert/strict";
import { hostTriple, sidecarName } from "./build-mcp-proxy.mjs";

test("reads the host triple from rustc -vV", () => {
  assert.equal(hostTriple("rustc 1.97.0\nbinary: rustc\nhost: x86_64-unknown-linux-gnu\nrelease: 1.97.0\n"), "x86_64-unknown-linux-gnu");
  assert.throws(() => hostTriple("nothing here"));
});

test("names the sidecar the way Tauri expects", () => {
  assert.equal(sidecarName("x86_64-unknown-linux-gnu"), "kerntex-mcp-x86_64-unknown-linux-gnu");
  assert.equal(sidecarName("x86_64-pc-windows-msvc"), "kerntex-mcp-x86_64-pc-windows-msvc.exe");
});
