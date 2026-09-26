export interface Entry {
  title: string;
  href: string;
  description: string;
  section: string;
  /** Other names for the entry, such as "toggle" for a switch. */
  keywords?: string[];
}

/** Scores entries against a query; higher is better, zero means no match. */
export function score(entry: Entry, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;

  const title = entry.title.toLowerCase();
  if (title === q) return 100;
  if (title.startsWith(q)) return 80;

  const keywords = (entry.keywords ?? []).map((keyword) => keyword.toLowerCase());
  if (keywords.includes(q)) return 70;
  if (title.split(/\s+/).some((word) => word.startsWith(q))) return 60;
  if (title.includes(q)) return 50;
  if (keywords.some((keyword) => keyword.split(/\s+/).some((word) => word.startsWith(q)))) return 40;
  if (keywords.some((keyword) => keyword.includes(q))) return 30;
  if (entry.description.toLowerCase().includes(q)) return 20;
  if (isSubsequence(q, title)) return 10;
  return 0;
}

function isSubsequence(needle: string, haystack: string): boolean {
  let i = 0;
  for (const char of haystack) {
    if (char === needle[i]) i += 1;
    if (i === needle.length) return true;
  }
  return false;
}

/** Returns matching entries, best first, keeping the index order for ties. */
export function rank(entries: Entry[], query: string): Entry[] {
  return entries
    .map((entry, index) => ({ entry, index, score: score(entry, query) }))
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .map((item) => item.entry);
}
