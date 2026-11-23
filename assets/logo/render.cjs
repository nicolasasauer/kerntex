// Renders the logo SVGs to every PNG KernTeX needs (needs Playwright's Chromium).
//   node assets/logo/render.cjs && npx tauri icon assets/logo.png -o src-tauri/icons && python3 assets/logo/make_ico.py
// Small sizes (≤ 71 px) use logo-small.svg (no "eX"), which stays legible.
const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");
const root = path.resolve(__dirname, "../..");
const full = fs.readFileSync(path.join(__dirname, "logo.svg"), "utf8");
const small = fs.readFileSync(path.join(__dirname, "logo-small.svg"), "utf8");

const jobs = [
  // [svg, size, output, {wide}]
  [full, 1024, "assets/logo.png"],
  [small, 16, "assets/logo/png/small-16.png"],
  [small, 24, "assets/logo/png/small-24.png"],
  [small, 32, "assets/logo/png/small-32.png"],
  [full, 48, "assets/logo/png/full-48.png"],
  [full, 256, "assets/logo/png/full-256.png"],
  // MSIX tiles
  [small, 44, "src-tauri/msix/Assets/Square44x44Logo.png"],
  [small, 50, "src-tauri/msix/Assets/StoreLogo.png"],
  [full, 150, "src-tauri/msix/Assets/Square150x150Logo.png"],
  [full, 310, "src-tauri/msix/Assets/Square310x310Logo.png"],
  [full, 120, "src-tauri/msix/Assets/Wide310x150Logo.png", { w: 310, h: 150 }],
  // Partner Center "Bilder für die Store-Anzeige"
  [full, 300, "docs/store/logos/logo-300x300.png"],
  [full, 150, "docs/store/logos/logo-150x150.png"],
  [small, 71, "docs/store/logos/logo-71x71.png"],
];

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  for (const [svg, size, out, box] of jobs) {
    const w = box?.w ?? size, h = box?.h ?? size;
    await page.setViewportSize({ width: w, height: h });
    const sized = svg.replace('width="512" height="512"', `width="${size}" height="${size}"`);
    await page.setContent(`<html><body style="margin:0;width:${w}px;height:${h}px;display:grid;place-items:center;background:transparent">${sized}</body></html>`);
    const file = path.join(root, out);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    await page.screenshot({ path: file, omitBackground: true });
    console.log(out);
  }
  await browser.close();
})();
