# Publishing to the Microsoft Store

The release workflow builds an **MSIX package** (`KernTeX_<version>_x64.msix`) next to the `.exe`/`.msi` installers and attaches it to the GitHub release. That package is what you upload to Partner Center.

## Signing

**No certificate is needed for the Store.** Microsoft re-signs MSIX packages during certification, which also means Store users never see a SmartScreen warning. You only need your own certificate to install the `.msix` directly on a PC for testing (outside the Store).

## One-time setup

1. **Reserve the name** in [Partner Center](https://partner.microsoft.com/dashboard) → *Apps and games* → *New product* → *MSIX or PWA app* → name `KernTeX`.
2. Open the new product → *Product management* → **Product identity**. Copy three values:

   | Partner Center field | GitHub variable |
   | --- | --- |
   | `Package/Identity/Name` | `MSIX_IDENTITY_NAME` |
   | `Package/Identity/Publisher` (starts with `CN=`) | `MSIX_PUBLISHER` |
   | `Package/Properties/PublisherDisplayName` | `MSIX_PUBLISHER_DISPLAY_NAME` |

3. In GitHub: *Settings → Secrets and variables → Actions → **Variables*** → add the three variables. They're not secret; they appear in the package anyway.

Without these variables the workflow still builds an MSIX (with a placeholder identity, so CI can check it packs), but Partner Center rejects it — the identity must match exactly.

## Each release

1. Bump `version` in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`; merge to `main`.
2. Publish a GitHub release with a new tag `vX.Y.Z` (or push the tag). The workflow builds everything and attaches the `.msix`.
3. In Partner Center → *New submission* → *Packages*: upload the `.msix`. The Store requires each version to be higher than the last; the fourth number is always `0`.
4. Store listing (first time): all texts (English and German), answers for the age rating, properties and the certification notes are in [store-listing.md](store-listing.md); screenshots (1920×1080) in `docs/store/`; privacy policy in [PRIVACY.md](../PRIVACY.md).
5. Age rating questionnaire, pricing (free), submit.

## Things to know

- **Capability `runFullTrust`:** KernTeX is a classic desktop app (it reads and writes the folders you open and runs the TeX engine), so the package declares `runFullTrust`. Partner Center asks why — answer: "LaTeX editor that opens user-chosen project folders and runs a bundled TeX engine (Tectonic) as a separate process."
- **WebView2:** the UI uses Microsoft Edge WebView2, which ships with Windows 11 and up-to-date Windows 10.
- **App data:** MSIX redirects writes to `%LOCALAPPDATA%` into a per-app location, so Tectonic's package cache is removed cleanly when the app is uninstalled. Projects stay where the user saved them (usually *Documents*).
- **Updates:** the Store updates the app automatically; no auto-updater is needed for Store installs.
