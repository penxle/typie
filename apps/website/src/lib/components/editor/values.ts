import BlockquoteLeftLine from './assets/blockquote/left-line.svelte';
import BlockquoteLeftQuote from './assets/blockquote/left-quote.svelte';
import BlockquoteMessageReceived from './assets/blockquote/message-received.svelte';
import BlockquoteMessageSent from './assets/blockquote/message-sent.svelte';
import HorizontalRuleCircle from './assets/horizontal-rule/circle.svelte';
import HorizontalRuleCircleLine from './assets/horizontal-rule/circle-line.svelte';
import HorizontalRuleDashedLine from './assets/horizontal-rule/dashed-line.svelte';
import HorizontalRuleDiamond from './assets/horizontal-rule/diamond.svelte';
import HorizontalRuleDiamondLine from './assets/horizontal-rule/diamond-line.svelte';
import HorizontalRuleLightLine from './assets/horizontal-rule/light-line.svelte';
import HorizontalRuleThreeCircles from './assets/horizontal-rule/three-circles.svelte';
import HorizontalRuleThreeDiamonds from './assets/horizontal-rule/three-diamonds.svelte';
import HorizontalRuleZigzag from './assets/horizontal-rule/zigzag.svelte';
import type { Component } from 'svelte';
import type { BlockquoteVariant, HorizontalRuleVariant } from '$lib/editor/types';

export const horizontalRuleVariants: { variant: HorizontalRuleVariant; component: Component }[] = [
  { variant: 'line', component: HorizontalRuleLightLine },
  { variant: 'dashed_line', component: HorizontalRuleDashedLine },
  { variant: 'circle_line', component: HorizontalRuleCircleLine },
  { variant: 'diamond_line', component: HorizontalRuleDiamondLine },
  { variant: 'circle', component: HorizontalRuleCircle },
  { variant: 'diamond', component: HorizontalRuleDiamond },
  { variant: 'three_circles', component: HorizontalRuleThreeCircles },
  { variant: 'three_diamonds', component: HorizontalRuleThreeDiamonds },
  { variant: 'zigzag', component: HorizontalRuleZigzag },
];

export const blockquoteVariants: { variant: BlockquoteVariant; component: Component }[] = [
  { variant: 'left_line', component: BlockquoteLeftLine },
  { variant: 'left_quote', component: BlockquoteLeftQuote },
  { variant: 'message_sent', component: BlockquoteMessageSent },
  { variant: 'message_received', component: BlockquoteMessageReceived },
];
