/**
 * Escapes an API signature for HTML and adds a break opportunity after
 * each `::`, so a long signature wraps at its spaces or between path
 * segments rather than in the middle of a name.
 */
export function breakableSignature(name: string): string {
  return name
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("::", "::<wbr>");
}
