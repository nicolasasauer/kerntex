# Microsoft Store submission — texts and answers

Copy-paste material for Partner Center. Keep it in sync with the app; update *What's new* for every submission. Limits in brackets are Partner Center's.

---

## 1. Pricing and availability

| Field | Value |
| --- | --- |
| Markets | All markets (default) |
| Visibility | **Public** — or *Private audience* for a first test round |
| Pricing | **Free**, no free trial |
| Publish date | As soon as possible after certification (or manually, if you want to check first) |

## 2. Properties

| Field | Value |
| --- | --- |
| Category | **Productivity** (no subcategory) |
| Privacy policy URL | `https://github.com/nicolasasauer/kerntex/blob/main/PRIVACY.md` *(needs a public repository)* |
| Website | `https://github.com/nicolasasauer/kerntex` |
| Support contact info | `https://github.com/nicolasasauer/kerntex/issues` |
| Display mode | Desktop (default) |

**Product declarations:**
- ☑ Customers can install this app to alternate drives or removable storage
- ☑ Windows can include this app's data in automatic backups
- ☐ This app has been tested to meet accessibility guidelines *(leave unchecked until tested)*
- ☐ Kinect / pen / etc. — nothing else applies

**System requirements:** Minimum — Memory 4 GB, architecture x64. Leave the rest at "not specified". Recommended — Memory 8 GB, internet connection for the first compile.

## 3. Age ratings (IARC questionnaire)

- Category: **App (not a game)** — e.g. "All other app types"
- Violence, fear, sexuality, language, drugs, gambling: **No** to all
- Does the app let users interact or exchange content with other users? **No**
- Does the app share the user's location with others? **No**
- Does the app allow purchases of digital goods? **No**
- Does the app give unrestricted internet access (e.g. a web browser)? **No** (only downloads TeX packages)
- Does the app collect or share personal information? **No**

Expected result: **3+ / Everyone**.

## 4. Store listing — English (en-us)

**Product name:** KernTeX

**Short title** [50]: `KernTeX – LaTeX Editor`

**Short description** [1000, shown at the top]:

> A fast, modern LaTeX editor that runs on your PC: file tree, editor and live PDF side by side, with clear error messages and a built-in TeX engine. Nothing else to install, no account, no cloud.

**Description** [10,000]:

> KernTeX is a fast, modern LaTeX editor for your desktop. It feels like the online editors you know — file tree, editor and PDF preview side by side — but your documents stay on your computer and it works offline.
>
> **Nothing else to install.** KernTeX comes with Tectonic, a modern TeX engine that fetches the LaTeX packages your document needs automatically. Install KernTeX, open it, start writing. The first start opens a short, compiled tour of what LaTeX can do.
>
> **Errors you can actually fix.** Every problem tells you where it comes from (LaTeX, a package, the bibliography, the engine), which file and line it's on — one click takes you there — and how to fix it, in plain language. Missing packages are noticed while you type and added with one click.
>
> **Made for writing.** Save and the PDF updates. Autocompletion knows your labels and citation keys. Drag images onto the file tree, preview them, insert a ready-made figure. Everything is one shortcut away in the command palette (Ctrl+K). Light and dark mode.
>
> **Works with what you have.** KernTeX edits ordinary folders: import an Overleaf project as ZIP, export it back, use it with Git. Prefer TeX Live or MiKTeX? Switch to latexmk with pdfLaTeX, XeLaTeX or LuaLaTeX.
>
> **Open source.** KernTeX is free and open source (MIT license), written in Rust. No ads, no tracking, no telemetry.

**What's new in this version** [1500]:

> First release in the Microsoft Store: live PDF preview, clear error messages with one-click fixes, built-in TeX engine, image previews and figure insertion, build details, Overleaf ZIP import/export, light and dark mode.

**Product features** [20 × 200]:

1. Built-in TeX engine (Tectonic) — no TeX installation needed
2. File tree, editor and live PDF preview side by side
3. Save to compile; progress is shown and builds can be stopped
4. Clear error messages with source, file:line and a plain-language fix
5. Detects missing packages while you type and adds them with one click
6. Autocompletion for commands, environments, labels and citations
7. Drag & drop images, preview them and insert a figure in one click
8. Import and export Overleaf projects as ZIP; clean ZIP for arXiv
9. Use your own TeX Live or MiKTeX via latexmk (pdfLaTeX, XeLaTeX, LuaLaTeX)
10. Command palette with every action and shortcut (Ctrl+K)
11. Light and dark mode, follows your system setting
12. Templates for articles, theses, presentations and letters
13. Free and open source, written in Rust — no account, no tracking

**Screenshots** (files in `docs/store/`, 1920×1080) with captions [200]:

| File | Caption |
| --- | --- |
| `1-editor-light.png` | File tree, editor and live PDF side by side |
| `2-editor-dark.png` | Dark mode, designed with the same care |
| `3-errors.png` | Clear errors with a fix — and missing packages added in one click |
| `4-figures.png` | Preview images and insert a figure with one click |
| `5-build-details.png` | See the engine and every package your document uses |
| `6-command-palette.png` | Every action one shortcut away |

**Store logos:** optional — Partner Center takes the tiles from the package. If asked for a 1:1 box art, use `assets/logo.png` (1024×1024).

**Search terms** [7]: `LaTeX`, `TeX`, `LaTeX editor`, `Overleaf`, `thesis`, `PDF`, `BibTeX`

**Copyright and trademark info** [200]: `© 2026 Nicolas Sauer. Open source under the MIT license.`

**Additional license terms:** `https://github.com/nicolasasauer/kerntex/blob/main/LICENSE`

**Developed by:** `Nicolas Sauer`

## 5. Store listing — German (de-de, optional but recommended)

Add the language under *Store listings → Manage additional languages → German*.

**Short title:** `KernTeX – LaTeX-Editor`

**Short description:**

> Ein schneller, moderner LaTeX-Editor für deinen PC: Dateibaum, Editor und Live-PDF nebeneinander, mit verständlichen Fehlermeldungen und eingebauter TeX-Engine. Keine weitere Installation, kein Konto, keine Cloud.

**Description:**

> KernTeX ist ein schneller, moderner LaTeX-Editor für den Desktop. Er fühlt sich an wie die bekannten Online-Editoren – Dateibaum, Editor und PDF-Vorschau nebeneinander –, aber deine Dokumente bleiben auf deinem Computer, und er funktioniert offline.
>
> **Nichts weiter zu installieren.** KernTeX bringt Tectonic mit, eine moderne TeX-Engine, die die benötigten LaTeX-Pakete automatisch lädt. Installieren, öffnen, schreiben. Beim ersten Start öffnet sich eine kurze, fertig kompilierte Tour durch LaTeX.
>
> **Fehler, die man versteht.** Jede Meldung sagt, woher sie kommt (LaTeX, ein Paket, das Literaturverzeichnis, die Engine), in welcher Datei und Zeile – ein Klick springt hin – und wie man sie behebt, in Alltagssprache. Fehlende Pakete erkennt KernTeX schon beim Tippen und fügt sie mit einem Klick hinzu.
>
> **Zum Schreiben gemacht.** Speichern, und das PDF aktualisiert sich. Die Autovervollständigung kennt deine Labels und Literaturschlüssel. Bilder in den Dateibaum ziehen, ansehen, fertige Abbildung einfügen. Jede Aktion ist per Befehlspalette (Strg+K) erreichbar. Heller und dunkler Modus.
>
> **Passt zu deinem Workflow.** KernTeX bearbeitet normale Ordner: Overleaf-Projekte als ZIP importieren und wieder exportieren, mit Git verwenden. Lieber TeX Live oder MiKTeX? Einfach auf latexmk mit pdfLaTeX, XeLaTeX oder LuaLaTeX umstellen.
>
> **Open Source.** KernTeX ist kostenlos und quelloffen (MIT-Lizenz), geschrieben in Rust. Keine Werbung, kein Tracking, keine Telemetrie.

**Neuigkeiten in dieser Version:**

> Erste Version im Microsoft Store: Live-PDF-Vorschau, verständliche Fehlermeldungen mit Ein-Klick-Lösungen, eingebaute TeX-Engine, Bildvorschau und Abbildungen einfügen, Build-Details, Overleaf-ZIP-Import/-Export, heller und dunkler Modus.

**Produktfeatures:**

1. Eingebaute TeX-Engine (Tectonic) – keine TeX-Installation nötig
2. Dateibaum, Editor und Live-PDF-Vorschau nebeneinander
3. Speichern kompiliert; Fortschritt sichtbar, Abbrechen jederzeit möglich
4. Verständliche Fehlermeldungen mit Quelle, Datei:Zeile und Lösungshinweis
5. Erkennt fehlende Pakete beim Tippen und fügt sie mit einem Klick hinzu
6. Autovervollständigung für Befehle, Umgebungen, Labels und Zitate
7. Bilder per Drag & Drop hinzufügen, ansehen und als Abbildung einfügen
8. Overleaf-Projekte als ZIP importieren und exportieren; bereinigtes ZIP für arXiv
9. Eigene TeX-Distribution per latexmk nutzbar (pdfLaTeX, XeLaTeX, LuaLaTeX)
10. Befehlspalette mit allen Aktionen und Tastenkürzeln (Strg+K)
11. Heller und dunkler Modus, folgt der Systemeinstellung
12. Vorlagen für Artikel, Abschlussarbeiten, Präsentationen und Briefe
13. Kostenlos und Open Source, geschrieben in Rust – kein Konto, kein Tracking

**Screenshot-Beschriftungen:** 1 „Dateibaum, Editor und Live-PDF nebeneinander“ · 2 „Dunkler Modus, genauso sorgfältig gestaltet“ · 3 „Fehler mit Lösung – fehlende Pakete per Klick ergänzt“ · 4 „Bilder ansehen und mit einem Klick als Abbildung einfügen“ · 5 „Engine und alle verwendeten Pakete auf einen Blick“ · 6 „Jede Aktion nur ein Tastenkürzel entfernt“

**Suchbegriffe:** `LaTeX`, `TeX`, `LaTeX Editor`, `Overleaf`, `Abschlussarbeit`, `PDF`, `BibTeX`

## 6. Submission options — Notes for certification

> KernTeX is a LaTeX editor (desktop app, open source: github.com/nicolasasauer/kerntex).
>
> runFullTrust: needed because KernTeX opens and saves project folders the user chooses anywhere on disk and runs its bundled TeX engine (Tectonic, tectonic.exe in the package) as a separate process to produce PDFs.
>
> How to test: on first start, KernTeX creates and opens the example project "Welcome to KernTeX" in the Documents folder and compiles it. The FIRST compile needs an internet connection: Tectonic downloads the required LaTeX packages from its package server (this can take a minute and is shown in the toolbar); later compiles work offline. No account or sign-in is needed.
