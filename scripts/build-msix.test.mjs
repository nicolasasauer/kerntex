// node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { msixVersion, renderManifest, identityFromEnv } from "./build-msix.mjs";

test("MSIX version has four parts ending in 0", () => {
  assert.equal(msixVersion("0.1.0"), "0.1.0.0");
  assert.equal(msixVersion("1.12.3-beta.1"), "1.12.3.0");
  assert.throws(() => msixVersion("dev"));
});

test("identity comes from env, else placeholders", () => {
  const real = identityFromEnv({ MSIX_IDENTITY_NAME: "123Nico.KernTeX", MSIX_PUBLISHER: "CN=ABC", MSIX_PUBLISHER_DISPLAY_NAME: "Nicolas Sauer" });
  assert.equal(real.storeReady, true);
  assert.equal(real.IDENTITY_NAME, "123Nico.KernTeX");
  const partial = identityFromEnv({ MSIX_IDENTITY_NAME: "123Nico.KernTeX", MSIX_PUBLISHER: " " });
  assert.equal(partial.storeReady, false);
  assert.equal(partial.PUBLISHER, "CN=KernTeX Development");
});

test("manifest template renders every placeholder and escapes XML", () => {
  const tpl = readFileSync(new URL("../src-tauri/msix/AppxManifest.xml", import.meta.url), "utf8");
  const xml = renderManifest(tpl, { IDENTITY_NAME: "A.B", PUBLISHER: 'CN=Me & "Co"', PUBLISHER_DISPLAY_NAME: "Me", VERSION: "0.1.0.0" });
  assert.ok(!/\{\{\w+\}\}/.test(xml));
  assert.ok(xml.includes('Publisher="CN=Me &amp; &quot;Co&quot;"'));
  assert.ok(xml.includes('Version="0.1.0.0"'));
  assert.throws(() => renderManifest("{{MISSING}}", {}));
});
