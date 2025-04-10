import IconAlignCenter from '~icons/lucide/align-center';
import IconAlignJustify from '~icons/lucide/align-justify';
import IconAlignLeft from '~icons/lucide/align-left';
import IconAlignRight from '~icons/lucide/align-right';
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
  fontFamily: [
    { label: '프리텐다드', value: 'Pretendard' },
    { label: '코펍월드돋움', value: 'KoPubWorldDotum' },
    { label: '나눔바른고딕', value: 'NanumBarunGothic' },
    { label: '리디바탕', value: 'RIDIBatang' },
    { label: '코펍월드바탕', value: 'KoPubWorldBatang' },
    { label: '나눔명조', value: 'NanumMyeongjo' },
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
    { label: '블랙', value: 'black', hex: '#09090b' },
    { label: '그레이', value: 'gray', hex: '#71717a' },
    { label: '화이트', value: 'white', hex: '#ffffff' },
    { label: '레드', value: 'red', hex: '#ef4444' },
    { label: '오렌지', value: 'orange', hex: '#f97316' },
    { label: '앰버', value: 'amber', hex: '#f59e0b' },
    { label: '옐로', value: 'yellow', hex: '#eab308' },
    { label: '라임', value: 'lime', hex: '#84cc16' },
    { label: '그린', value: 'green', hex: '#22c55e' },
    { label: '에메랄드', value: 'emerald', hex: '#10b981' },
    { label: '틸', value: 'teal', hex: '#14b8a6' },
    { label: '시안', value: 'cyan', hex: '#06b6d4' },
    { label: '스카이', value: 'sky', hex: '#0ea5e9' },
    { label: '블루', value: 'blue', hex: '#3b82f6' },
    { label: '인디고', value: 'indigo', hex: '#6366f1' },
    { label: '바이올렛', value: 'violet', hex: '#8b5cf6' },
    { label: '퍼플', value: 'purple', hex: '#a855f7' },
    { label: '마젠타', value: 'fuchsia', hex: '#d946ef' },
    { label: '핑크', value: 'pink', hex: '#ec4899' },
    { label: '로즈', value: 'rose', hex: '#f43f5e' },
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
  fontSize: 16,
  textColor: 'black',
  lineHeight: 1.6,
  letterSpacing: 0,
  textAlign: 'left',
  blockquote: 'blockquote_1',
  horizontalRule: 'horizontal_rule_1',
  callout: 'info',
  paragraphIndent: 1,
  maxWidth: 800,
  blockGap: 1,
} as const;
