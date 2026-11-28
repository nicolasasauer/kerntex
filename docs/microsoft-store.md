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

**One click:** *Actions → Ship release → Run workflow* (choose patch / minor / major, or type an exact version). It raises the version on `main`, tags it, builds every installer, publishes the release and submits the `.msix` to the Store. Turn *publish* off to stop at a draft release you check first. If `main` requires pull requests, allow GitHub Actions to bypass that rule (*Settings → Rules*), or use the manual steps below.

By hand:

1. Run `npm run version:set -- X.Y.Z` (updates every file that states the version); merge to `main`.
2. Push the tag `vX.Y.Z` (or publish a GitHub release with that new tag; the Store submission then waits until the `.msix` is attached). The workflow stops if the tag and the project version differ; otherwise it builds everything and attaches the `.msix`.
3. Publish the draft release. With [automatic submission](#automatic-submission-cd) set up, the `.msix` goes to the Store by itself; otherwise in Partner Center → *New submission* → *Packages*: upload the `.msix`. The Store requires each version to be higher than the last; the fourth number is always `0`.
4. Store listing (first time): all texts (English and German), answers for the age rating, properties and the certification notes are in [store-listing.md](store-listing.md); screenshots (1920×1080) in `docs/store/`; privacy policy in [PRIVACY.md](../PRIVACY.md).
5. Age rating questionnaire, pricing (free), submit.

## Automatic submission (CD)

Once the **first** submission has gone through by hand (Partner Center needs the listing, age rating and pricing once), `.github/workflows/store.yml` takes over step 3: when you **publish** the draft release, it downloads the `.msix` from that release and submits it to the Store with the [Microsoft Store Developer CLI](https://learn.microsoft.com/windows/apps/publish/msstore-dev-cli/overview). Listing texts and screenshots are kept from the previous submission.

One-time setup:

1. **Entra ID app** (lets the workflow sign in): Partner Center → ⚙️ *Account settings* → *User management* → *Microsoft Entra applications* → **Create Azure AD application** (role *Manager*). Note the *Tenant ID* and *Client ID*, then *Add new key* and copy the secret (shown once; it expires, renew it before then).
2. **Seller ID**: Partner Center → *Account settings* → *Legal info* → *Developer* → *Seller ID*.
3. **Product ID**: the 12-character Store ID of KernTeX (*Product management → Product identity*, e.g. `9NXXXXXXXXXX`).
4. In GitHub, *Settings → Environments* → create **`microsoft-store`** (optionally with yourself as required reviewer, so each submission waits for a click) and add:

   | Kind | Name | Value |
   | --- | --- | --- |
   | Secret | `AZURE_AD_TENANT_ID` | Tenant ID |
   | Secret | `AZURE_AD_CLIENT_ID` | Client ID |
   | Secret | `AZURE_AD_CLIENT_SECRET` | the key from step 1 |
   | Secret | `SELLER_ID` | Seller ID |
   | Variable | `MSSTORE_PRODUCT_ID` | Product ID |

5. Dry run: *Actions → Microsoft Store → Run workflow* with a released tag and *commit* off. That uploads the package into a draft submission without sending it to certification; check it in Partner Center and delete it or submit it there.

**What's new in this version:** each release body starts with a *What's new* section listing the commits since the previous tag. Edit it in the draft release before publishing if you like; the Store gets that section as plain text (all languages, at most 1500 characters). A release without the section keeps the Store's previous text, and if setting it fails, the update is submitted anyway (with a warning in the run).

Only one submission can be in progress at a time: if the previous one is still in certification, the run fails and can be re-run later. Updates via the CLI work for free products only (KernTeX is free).

## Things to know

- **Capability `runFullTrust`:** KernTeX is a classic desktop app (it reads and writes the folders you open and runs the TeX engine), so the package declares `runFullTrust`. Partner Center asks why — answer: "LaTeX editor that opens user-chosen project folders and runs a bundled TeX engine (Tectonic) as a separate process."
- **WebView2:** the UI uses Microsoft Edge WebView2, which ships with Windows 11 and up-to-date Windows 10.
- **App data:** MSIX redirects writes to `%LOCALAPPDATA%` into a per-app location, so Tectonic's package cache is removed cleanly when the app is uninstalled. Projects stay where the user saved them (usually *Documents*).
- **Updates:** the Store updates the app automatically; no auto-updater is needed for Store installs.
