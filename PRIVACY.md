# KernTeX Privacy Policy

*Last updated: 23 November 2026 (Typst packages added)*

KernTeX is a LaTeX and Typst editor that runs entirely on your computer. It has no user accounts, no analytics, no telemetry, no ads and no tracking.

## What KernTeX stores

- **Your documents** stay in the folders you open or create. KernTeX reads and writes only there; build output goes to a `.kerntex` folder inside your project.
- **Settings and the list of recent projects** are stored locally on your computer.
- Nothing of this is sent anywhere by KernTeX.

## Network connections

KernTeX itself does not contact any server. Three things can use the internet:

1. **The built-in TeX engine (Tectonic)** downloads the LaTeX packages a document needs from the Tectonic project's package server — always on the first compile, later only for packages it hasn't downloaded yet. Like any web server, that server receives your IP address and the names of the requested package files. It does **not** receive your documents. The downloaded packages are cached on your computer. This is operated by the open-source Tectonic project ([tectonic-typesetting.github.io](https://tectonic-typesetting.github.io)), not by KernTeX.
2. **Typst packages.** If a Typst document imports a package (`#import "@preview/…"`), the built-in Typst compiler downloads it once from Typst Universe (packages.typst.org) and caches it on your computer. That server receives your IP address and the package name, not your documents. Documents without such imports never go online.
3. **Links you click** (for example "Look up on CTAN" or "KernTeX on GitHub") open in your web browser; the browser's and the website's privacy policies apply.

If you use your own TeX distribution (latexmk with TeX Live or MiKTeX), its own package manager decides when to go online.

## Children

KernTeX doesn't collect personal data from anyone, including children.

## Changes

Changes to this policy are published in this file in the KernTeX repository, with a new date above.

## Contact

Questions: open an issue at [github.com/nicolasasauer/kerntex/issues](https://github.com/nicolasasauer/kerntex/issues).

---

# Datenschutzerklärung (Deutsch)

KernTeX ist ein LaTeX- und Typst-Editor, der vollständig auf deinem Computer läuft. Es gibt keine Benutzerkonten, keine Analyse, keine Telemetrie, keine Werbung und kein Tracking.

- **Deine Dokumente** bleiben in den Ordnern, die du öffnest oder anlegst. Einstellungen und die Liste zuletzt geöffneter Projekte werden nur lokal gespeichert. KernTeX überträgt davon nichts.
- **Die eingebaute TeX-Engine (Tectonic)** lädt die LaTeX-Pakete, die ein Dokument braucht, vom Paketserver des Tectonic-Projekts herunter – immer beim ersten Kompilieren, später nur für noch fehlende Pakete. Dieser Server erhält dabei wie jeder Webserver deine IP-Adresse und die Namen der angeforderten Paketdateien, **nicht** deine Dokumente. Betreiber ist das Open-Source-Projekt Tectonic, nicht KernTeX.
- **Typst-Pakete:** Importiert ein Typst-Dokument ein Paket (`#import "@preview/…"`), lädt der eingebaute Typst-Compiler es einmalig von Typst Universe (packages.typst.org) und speichert es lokal. Der Server erhält dabei deine IP-Adresse und den Paketnamen, nicht deine Dokumente.
- **Links**, die du anklickst (z. B. zu CTAN oder GitHub), öffnen sich in deinem Browser; dort gelten die Datenschutzbestimmungen des Browsers und der Website.

Fragen: [github.com/nicolasasauer/kerntex/issues](https://github.com/nicolasasauer/kerntex/issues).
