import type { FeedbackResult } from './types.ts';

export type Verdict = NonNullable<FeedbackResult['verdicts']>[number];

// 판정점의 작가 대면 번역 — prism 기준표의 정의(1=부재·실패 / 2=존재하나 불안정 / 3=일관 성립 / 4=탁월)를
// 작가가 읽을 낱말로 옮긴다. 숫자는 화면에 세우지 않는다(오너 결정 2026-08-12: 성적표로 읽히지 않게).
const LABELS: Record<number, string> = { 1: '아직', 2: '흔들림', 3: '자리 잡음', 4: '단단함' };

export const VERDICT_POINTS = [1, 2, 3, 4] as const;

export const verdictLabel = (point: number): string | null => LABELS[point] ?? null;
