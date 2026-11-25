// node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mismatches, places, readVersions, setVersion } from "./version.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));

test("all project files state the same version", () => {
  const [first] = readVersions();
  assert.deepEqual(mismatches(`v${first.version}`), []);
});

test("setting a version updates every place and nothing is left behind", () => {
  const dir = mkdtempSync(join(tmpdir(), "kerntex-version-"));
  try {
    for (const [file] of places) {
      mkdirSync(dirname(join(dir, file)), { recursive: true });
      cpSync(join(root, file), join(dir, file));
    }
    setVersion("9.8.7", dir);
    assert.deepEqual(mismatches("v9.8.7", dir), []);
    assert.equal(mismatches("v9.8.8", dir).length, places.length);
    assert.throws(() => setVersion("next", dir));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
