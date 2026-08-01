// Editorial 파이프라인(RESEARCH→PLAN→EXECUTE→COMPOSE)의 단계 간 계약.
// 스펙: docs/superpowers/specs/2026-07-28-editorial-pipeline-design.md

export type Range = { start: number; end: number };

// RESEARCH 산출 — 스펙 §5의 여섯 블록.
export type Research = {
  nature: {
    form: string;
    completeness: { level: 'draft' | 'in-revision' | 'complete' | 'undetermined'; note: string };
    feedbackFit: string;
  };
  voice: { pov: string; conventions: { pattern: string; evidence: string[] }[] };
  names: { name: string; aliases: string[]; note: string }[];
  premise: {
    sourceWork: { status: 'identified' | 'not-identified' | 'undetermined'; name: string; brief: string };
    genreConventions: string;
    seriesContext: string;
  };
  boundaries: { startQuote: string; endQuote: string; reason: string }[];
  unverified: string[];
};

// 코드가 boundaries의 인용을 좌표로 해석해 붙인 결과.
export type ResolvedResearch = Research & { boundaryRanges: Range[] };

// PLAN 산출 — 스펙 §6의 다섯 블록.
export type EditorialPlan = {
  intent: string;
  protected: { technique: string; evidence: string[]; rationale: string }[];
  axes: {
    label: string;
    inquiry: string;
    risk: string;
    readerCost: string;
    expectedFinding: string;
    evidence: string[];
    conventionsCheck: string;
    conventionsBasis: 'charter' | 'search' | 'unrelated' | 'unresolved';
  }[];
  verifications: { question: string; tools: ('search' | 'grep' | 'read')[]; detail: string; conclusion: string }[];
  reviewResponses: { target: string; disposition: 'adopted' | 'partial' | 'rejected'; reason: string }[];
};

export type PlanReviewFinding = {
  kind: string;
  target: string;
  rationale: string;
  conventionsCheck: string;
  fix: string;
  // 이대로 실행되면 검토가 오염되는가 — 수렴 판정은 이 자기 신고에서 기계 유도된다.
  blocking: boolean;
};
export type PlanReview = { verdict: 'approve' | 'needs-attention'; findings: PlanReviewFinding[] };

// EXECUTE의 건별 제출 — 스펙 §7. manuscriptCheck(원고 대조)와 conventionsCheck(규약 대조)는
// 무조건 서술 필드이고, 기계 판독 분류(manuscriptBasis)의 grep 신고는 원장 대조로 집행된다.
export type EditorialFinding = {
  axis: string;
  quoteStart: string;
  quoteEnd: string;
  intent: string;
  observation: string;
  cause: string;
  direction: string;
  evidence: string;
  stake: string;
  manuscriptBasis: 'grep' | 'reread' | 'local';
  manuscriptCheck: string;
  conventionsCheck: string;
};

export type AcceptedFinding = EditorialFinding & { matchStart: number; matchEnd: number; filedAtTurn: number };

export type EditorialStrength = { quoteStart: string; quoteEnd: string; principle: string };
export type AcceptedStrength = EditorialStrength & { matchStart: number; matchEnd: number };
