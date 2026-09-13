export type HighlightPart = { text: string; hit: boolean };

const escapeRegExp = (value: string) => value.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);

export const splitHighlight = (text: string, query: string): HighlightPart[] => {
  const terms = query
    .trim()
    .split(/\s+/)
    .filter((term) => term.length > 0);
  if (text.length === 0 || terms.length === 0) return [{ text, hit: false }];

  const pattern = new RegExp(terms.map((term) => escapeRegExp(term)).join('|'), 'gi');
  const parts: HighlightPart[] = [];
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > last) parts.push({ text: text.slice(last, match.index), hit: false });
    parts.push({ text: match[0], hit: true });
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push({ text: text.slice(last), hit: false });
  return parts;
};
