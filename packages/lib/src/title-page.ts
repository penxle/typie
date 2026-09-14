export type TitlePageColors = {
  base: string;
  background: string;
  frame: string;
  title: string;
  rule: string;
  caption: string;
};

export const TITLE_PAGE_PALETTE: readonly TitlePageColors[] = [
  { base: '#d77b64', background: '#ffe0d8', frame: '#f8beaf', title: '#74301f', rule: '#e29480', caption: '#8f4f3f' },
  { base: '#6faa62', background: '#d9f0d3', frame: '#b6daae', title: '#26541c', rule: '#89bb7e', caption: '#46703d' },
  { base: '#48a1db', background: '#d2ecff', frame: '#a4d5f7', title: '#004d74', rule: '#6db3e4', caption: '#2b6991' },
  { base: '#b97fc6', background: '#f6dffb', frame: '#e3c0eb', title: '#5f3369', rule: '#c796d2', caption: '#7a5283' },
  { base: '#d5778e', background: '#ffdee4', frame: '#f7bbc7', title: '#722d40', rule: '#e190a2', caption: '#8e4c5c' },
  { base: '#00afa8', background: '#c7f2ee', frame: '#97ddd8', title: '#005350', rule: '#52bfb9', caption: '#00736e' },
  { base: '#c38c37', background: '#fae4c7', frame: '#e9c89b', title: '#624000', rule: '#d0a260', caption: '#815b1f' },
  { base: '#8a8fe1', background: '#e3e6ff', frame: '#c4c9fc', title: '#3e3f7c', rule: '#9ea4e9', caption: '#595d96' },
];

export const hashText = (text: string): number => {
  let hash = 2_166_136_261;
  for (const char of text) {
    hash ^= char.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
};

export const titlePageColors = (seed: string): TitlePageColors => TITLE_PAGE_PALETTE[hashText(seed) % TITLE_PAGE_PALETTE.length];

const TITLE_PAGE_COVER_REVISION = 3;

export const titlePageCoverVersion = (input: { title: string; spaceName: string }): string =>
  hashText(`${TITLE_PAGE_COVER_REVISION}\n${input.title}\n${input.spaceName}`).toString(36);
