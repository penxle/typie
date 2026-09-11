import { clamp } from '@typie/ui/utils';
import type { Card, Lead } from './features';

export type SceneProps = { progress: number; cardsProgress?: number; lead: Lead; cards: readonly Card[] };

export const SCENE_SLIDE_RANGE: readonly [number, number] = [0.76, 0.88];

export const SCENE_LEAD_RANGE: readonly [number, number] = [0.84, 0.94];

// 작성 씬은 시각물이 자리를 비키는 대신 기기가 접힌다 — 접히자마자 리드가 들어서야 한다
export const WRITE_LEAD_RANGE: readonly [number, number] = [0.62, 0.72];

// 한 걸음 안에서 카드 네 장을 차례로 보여준다.
export const CARD_STEP = 0.25;
export const CARD_SPAN = 0.25;

// 씬 애니메이션이 섹션 진행도 0~SCENE_SPAN, 카드가 그 뒤를 쓴다.
// 섹션 440dvh 기준 씬 240dvh·카드 200dvh — 카드 한 장에 50dvh(전환 20 + 머무름 30)를 준다.
export const SCENE_SPAN = 240 / 440;

export const SECTION_DVH = 440;
export const STAGE_DVH = 100;

export const mobileSectionDvh = (end: number): number => Math.round(STAGE_DVH + (SECTION_DVH - STAGE_DVH) * SCENE_SPAN * end);

export const HIDDEN_BELOW = 0.02;

export const ramp = (value: number, from: number, to: number): number => clamp((value - from) / (to - from), 0, 1);

export const rise = (progress: number, distance: number, unit: 'px' | 'em' = 'px'): string =>
  `translateY(${(1 - progress) * distance}${unit})`;
