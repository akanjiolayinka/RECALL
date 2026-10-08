/** A piece of text, marked if it should be highlighted. */
export interface TextPart {
  text: string;
  highlight: boolean;
}

const escapeRegExp = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Split `text` so that occurrences of `words` (case-insensitive, at the start
 * of a word, so "budget" also marks "budgets") are highlighted.
 */
export function highlightWords(text: string, words: string[]): TextPart[] {
  const unique = [...new Set(words.map((w) => w.trim()).filter(Boolean))];
  if (unique.length === 0 || text === "") return text ? [{ text, highlight: false }] : [];

  // Longest first, so "garden party" wins over "garden" when both are given.
  unique.sort((a, b) => b.length - a.length);
  const pattern = new RegExp(`(?<![\\p{L}\\p{N}])(?:${unique.map(escapeRegExp).join("|")})[\\p{L}\\p{N}]*`, "giu");

  const parts: TextPart[] = [];
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    const start = match.index ?? 0;
    if (start > last) parts.push({ text: text.slice(last, start), highlight: false });
    parts.push({ text: match[0], highlight: true });
    last = start + match[0].length;
  }
  if (last < text.length) parts.push({ text: text.slice(last), highlight: false });
  return parts;
}
