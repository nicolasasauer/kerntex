// Clickable links in the PDF preview. pdf.js only paints the page, so every
// link annotation gets a transparent button on top of its page.

import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";

export type PdfLink = {
  /** Position in percent of the page, so it follows every zoom level. */
  left: number;
  top: number;
  width: number;
  height: number;
} & ({ url: string } | { dest: string | unknown[] } | { action: string });

/** Where an internal link points: page index and fraction of that page from the top. */
export type PdfTarget = { index: number; fy: number };

/** A PDF may come from an imported project: only open web and mail addresses. */
const SAFE_URL = /^(https?|mailto):/i;

export async function pageLinks(page: PDFPageProxy): Promise<PdfLink[]> {
  const viewport = page.getViewport({ scale: 1 });
  const links: PdfLink[] = [];
  for (const a of await page.getAnnotations()) {
    if (a.subtype !== "Link") continue;
    const target =
      typeof a.url === "string" && SAFE_URL.test(a.url) ? { url: a.url } : a.dest ? { dest: a.dest } : a.action ? { action: a.action } : null;
    if (!target) continue;
    const [x1, y1, x2, y2] = viewport.convertToViewportRectangle(a.rect);
    links.push({
      left: (Math.min(x1, x2) / viewport.width) * 100,
      top: (Math.min(y1, y2) / viewport.height) * 100,
      width: (Math.abs(x2 - x1) / viewport.width) * 100,
      height: (Math.abs(y2 - y1) / viewport.height) * 100,
      ...target,
    });
  }
  return links;
}

/** Resolves a named or explicit PDF destination. */
export async function resolveDest(doc: PDFDocumentProxy, dest: string | unknown[]): Promise<PdfTarget | null> {
  const explicit = typeof dest === "string" ? await doc.getDestination(dest) : dest;
  if (!explicit) return null;
  const [ref, mode, ...args] = explicit as [any, { name: string }, ...(number | null)[]];
  const index = typeof ref === "number" ? ref : await doc.getPageIndex(ref);
  // The vertical position in PDF coordinates sits in a different slot per destination type.
  const y = mode.name === "XYZ" ? args[1] : mode.name === "FitH" || mode.name === "FitBH" ? args[0] : mode.name === "FitR" ? args[3] : null;
  if (y == null) return { index, fy: 0 };
  const viewport = (await doc.getPage(index + 1)).getViewport({ scale: 1 });
  const [, vy] = viewport.convertToViewportPoint(0, y);
  return { index, fy: Math.max(0, Math.min(1, vy / viewport.height)) };
}
