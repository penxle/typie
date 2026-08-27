import { resolveAnchors } from '@typie/prism';
import { normalizeStableSelection } from './comment-selection.ts';
import { assembleOutcomeAnchors, outcomeAnchorSites, unresolvedOutcomeAnchors } from './prism-review-core.ts';
import type { ProseAnchorCapture, ProseRange, StableSelection } from '@typie/editor-ffi/server';
import type { ReviewOutcome } from '@typie/prism';
import type { AnchorHit, OutcomeAnchors } from './prism-review-core.ts';

export type AnchorCaptureFailure = { kind: 'text_mismatch' } | { kind: 'capture_failed'; error: unknown };

// 외부 효과는 전부 여기로 들어온다 — 사영 경로가 실제 배선을, 테스트가 가짜를 준다
export type AnchorCaptureDeps = {
  readGraph: () => Promise<Uint8Array>;
  capture: (graph: Uint8Array, heads: Uint8Array, expectedText: string, ranges: ProseRange[]) => Promise<ProseAnchorCapture>;
  report: (failure: AnchorCaptureFailure) => void;
};

// 앵커 해석·캡처는 사영의 입력일 뿐 사영을 막지 않는다 — 어떤 실패도 "자리 없음"으로 접고 리뷰는 착지한다.
// 매칭은 스냅샷 텍스트(prism이 읽은 그 파일)에서, 캡처는 스냅샷 heads 시점 상태에서 한다.
export const resolveOutcomeAnchors = async (
  outcome: ReviewOutcome,
  version: { content: string; heads: Uint8Array },
  deps: AnchorCaptureDeps,
): Promise<OutcomeAnchors> => {
  const sites = outcomeAnchorSites(outcome);
  if (sites.length === 0) return unresolvedOutcomeAnchors(outcome);

  const ranges = resolveAnchors(
    version.content,
    sites.map((site) => site.anchor),
  );
  const targets = ranges.flatMap((range, site) => (range === null ? [] : [{ site, range }]));
  if (targets.length === 0) return unresolvedOutcomeAnchors(outcome);

  try {
    const graph = await deps.readGraph();
    const result = await deps.capture(
      graph,
      version.heads,
      version.content,
      targets.map((target) => target.range),
    );
    if (!result.text_matches) {
      deps.report({ kind: 'text_mismatch' });
      return unresolvedOutcomeAnchors(outcome);
    }

    const hits: (AnchorHit | null)[] = sites.map(() => null);
    for (const hit of result.anchors) {
      const target = targets[hit.index];
      if (target === undefined) continue;
      hits[target.site] = { selection: normalizeStableSelection(hit.selection) as StableSelection, text: hit.text };
    }

    return assembleOutcomeAnchors(outcome, sites, hits);
  } catch (err) {
    deps.report({ kind: 'capture_failed', error: err });
    return unresolvedOutcomeAnchors(outcome);
  }
};
