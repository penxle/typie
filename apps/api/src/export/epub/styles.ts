import type { FontFile } from './fonts';

export type DocDefaults = {
  fontFamily: string;
  fontSizePt: number;
  lineHeight: number;
  blockGapPx: number;
  paragraphIndentPx: number;
};

function escapeCSSString(value: string): string {
  return value.replaceAll('\\', String.raw`\\`).replaceAll("'", String.raw`\'`);
}

export function generateStylesheet(fonts: Map<string, FontFile>, docDefaults: DocDefaults): string {
  const { fontFamily, fontSizePt, lineHeight, blockGapPx, paragraphIndentPx } = docDefaults;

  const fontFaces = [...fonts.values()]
    .map(
      (f) => `@font-face {
  font-family: '${escapeCSSString(f.familyName)}';
  font-weight: ${f.weight};
  src: url('fonts/${f.filename}') format('woff2');
}`,
    )
    .join('\n\n');

  return `${fontFaces}

body {
  font-family: '${escapeCSSString(fontFamily)}', sans-serif;
  font-size: ${fontSizePt}pt;
  line-height: ${lineHeight}%;
  margin: 0;
  padding: 0;
}

p {
  margin: 0 0 ${blockGapPx}px 0;
  text-indent: ${paragraphIndentPx}px;
}

blockquote {
  margin: 0 0 ${blockGapPx}px 1em;
}

blockquote.left-line {
  border-left: 3px solid currentColor;
  padding-left: 12px;
  margin-left: 0;
}

blockquote.callout, blockquote.fold, blockquote.message-sent, blockquote.message-received {
  border: 1px solid currentColor;
  padding: 8px 12px;
  margin-left: 0;
}

blockquote.message-sent, blockquote.message-received {
  width: fit-content;
  max-width: 75%;
}

blockquote.message-sent {
  margin-left: auto;
}

td > *:last-child, blockquote > *:last-child, details > *:last-child, li > *:last-child { margin-bottom: 0; }

table { border-collapse: collapse; width: 100%; margin: 0 0 ${blockGapPx}px 0; }
td { padding: 4px 8px; }

hr { margin: ${blockGapPx}px 0; }

details { margin: 0 0 ${blockGapPx}px 0; }
summary { font-weight: bold; cursor: pointer; }
details[open] > summary { margin-bottom: 0.5em; }

img { max-width: 100%; height: auto; }
`;
}
