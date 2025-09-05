import IconAlignCenter from '~icons/lucide/align-center';
import IconAlignJustify from '~icons/lucide/align-justify';
import IconAlignLeft from '~icons/lucide/align-left';
import IconAlignRight from '~icons/lucide/align-right';
import BlockquoteLeftLine from './assets/blockquote/left-line.svelte';
import BlockquoteLeftQuote from './assets/blockquote/left-quote.svelte';
import HorizontalRuleCircle from './assets/horizontal-rule/circle.svelte';
import HorizontalRuleCircleLine from './assets/horizontal-rule/circle-line.svelte';
import HorizontalRuleDashedLine from './assets/horizontal-rule/dashed-line.svelte';
import HorizontalRuleDiamond from './assets/horizontal-rule/diamond.svelte';
import HorizontalRuleDiamondLine from './assets/horizontal-rule/diamond-line.svelte';
import HorizontalRuleLightLine from './assets/horizontal-rule/light-line.svelte';
import HorizontalRuleThreeCircles from './assets/horizontal-rule/three-circles.svelte';
import HorizontalRuleThreeDiamonds from './assets/horizontal-rule/three-diamonds.svelte';
import HorizontalRuleZigzag from './assets/horizontal-rule/zigzag.svelte';
import { defaultValues as defaultValuesBase, values as valuesBase } from './values-base';

export const values = {
  ...valuesBase,

  textAlign: [
    { label: '왼쪽', value: 'left', icon: IconAlignLeft },
    { label: '중앙', value: 'center', icon: IconAlignCenter },
    { label: '오른쪽', value: 'right', icon: IconAlignRight },
    { label: '양쪽', value: 'justify', icon: IconAlignJustify },
  ],

  blockquote: [
    { label: '왼쪽 선', type: 'left-line', component: BlockquoteLeftLine },
    { label: '왼쪽 따옴표', type: 'left-quote', component: BlockquoteLeftQuote },
  ],

  horizontalRule: [
    { label: '옅은 선', type: 'light-line', component: HorizontalRuleLightLine },
    { label: '점선', type: 'dashed-line', component: HorizontalRuleDashedLine },
    { label: '동그라미가 있는 선', type: 'circle-line', component: HorizontalRuleCircleLine },
    { label: '마름모가 있는 선', type: 'diamond-line', component: HorizontalRuleDiamondLine },
    { label: '동그라미', type: 'circle', component: HorizontalRuleCircle },
    { label: '마름모', type: 'diamond', component: HorizontalRuleDiamond },
    { label: '세 개의 동그라미', type: 'three-circles', component: HorizontalRuleThreeCircles },
    { label: '세 개의 마름모', type: 'three-diamonds', component: HorizontalRuleThreeDiamonds },
    { label: '지그재그', type: 'zigzag', component: HorizontalRuleZigzag },
  ],
} as const;

export const defaultValues = {
  ...defaultValuesBase,
} as const;
