export type PhasePrompt = { system: string; model: string; effort: string | null };

export type PhaseSpec = { key: string; label: string; prompt?: false };
export type FacetSpec = { key: string; label: string; groupBy?: boolean };
export type ItemKindSpec = { key: string; label: string };

// 필드는 자기를 설명한다. 코어는 종류를 모르고 셋만 쓴다 — 완결 조건에 드는가(required),
// 어떻게 걸러내는가(sanitize), 무엇을 그려야 하는가(render, 세대만 해석).
//
// 종류를 코어 유니온으로 두면 새 위젯(다중 선택·순위 등)마다 코어 두 곳과 세대 렌더러를
// 함께 고쳐야 해서 "세대 추가 = 디렉토리 하나"가 깨진다.
export type FieldSpec = {
  key: string;
  required: boolean;
  // 답이 없으면 null. 이 판정이 곧 완결 조건이라 서버와 화면이 어긋날 수 없다.
  sanitize: (raw: unknown) => unknown;
  render: unknown;
};

export type ItemMatchTarget = { kind: string; facets: Record<string, string> };

export type ItemTargetSpec = {
  match: (item: ItemMatchTarget) => boolean;
  fields: FieldSpec[];
};

// 평가의 한 단계. 단계들은 순서대로 확정되고, 확정된 단계의 답은 고칠 수 없다.
export type EvaluationStage = {
  key: string;
  // 공용 화면이 문구를 조립할 재료 — 세대의 어휘는 선언 안에만 산다.
  label: string;
  run: FieldSpec[];
  items: ItemTargetSpec[];
};

export type EvaluationSpec = {
  id: string;
  label: string;
  // 모든 단계가 한 페이로드를 공유하므로 필드 키는 전 단계에 걸쳐 유일해야 한다.
  // judgments.stage가 확정된 단계 수이고, 마지막 단계 제출이 곧 판정 확정이다.
  stages: EvaluationStage[];
};

// phase는 프롬프트 키이자 진행 표시 단위이자 비용표의 행이다. 셋이 갈려 있으면 같은 문자열이
// 서로 다른 뜻으로 쓰여 회계가 어긋난다.
// 단계 산출물 열람. 어떤 원장을 읽는지도, 무엇이라 부르는지도, 무엇으로 접는지도 세대가 정한다 —
// 공용 화면은 이 선언만 보고 버튼을 걸므로, 모듈을 지우면 버튼도 함께 사라진다.
export type ArtifactSpec = {
  label: string;
  ledgerKeys: string[];
  // 원장 행(key → value)을 렌더러가 받을 값으로 접는다. 접을 수 없으면 null — 화면은 버튼을 감춘다.
  select: (rows: Record<string, unknown>) => unknown | null;
};

export type GenerationManifest = {
  id: string;
  label: string;
  status: 'active' | 'frozen';
  phases: PhaseSpec[];
  itemKinds: ItemKindSpec[];
  facets: FacetSpec[];
  evaluations: EvaluationSpec[];
  artifacts: ArtifactSpec | null;
};

// cachedTokens와 cacheWriteTokens는 promptTokens에 포함된 값이다(별도 합이 아니다).
// 캐시 읽기는 입력 단가의 10%, 쓰기는 1.25배라 나눠 세지 않으면 캐싱의 손익을 못 낸다.
export type Usage = {
  calls: number;
  promptTokens: number;
  completionTokens: number;
  cachedTokens: number;
  cacheWriteTokens: number;
};

export const emptyUsage = (): Usage => ({
  calls: 0,
  promptTokens: 0,
  completionTokens: 0,
  cachedTokens: 0,
  cacheWriteTokens: 0,
});

// 실행 중 도구 호출 기록. 열람 범위·커버리지 판정의 진실 원천이며, 워크플로 리플레이 때
// 캐시된 도구 실행 결과에서 재구성되므로 순수한 값이어야 한다.
export type ToolRecord =
  | { turn: number; tool: 'read'; start: number; end: number }
  | { turn: number; tool: 'grep'; pattern: string; total: number }
  | { turn: number; tool: 'search'; query: string; hits: number };

export type AnchorDraft = {
  quoteStart: string;
  quoteEnd: string;
  matchStart: number | null;
  matchEnd: number | null;
  note?: string;
};

// links는 같은 items 배열 안의 인덱스다. 코어가 저장할 때 항목 id로 해석한다.
export type ItemDraft = {
  kind: string;
  ord: number;
  body: string;
  facets: Record<string, string>;
  anchors: AnchorDraft[];
  links: number[];
};
