// Takes the README and Microsoft Store screenshots from the UI running in a
// browser with the mock backend (`src/lib/mock/`), so they can be refreshed
// with one command whenever the UI changes:
//
//   npm run screenshots              # all scenes, README + Store sizes
//   npm run screenshots -- git dark  # only these scenes
//
// Needs Playwright with Chromium (`npm i --no-save playwright && npx playwright install chromium`).
// Output: docs/screenshots/<scene>.png (1440×860) and docs/store/<n>-<scene>.png (1920×1080).

import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const PORT = 1427;
const BASE = `http://localhost:${PORT}`;

/** Sizes: README images and Microsoft Store screenshots (which must be exactly 1920×1080). */
const TARGETS = [
  { dir: "docs/screenshots", width: 1440, height: 860, name: (s) => `${s.id}.png` },
  { dir: "docs/store", width: 1920, height: 1080, name: (s) => `${s.store}-${s.storeName}.png`, only: (s) => s.store },
];

/**
 * Each scene opens the example project and gets the app into one state.
 * `run` gets the page; `window.__kerntex` is the dev-only workspace handle.
 * `settings` are stored before the app loads.
 */
const SCENES = [
  { id: "light", store: 1, storeName: "editor-light", theme: "light" },
  { id: "dark", store: 2, storeName: "editor-dark", theme: "dark" },
  {
    id: "errors",
    store: 3,
    storeName: "errors",
    theme: "light",
    query: "?demo=problems",
    async run(page) {
      // A typo and a command whose package isn't loaded, after a paragraph.
      await page.evaluate(() => {
        const ed = window.__kerntex.editor;
        const text = ed.text("main.tex");
        const line = "Paragraphs are separated by an empty line in the source.";
        const at = text.indexOf(line) + line.length;
        ed.insertAt("main.tex", at, " \\foo{} and \\SI{3}{\\metre}.");
        ed.gotoLine(text.slice(0, at).split("\n").length);
      });
      await page.keyboard.press("Control+s");
      await waitForBuild(page);
      await page.locator(".cm-lintRange-warning").first().hover();
      await page.locator(".cm-tooltip-lint").waitFor();
    },
  },
  {
    id: "figures",
    store: 4,
    storeName: "figures",
    theme: "light",
    async run(page) {
      await page.evaluate(() => window.__kerntex.openFile("images/example.png"));
      await page.getByRole("button", { name: /Insert into/ }).waitFor();
    },
  },
  {
    id: "build-details",
    store: 5,
    storeName: "build-details",
    theme: "light",
    async run(page) {
      await page.getByRole("button", { name: /^Problems/ }).first().click();
      await page.getByRole("button", { name: "Build details", exact: true }).click();
      await page.getByText(/loaded by your document/).waitFor();
    },
  },
  {
    id: "palette",
    store: 6,
    storeName: "command-palette",
    theme: "dark",
    async run(page) {
      await page.keyboard.press("Control+k");
      await page.keyboard.type("ex");
    },
  },
  {
    id: "git",
    store: 7,
    storeName: "git",
    theme: "light",
    async run(page) {
      await page.getByRole("tab", { name: /Git/ }).click();
      // The diff of the main file, which the welcome demo has modified.
      await page.locator(".git button.change", { hasText: "main.tex" }).click();
      await page.getByText("Changes since the last commit").waitFor();
    },
  },
  {
    id: "ai",
    store: 8,
    storeName: "ai-assistants",
    theme: "dark",
    query: "?demo=agent",
    settings: { mcpEnabled: true },
    fixedClock: true, // the activity list shows times
    async run(page) {
      await page.locator("button.agent").click();
      await page.getByRole("dialog").waitFor();
    },
  },
];

/** Waits until a build has finished and no other one starts right after it. */
async function waitForBuild(page) {
  const done = () => page.waitForFunction(() => window.__kerntex?.status?.state === "done", null, { timeout: 15000 });
  await done();
  await page.waitForTimeout(800);
  await done();
}

async function startVite() {
  const vite = spawn(process.execPath, [path.join(root, "node_modules/vite/bin/vite.js"), "--port", String(PORT), "--strictPort"], {
    cwd: root,
    stdio: ["ignore", "pipe", "pipe"],
  });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Vite didn't start within 30 s")), 30000);
    vite.stdout.on("data", (d) => {
      if (String(d).includes("Local:")) {
        clearTimeout(timer);
        resolve();
      }
    });
    vite.on("exit", (code) => reject(new Error(`Vite exited with code ${code}`)));
  });
  return vite;
}

async function shoot(browser, scene, target) {
  const context = await browser.newContext({
    viewport: { width: target.width, height: target.height },
    deviceScaleFactor: 1,
    colorScheme: scene.theme,
    reducedMotion: "reduce",
    // Same clock, language and time zone on every run, so unchanged UI gives identical pictures.
    locale: "en-US",
    timezoneId: "Europe/Berlin",
  });
  // A frozen clock also stops editor hover tooltips, so only where times are shown.
  if (scene.fixedClock) await context.clock.setFixedTime(new Date("2026-11-23T10:30:00+01:00"));
  await context.addInitScript((stored) => {
    localStorage.setItem("kerntex.settings.v1", JSON.stringify(stored));
  }, { theme: scene.theme, welcomed: true, ...scene.settings });
  const page = await context.newPage();
  await page.goto(`${BASE}/${scene.query ?? "?demo"}`);
  await waitForBuild(page);
  // The PDF preview renders its first page.
  await page.locator(".pdf canvas, canvas").first().waitFor();
  // No "Example project saved in …" notices on the pictures.
  await page.addStyleTag({ content: ".toasts { display: none !important; }" });
  if (scene.run) await scene.run(page);
  await page.waitForTimeout(400);
  const file = path.join(root, target.dir, target.name(scene));
  await page.screenshot({ path: file });
  await context.close();
  return path.relative(root, file);
}

async function main() {
  const wanted = process.argv.slice(2);
  const scenes = wanted.length ? SCENES.filter((s) => wanted.includes(s.id)) : SCENES;
  if (!scenes.length) throw new Error(`Unknown scene. Available: ${SCENES.map((s) => s.id).join(", ")}`);

  let chromium;
  try {
    ({ chromium } = require("playwright"));
  } catch {
    throw new Error("Playwright isn't installed. Run: npm i --no-save playwright && npx playwright install chromium");
  }

  const vite = await startVite();
  const browser = await chromium.launch();
  try {
    for (const target of TARGETS) await mkdir(path.join(root, target.dir), { recursive: true });
    for (const scene of scenes) {
      for (const target of TARGETS) {
        if (target.only && !target.only(scene)) continue;
        console.log(`✓ ${await shoot(browser, scene, target)}`);
      }
    }
  } finally {
    await browser.close();
    vite.kill();
  }
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
