// Fingerprint of a file's content: FNV-1a (32 bit) over the UTF-8 bytes, as
// 8 hex digits. Must match `project::content_hash` in the Rust core, which
// uses it to refuse a save when the file changed on disk since it was loaded.

const encoder = new TextEncoder();

export function contentHash(text: string): string {
  let h = 0x811c9dc5;
  for (const b of encoder.encode(text)) {
    h ^= b;
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}
