import { UI as analysis } from '../../generations/analysis/ui/index.ts';
import { UI as editorial } from '../../generations/editorial/ui/index.ts';
import type { Component, Snippet } from 'svelte';
import type { RoundView } from './server/round-view.ts';
import type { ViewItem } from './server/run-view.ts';

type Answer = { value: Record<string, unknown>; onchange: (next: Record<string, unknown>) => void; readOnly?: boolean };

// 세대 렌더러의 계약. 어느 한 세대를 기준으로 타입을 뜨면 그 세대를 지우는 순간 나머지가
// 기준을 잃는다 — 계약은 세대 밖에 둔다.
// bind:this로 꺼내 쓰는 조작 창구. 세대는 이 둘을 반드시 내보내야 한다.
export type GenerationViewHandle = { focus: (itemId: string) => void; toggleTab: () => void };

// 슬롯 집합은 단계 수와 무관하게 동일하다 — "지금 몇 단계인가"는 stageKey로 흐르고,
// 각 단계에 무엇을 그리고 무엇을 잠글지는 세대가 자기 평가 선언을 보고 정한다.
// stageKey가 null이면 판정 없는 열람이다. artifacts도 값만 흐른다 — 노출 여부·시점은 세대가 정한다.
export type GenerationUi = {
  GenerationView: Component<
    {
      items: ViewItem[];
      numbers: Record<string, number>;
      stageKey: string | null;
      artifacts: { label: string; value: unknown } | null;
      focusedId?: string | null;
      onHover?: (itemId: string | null) => void;
      onSelect?: (itemId: string, anchorIndex: number) => void;
      onReveal?: (itemId: string) => void;
      control?: Snippet<[ViewItem]>;
      runReview?: Snippet;
    },
    GenerationViewHandle
  >;
  // 항목마다 걸 문항이 다르다 — 어느 항목의 판정인지는 세대가 item을 보고 정한다.
  ItemControl: Component<Answer & { item: ViewItem; stageKey: string | null }>;
  RunControl: Component<Answer & { stageKey: string | null }>;
  RunReview: Component<Answer & { stageKey: string | null }>;
  Summary: Component<{ view: RoundView }>;
};

// 세대 추가는 디렉토리 하나 + 여기 한 줄. 워커 번들에 Svelte가 새지 않도록 코어 레지스트리와
// 분리해 둔다.
export const GENERATION_UI: Record<string, GenerationUi> = {
  editorial,
  analysis,
};

export const generationUi = (generationId: string | null): GenerationUi | null =>
  generationId && Object.hasOwn(GENERATION_UI, generationId) ? GENERATION_UI[generationId] : null;
