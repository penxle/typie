import Clapping from '~icons/twemoji/clapping-hands';
import HundredPoints from '~icons/twemoji/hundred-points';
import LoudlyCryingFace from '~icons/twemoji/loudly-crying-face';
import SparklingHeart from '~icons/twemoji/sparkling-heart';
import ThumbsUp from '~icons/twemoji/thumbs-up';
import type { Component } from 'svelte';

export const emojis: Record<string, Component> = {
  clapping: Clapping,
  'loudly-crying-face': LoudlyCryingFace,
  'sparkling-heart': SparklingHeart,
  'thumbs-up': ThumbsUp,
  'hundred-points': HundredPoints,
};
