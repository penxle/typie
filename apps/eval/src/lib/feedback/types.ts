// prism 계약 미러 — 원본: prism apps/feedback/result.ts의 FeedbackResult, core/terminal.ts의 foldUsage,
// core/do.ts의 WorkflowView. prism 쪽 개정 시 이 파일을 함께 갱신한다.
// 재리뷰 계약(ThreadDisposition·dispositions·conclusion.progress)의 원본도 apps/feedback/result.ts이며,
// verdict 값은 apps/feedback/stages/disposition.ts의 Verdict다.

import type { AskAnswer, AskQuestion } from './live.ts';

export type Anchor = { start: number; end: number; head: string; tail: string };

// thread는 지난 회차에서 계속되는(kept) 지적의 스레드 신원이다 — 사영은 이 표지의 이슈로 새 스레드를 만들지
// 않고, 기존 스레드를 이번 회차 번호 공간에 다시 앉힌다.
export type Pass = 'judgment' | 'stylistic';

// id는 이슈의 신원이다 — 배열 위치와 달리 조립이 바뀌어도 흔들리지 않아 총평이 이것으로 지적을 가리킨다.
// 지적을 id로 다루지 않는 티어(medium·low)와 id 도입 전에 굳은 결과에는 자리 자체가 없다.
export type FeedbackIssue = { id?: string; trait: string; pass: Pass; body: string | null; anchors: Anchor[]; thread?: string };

export type FeedbackConclusion = {
  understanding: string | null;
  // 진전 서술은 재리뷰 회차에만 있다 — 1회차 결과에는 키 자체가 서지 않는다.
  progress?: string | null;
  strengths: (Anchor & { body: string | null })[];
  // 무혐의는 판정점이 없는 티어의 것이다 — 특질마다 판정이 서는 티어에서는 verdicts가 그 자리를 대신한다.
  clearances?: { trait: string; note: string }[];
  // 참조의 형태는 티어마다 갈린다 — 지적을 번호로 세는 티어는 번호를, id로 다루는 티어는 이슈의 id를 싣는다.
  // 저장된 구 결과는 영구히 번호다(굳은 JSON을 다시 쓰지 않는다).
  patterns: { theme: string | null; body: string; issues: number[] | string[] }[];
  priorities: { body: string; issues: number[] | string[] }[];
};

// 지난 회차 스레드의 처분 — 판정(verdict)은 작품 검토·교열이, 작가 대면 문면(comment)은 rephrase가 만든다.
// comment가 null이면 생략된 선택 코멘트다(새 답글 없이 이어지는 kept) — 사영은 댓글을 게시하지 않는다.
// 이어지는 지적의 새 자리는 여기 서지 않는다 — 승계 이슈(issues[].anchors)가 그 정본이고 재앉힘도 그쪽만 본다.
export type ThreadDisposition = {
  threadId: string;
  verdict: 'resolved' | 'kept' | 'withdrawn';
  comment: string | null;
};

export type FeedbackResult = {
  version: 1;
  // 결과 판별자 — classify 편입(prism 2026-08-15) 이후의 결과에만 실린다. 구 저장 결과에는 키 자체가
  // 없으므로 거부 판별은 kind === 'rejected'로만 한다(부재 = 정상 결과, 키 존재 검사 금지).
  kind?: 'feedback' | 'issues';
  issues: FeedbackIssue[];
  // low 티어 결과에는 키 자체가 없다 — 마무리 글을 쓰는 에이전트가 그 티어에 없다.
  conclusion?: FeedbackConclusion;
  // 재리뷰 회차에만 키가 선다 — 승계할 스레드가 없어 처분이 비어도 빈 배열로 서므로, 1회차(키 부재)와 구분된다.
  dispositions?: ThreadDisposition[];
  // 특질별 작품 수준 판정(1=특질 부재·실패 ~ 4=탁월) — 기준표를 세우는 티어에만 키가 선다.
  verdicts?: { trait: string; point: number; note: string | null }[];
  // 격상 — 3점(일관 성립) 특질에서 관측된 4점 미달 지점. 결함이 아니라 다음 단계의 제안이라 issues와 목록이 다르다.
  elevations?: { trait: string; body: string | null; anchors: Anchor[] }[];
  // 판정이 보았으나 작가에게 가지 않은 관측 — 운영자 화면에만 세운다. 작가 대면 표면에 실으면 안 된다.
  diagnostics?: FeedbackDiagnostics;
};

export type FeedbackDiagnostics = {
  // 반려 라운드에 제출됐다가 접수본에 없는 지적의 특질별 건수 — 고치는 대신 지워서 통과한 흔적이다.
  dropped: { trait: string; count: number }[];
  // 기준표가 덮지 못해 지적이 될 자리가 없었던 결함 후보 — 기준표가 좁았다는 증거다.
  gaps: (string | null)[];
  // 걸렸으나 지적으로 서지 않은 독서 기록 항목 — 승격분은 지적이 이미 담으므로 여기 없다.
  withheld: {
    kind: string;
    head: string;
    reading: string | null;
    disposition: 'explained' | 'withheld';
    note: string | null;
  }[];
};

// 재리뷰 시작 입력 — prism apps/feedback/followup.ts의 PREVIOUS 스키마 미러. 스레드는 상태 불문 전건이 실리고,
// 상태별 역할(처분 대상·억제)은 prism이 가른다. fresh는 시각 판별이 끝난 표지다 — prism은 시각을 모르므로
// "지난 리뷰가 응답하지 않은 새 답글"의 판별은 eval이 마치고 boolean만 넘긴다.
export type PreviousThread = {
  id: string;
  pass: Pass;
  trait: string;
  body: string;
  anchors: { head: string; tail: string }[];
  replies: { body: string; fresh: boolean }[];
  state: 'open' | 'closed' | 'resolved' | 'withdrawn';
  issue?: string;
};

export type ManuscriptMeta = { title: string | null; subtitle: string | null };

export type PreviousInput = { manuscriptPath: string; meta: ManuscriptMeta; threads: PreviousThread[] };

export type UsageFold = {
  provider: string;
  agent: string;
  model: string;
  effort: string | null;
  turns: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  thinkingTokens: number | null;
};

export type RunUsage = { complete: boolean; folds: UsageFold[] };

// prism 와이어 — settled가 판별자다. DB(Reviews.usage)에는 판별자를 벗긴 RunUsage만 남는다.
export type WorkflowUsage = ({ settled: true } & RunUsage) | { settled: false; folds: UsageFold[] };

// 거부 종결 — 원본: prism apps/feedback/stages/classify.ts. 워크플로는 정상 완료(status: completed)하고
// 결과만 이 형태다. message는 prism이 확정한 작가 대면 고정 문면이고, basis는 운영 진단용이라 작가
// 표면에 싣지 않는다.
export type RejectionCategory = 'diary' | 'practical' | 'non-text' | 'non-korean' | 'unprocessable' | 'unclassifiable';

export type FeedbackRejection = {
  version: 1;
  kind: 'rejected';
  rejected: { category: RejectionCategory; message: string; basis: string | null };
};

export type FeedbackOutcome = FeedbackResult | FeedbackRejection;

export type PrismWorkflow = {
  status: 'running' | 'completed' | 'failed' | 'canceled';
  result: FeedbackOutcome | null;
  error: string | null;
  usage: WorkflowUsage | null;
  startedAt: number;
  finishedAt: number | null;
};

export type PrismWorkflowView = { workflow: PrismWorkflow };

// 사영이 굳히는 질문 기록 — 카드 자체는 이벤트 재생이 세우므로, 굳이 따로 남기는 이유는 재생으로 복원할 수 없는
// 답변 문면 하나다(이벤트에 없고 prism 원장에만 있다). 나머지 필드는 그 문면의 귀속처이자 기록의 자족성이다.
export type ReviewQuestionRecord = {
  agentName: string;
  toolCallId: string;
  stage: string | null;
  at: number | null;
  status: 'answered' | 'closed';
  questions: AskQuestion[];
  answers: AskAnswer[] | null;
};
