import { createStableContext } from '@typie/ui/context/stable';
import type { DataOf } from '@mearie/svelte';
import type { DocumentPrismReviewMargin_Round_Query } from '$mearie';
import type { DetailRound } from '../../../@prism/review/round-view.ts';
import type { MarginMode, RoundOption, ThreadCallouts } from './margin-view.ts';

type Round = DataOf<DocumentPrismReviewMargin_Round_Query>['prismReviewRound'];

export type MarginThread = Round['threads'][number];
export type MarginComment = MarginThread['comments'][number];

export type MarginStrength = { index: number; quote: string; body: string | null };
export type MarginActivation = { id: string | null; rangeId: string | null; sequence: number };

// 목록의 갈래 — 닫는다고 옮겨 가지 않는다. 'settled'로 옮겨 오는 유일한 경로는 재리뷰 사영이다.
export type MarginSegment = 'open' | 'settled' | 'lost';

export type MarginPlacement = {
  id: string;
  kind: 'issue' | 'strength';
  number: number;
  rangeIds: string[];
  callouts: ThreadCallouts;
  strengthIndex: number | null;
};

export type MarginItem = MarginPlacement & {
  anchored: boolean;
  thread: MarginThread | null;
  strength: MarginStrength | null;
};

export type MarginController = {
  readonly rounds: RoundOption[];
  readonly selectedRoundId: string | null;
  readonly detailRound: DetailRound | null;
  readonly items: MarginItem[];
  readonly segment: MarginSegment;
  readonly segmentCounts: Record<MarginSegment, number>;
  readonly segmentCards: MarginItem[];
  readonly activation: MarginActivation;
  readonly activeId: string | null;
  readonly mode: MarginMode;
  readonly ready: boolean;
  readonly presentationRoundId: string | null;
  readonly presentationProgress: number;
  readonly presentationInteractive: boolean;
  readonly myId: string;
  readonly locked: boolean;
  select: (roundId: string | null) => void;
  activate: (id: string | null) => void;
  markPresentationPrepared: (roundId: string) => void;
  setSegment: (next: MarginSegment) => void;
  reply: (threadId: string, body: string) => Promise<void>;
  editComment: (commentId: string, body: string) => Promise<void>;
  deleteComment: (commentId: string) => Promise<void>;
  close: (threadId: string) => Promise<void>;
  reopen: (threadId: string) => Promise<void>;
  react: (threadId: string, value: 'UP' | 'DOWN' | null) => Promise<void>;
};

const [getRequiredMarginContext, setMarginContext, getOptionalMarginContext] =
  createStableContext<MarginController>('prism-review.PrismReviewMargin');

export const setupMarginContext = (controller: MarginController) => setMarginContext(controller);

// 여백 안쪽 컴포넌트용 — 컨텍스트 없이 뜨는 일이 없으므로 없으면 결함이다
export const getMarginContext = (): MarginController => {
  return getRequiredMarginContext();
};

// 툴바용 — 미리보기·뷰어 에디터는 이 컨텍스트 없이 뜬다
export const tryMarginContext = (): MarginController | undefined => getOptionalMarginContext();
