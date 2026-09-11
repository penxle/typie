import ClappingHands from '~icons/twemoji/clapping-hands';
import HundredPoints from '~icons/twemoji/hundred-points';
import LoudlyCryingFace from '~icons/twemoji/loudly-crying-face';
import SparklingHeart from '~icons/twemoji/sparkling-heart';
import ThumbsUp from '~icons/twemoji/thumbs-up';
import type { Component } from 'svelte';

export const ADDRESS = 'https://typie.me/aB3xK9';

export const EMOJIS: readonly Component[] = [ClappingHands, LoudlyCryingFace, SparklingHeart, ThumbsUp, HundredPoints];

// 원고 낱말이 하나씩 들어서는 창 — 마지막 낱말은 창 끝에서 시작해 DOC_SPAN 뒤에 끝난다
export const DOC_SPAN = 0.05;
export const DOC_REVEAL_RANGE = [0, 0.24] as const;

export const SHARE_FLOW = {
  rowIn: [0.32, 0.38],
  on: [0.4, 0.44],
  addressIn: [0.46, 0.52],
  button: [0.5, 0.54],
} as const;

export const REACTION = { start: 0.54, step: 0.017, span: 0.04 } as const;

export const REACTION_SEQUENCE: readonly number[] = [0, 2, 3, 0, 4, 2, 1, 0, 3, 2, 4, 0];
