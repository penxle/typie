import { token } from '@typie/styled-system/tokens';
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

export const values = {
  fontFamily: [
    { label: '프리텐다드', value: 'Pretendard', weights: [100, 200, 300, 400, 500, 600, 700, 800, 900] },
    { label: '코펍월드돋움', value: 'KoPubWorldDotum', weights: [500, 700] },
    { label: '나눔바른고딕', value: 'NanumBarunGothic', weights: [400, 700] },
    { label: '리디바탕', value: 'RIDIBatang', weights: [400] },
    { label: '코펍월드바탕', value: 'KoPubWorldBatang', weights: [500, 700] },
    { label: '나눔명조', value: 'NanumMyeongjo', weights: [400, 700] },
  ],

  fontWeight: [
    { label: '가장 가늘게', value: 100 },
    { label: '아주 가늘게', value: 200 },
    { label: '가늘게', value: 300 },
    { label: '보통', value: 400 },
    { label: '중간', value: 500 },
    { label: '약간 굵게', value: 600 },
    { label: '굵게', value: 700 },
    { label: '아주 굵게', value: 800 },
    { label: '가장 굵게', value: 900 },
  ],

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

  textColor: [
    { label: '블랙', value: 'black', hex: '#18181b', color: token('colors.prosemirror.black') },
    { label: '그레이', value: 'gray', hex: '#71717a', color: token('colors.prosemirror.gray') },
    { label: '화이트', value: 'white', hex: '#ffffff', color: token('colors.prosemirror.white') },
    { label: '레드', value: 'red', hex: '#ef4444', color: token('colors.prosemirror.red') },
    { label: '오렌지', value: 'orange', hex: '#f97316', color: token('colors.prosemirror.orange') },
    { label: '앰버', value: 'amber', hex: '#f59e0b', color: token('colors.prosemirror.amber') },
    { label: '옐로', value: 'yellow', hex: '#eab308', color: token('colors.prosemirror.yellow') },
    { label: '라임', value: 'lime', hex: '#84cc16', color: token('colors.prosemirror.lime') },
    { label: '그린', value: 'green', hex: '#22c55e', color: token('colors.prosemirror.green') },
    { label: '에메랄드', value: 'emerald', hex: '#10b981', color: token('colors.prosemirror.emerald') },
    { label: '틸', value: 'teal', hex: '#14b8a6', color: token('colors.prosemirror.teal') },
    { label: '시안', value: 'cyan', hex: '#06b6d4', color: token('colors.prosemirror.cyan') },
    { label: '스카이', value: 'sky', hex: '#0ea5e9', color: token('colors.prosemirror.sky') },
    { label: '블루', value: 'blue', hex: '#3b82f6', color: token('colors.prosemirror.blue') },
    { label: '인디고', value: 'indigo', hex: '#6366f1', color: token('colors.prosemirror.indigo') },
    { label: '바이올렛', value: 'violet', hex: '#8b5cf6', color: token('colors.prosemirror.violet') },
    { label: '퍼플', value: 'purple', hex: '#a855f7', color: token('colors.prosemirror.purple') },
    { label: '마젠타', value: 'fuchsia', hex: '#d946ef', color: token('colors.prosemirror.fuchsia') },
    { label: '핑크', value: 'pink', hex: '#ec4899', color: token('colors.prosemirror.pink') },
    { label: '로즈', value: 'rose', hex: '#f43f5e', color: token('colors.prosemirror.rose') },
  ],

  textBackgroundColor: [
    { label: '배경 없음', value: 'none', hex: '#ffffff', color: null },
    { label: '그레이', value: 'gray', hex: '#f1f1f2', color: token('colors.prosemirror.bg.gray') },
    { label: '레드', value: 'red', hex: '#fdebec', color: token('colors.prosemirror.bg.red') },
    { label: '오렌지', value: 'orange', hex: '#ffecd5', color: token('colors.prosemirror.bg.orange') },
    { label: '옐로', value: 'yellow', hex: '#fef3c7', color: token('colors.prosemirror.bg.yellow') },
    { label: '그린', value: 'green', hex: '#dff3e3', color: token('colors.prosemirror.bg.green') },
    { label: '블루', value: 'blue', hex: '#e7f3f8', color: token('colors.prosemirror.bg.blue') },
    { label: '퍼플', value: 'purple', hex: '#f0e7fe', color: token('colors.prosemirror.bg.purple') },
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

  callout: [
    { label: '정보', type: 'info' },
    { label: '성공', type: 'success' },
    { label: '경고', type: 'warning' },
    { label: '주의', type: 'danger' },
  ],

  paragraphIndent: [
    { label: '없음', value: 0 },
    { label: '0.5칸', value: 0.5 },
    { label: '1칸', value: 1 },
    { label: '2칸', value: 2 },
  ],

  maxWidth: [
    { label: '600px', value: 600 },
    { label: '800px', value: 800 },
    { label: '1000px', value: 1000 },
  ],

  blockGap: [
    { label: '없음', value: 0 },
    { label: '0.5줄', value: 0.5 },
    { label: '1줄', value: 1 },
    { label: '2줄', value: 2 },
  ],
} as const;

export const defaultValues = {
  fontFamily: 'Pretendard',
  fontWeight: 400,
  fontSize: 16,
  textColor: 'black',
  textBackgroundColor: 'none',
  lineHeight: 1.6,
  letterSpacing: 0,
  textAlign: 'left',
  blockquote: 'left-line',
  horizontalRule: 'light-line',
  callout: 'info',
  paragraphIndent: 1,
  maxWidth: 800,
  blockGap: 1,
} as const;
