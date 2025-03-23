import IconAlignCenter from '~icons/lucide/align-center';
import IconAlignJustify from '~icons/lucide/align-justify';
import IconAlignLeft from '~icons/lucide/align-left';
import IconAlignRight from '~icons/lucide/align-right';
import IconList from '~icons/lucide/list';
import IconListOrdered from '~icons/lucide/list-ordered';
import Blockquote1 from './assets/blockquote/1.svelte';
import Blockquote2 from './assets/blockquote/2.svelte';
import HorizontalRule1 from './assets/horizontal-rule/1.svelte';
import HorizontalRule2 from './assets/horizontal-rule/2.svelte';
import HorizontalRule3 from './assets/horizontal-rule/3.svelte';
import HorizontalRule4 from './assets/horizontal-rule/4.svelte';
import HorizontalRule5 from './assets/horizontal-rule/5.svelte';
import HorizontalRule6 from './assets/horizontal-rule/6.svelte';
import HorizontalRule7 from './assets/horizontal-rule/7.svelte';

export const values = {
  defaultColor: '#09090B',

  fontFamily: [{ label: '프리텐다드', value: 'Pretendard' }],

  fontSize: [
    { label: '8pt', value: 8 },
    { label: '10pt', value: 10 },
    { label: '12pt', value: 12 },
    { label: '14pt', value: 14 },
    { label: '16pt', value: 16 },
    { label: '18pt', value: 18 },
    { label: '20pt', value: 20 },
    { label: '22pt', value: 22 },
    { label: '24pt', value: 24 },
    { label: '36pt', value: 36 },
    { label: '48pt', value: 48 },
    { label: '60pt', value: 60 },
    { label: '72pt', value: 72 },
  ],

  lineHeight: [
    { label: '80%', value: 0.8 },
    { label: '100%', value: 1 },
    { label: '120%', value: 1.2 },
    { label: '140%', value: 1.4 },
    { label: '160%', value: 1.6 },
    { label: '180%', value: 1.8 },
    { label: '200%', value: 2 },
    { label: '220%', value: 2.2 },
  ],

  letterSpacing: [
    { label: '-10%', value: -0.1 },
    { label: '-5%', value: -0.05 },
    { label: '0%', value: 0 },
    { label: '5%', value: 0.05 },
    { label: '10%', value: 0.1 },
    { label: '20%', value: 0.2 },
    { label: '40%', value: 0.4 },
  ],

  textAlign: [
    { label: '왼쪽', value: 'left', icon: IconAlignLeft },
    { label: '중앙', value: 'center', icon: IconAlignCenter },
    { label: '오른쪽', value: 'right', icon: IconAlignRight },
    { label: '양쪽', value: 'justify', icon: IconAlignJustify },
  ],

  blockquote: [
    { label: '인용구 1', type: 'blockquote_1', component: Blockquote1 },
    { label: '인용구 2', type: 'blockquote_2', component: Blockquote2 },
  ],

  horizontalRule: [
    { label: '구분선 1', type: 'horizontal_rule_1', component: HorizontalRule1 },
    { label: '구분선 2', type: 'horizontal_rule_2', component: HorizontalRule2 },
    { label: '구분선 3', type: 'horizontal_rule_3', component: HorizontalRule3 },
    { label: '구분선 4', type: 'horizontal_rule_4', component: HorizontalRule4 },
    { label: '구분선 5', type: 'horizontal_rule_5', component: HorizontalRule5 },
    { label: '구분선 6', type: 'horizontal_rule_6', component: HorizontalRule6 },
    { label: '구분선 7', type: 'horizontal_rule_7', component: HorizontalRule7 },
  ],

  list: [
    {
      label: '순서 없는 목록',
      value: 'bullet_list',
      icon: IconList,
    },
    {
      label: '순서 있는 목록',
      value: 'ordered_list',
      icon: IconListOrdered,
    },
  ],

  paragraphIndent: [
    { label: '없음', value: 0 },
    { label: '0.5칸', value: 0.5 },
    { label: '1칸', value: 1 },
    { label: '2칸', value: 2 },
  ],

  blockGap: [
    { label: '없음', value: 0 },
    { label: '0.5줄', value: 0.5 },
    { label: '1줄', value: 1 },
    { label: '2줄', value: 2 },
  ],
} as const;
