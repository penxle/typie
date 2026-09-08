import { words } from '$lib/landing/text';

export const CLOSING_HEADLINE = ['쓰고 싶어졌다면,', '지금 시작하세요.'].map((line) => words(line));

export const CLOSING_LINE_OFFSETS = CLOSING_HEADLINE.map((_, index) =>
  CLOSING_HEADLINE.slice(0, index).reduce((total, line) => total + line.length, 0),
);

const HEADLINE_COUNT = CLOSING_HEADLINE.reduce((total, line) => total + line.length, 0);
const HEADLINE_STEP = 0.35;
const HEADLINE_SPAN = 0.08;

export const headlineRamp = (index: number): readonly [number, number] => [
  (index / HEADLINE_COUNT) * HEADLINE_STEP,
  (index / HEADLINE_COUNT) * HEADLINE_STEP + HEADLINE_SPAN,
];

export const SUMMARY_RANGE = [0.42, 0.52] as const;
export const ACTION_RANGE = [0.52, 0.62] as const;
export const ASSURANCE_RANGE = [0.6, 0.68] as const;
export const FOOTER_RANGE = [0.72, 0.9] as const;
