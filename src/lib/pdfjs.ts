// pdf.js is ~1 MB: load it once, only when the first PDF is shown.
type Pdfjs = typeof import("pdfjs-dist");
let pdfjs: Promise<Pdfjs> | null = null;

export function loadPdfjs(): Promise<Pdfjs> {
  pdfjs ??= Promise.all([import("pdfjs-dist"), import("pdfjs-dist/build/pdf.worker.min.mjs?url")]).then(([lib, worker]) => {
    lib.GlobalWorkerOptions.workerSrc = worker.default;
    return lib;
  });
  return pdfjs;
}
