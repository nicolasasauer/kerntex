# KernTeX logo

A K and a lowered T, interlocked like the letters of the TeX logo, with a small "eX" under the T's arm — set in Latin Modern Roman Bold, the classic TeX typeface. Navy `#1f2a5c`, cream `#fbf7ef`, gold `#ffc94d`.

| File | Use |
| --- | --- |
| `logo.svg` | Full logo (48 px and larger): app icon, Store, installer, README |
| `logo-small.svg` | Without "eX", for 16–32 px (taskbar, title bar, in-app toolbar) |

Both are pure vector paths, no font needed to display them.

## Regenerating all icons

```sh
python3 assets/logo/build_logo.py        # SVGs from the font (needs fontTools + lmodern fonts)
node assets/logo/render.cjs              # PNGs for MSIX tiles, Store logos, assets/logo.png (needs Playwright)
npx tauri icon assets/logo.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/Square*.png src-tauri/icons/StoreLogo.png
python3 assets/logo/make_ico.py          # icon.ico with the small art for 16–32 px
```
