// 재설계된 분석 파이프라인(SURVEY → REVIEW → VERIFY → COMPOSE)의 단계 간 계약.
// 단계 사이에는 구조화된 데이터만 흐른다 — 산문 요약이 흐르던 구 파이프라인에서
// 상위 모델이 부정확한 요약과 본문을 대조하다 오판하던 경로를 없애기 위해서다.

export type WorkProfile = {
  form: string;
  isDerivative: boolean;
  pov: string;
  // 화자가 사건을 객관적으로 전달하는지, 주관적 왜곡이 있는지. 구 META의 reliability에 해당한다.
  reliability: string;
  tense: string;
  dialogueConvention: string;
  // 두 곳 이상에서 반복 확인된 패턴만 등재한다 — 한 번 나타난 특이 표현은 문체가 아니라 실수일 수 있다.
  deliberateStyles: { pattern: string; evidence: string }[];
  properNouns: string[];
  nonAnalyticRanges: { start: number; end: number; reason: string }[];
};

// boundaryQuality는 이 장면이 끝나는 지점에서 창을 닫아도 되는지를 뜻한다.
export type Scene = {
  start: number;
  end: number;
  gist: string;
  characters: string[];
  setting: string;
  pov: string;
  // 회상·삽입 장면 여부와 현재 시점 복귀 지점. 구 SUMMARIZE의 transitions에 해당하며,
  // 검토자가 시간 이동을 결함으로 오인하지 않게 한다.
  flashback: string;
  boundaryQuality: 'clean' | 'weak' | 'none';
};

export type FindingKind = 'error' | 'readability' | 'structure';

// 비평 계획. 검토는 axes로만 이루어지고 protected는 지적 대상에서 제외된다 —
// 문서 수준 판단(총평에서 거의 만점)을 생성 앞으로 옮기는 장치다.
//
// evidence는 원고에서 글자 그대로 복사한 인용 배열이다. 앵커와 같은 취급 — 코드가
// 원고와 대조하고, 찾지 못한 인용은 그 자리에서 삭제된다(plan-check).
export type Plan = {
  intent: string;
  protected: { technique: string; evidence: string[]; rationale: string }[];
  // 검수 발견을 반영하지 않았다면 그 사유. 침묵 기각을 막고, 다음 라운드 검수가
  // 재론 여부를 정할 수 있게 한다.
  rejectedFindings: { target: string; reason: string }[];
  axes: { label: string; description: string; risk: string; evidence: string[] }[];
};

// observation → cause → direction 3분할이 오라클의 상을 스키마로 강제한다.
// direction을 별도 필드로 두면 대안 문장 대필이 구조적으로 억제된다.
export type Finding = {
  quoteStart: string;
  quoteEnd: string;
  matchStart: number | null;
  matchEnd: number | null;
  kind: FindingKind;
  // 읽다가 멈춘 자리. 이 단계의 기준이 "실제로 멈춘 곳만 적으라"이므로, 그 주장을 말이 아니라
  // 좌표로 받는다. 조건 없이 모든 지적에 요구한다 — 면제되는 갈래를 하나라도 두면 그리로 몰린다.
  stumbleQuote: string;
  stumbleStart: number | null;
  stumbleEnd: number | null;
  // 이 대목이 하려는 일을 작가의 편에서 읽어낸 것. 라운드 2 라벨 데이터에서 의도를 먼저 읽은
  // 지적이 '핵심 지적'·'즉시 적용 가능'으로 평가받았고, 의도를 건너뛴 지적이 '의도 무시'·'스타일 강요'로 찍혔다.
  intent: string;
  observation: string;
  cause: string;
  direction: string;
  evidence: string;
  // 계획 귀속 검토에서만 존재한다 — 이 지적이 비평 계획의 어느 축을 위한 것인가.
  axis?: string;
  // 몇 번째 REVIEW 실행에서 나왔는가. 같은 문제를 묶는 일은 COMPOSE가 하며,
  // 한 묶음에 서로 다른 실행이 몇 개나 들어왔는지가 그 지적의 신뢰도가 된다.
  runIndex: number;
};

export type Verdict = {
  keep: boolean;
  ground: 'valid' | 'evidence-missing' | 'deliberate-style' | 'out-of-scope';
  reason: string;
};

export type FeedbackAnchor = {
  quoteStart: string;
  quoteEnd: string;
  matchStart: number | null;
  matchEnd: number | null;
  note?: string;
};

// anchors가 비어 있으면 특정 위치에 붙지 않는 지적이다.
// polarity는 남기되 값은 항상 issue다 — 강점은 이 경로를 타지 않는다.
export type Feedback = {
  category: string;
  polarity: 'issue';
  body: string;
  anchors: FeedbackAnchor[];
};

// 짚을 곳과 달리 중복 묶기·피드백 쓰기를 거치지 않고 총평으로 바로 간다.
export type Strength = {
  quoteStart: string;
  quoteEnd: string;
  principle: string;
  matchStart: number | null;
  matchEnd: number | null;
};

export type WorkReview = {
  characterization: string;
  strengths: { body: string; quoteStart: string; quoteEnd: string }[];
  patterns: { theme: string; body: string }[];
  priority: string;
};

export type AnalysisResult = {
  feedbacks: Feedback[];
  review: WorkReview;
};

// 분석 창. head/tail은 읽기용 원문이며 앵커를 달 수 없다 — 모델이 문장의 완결을
// 직접 확인하게 해서 "미완성 문장" 오독의 근거 자체를 없앤다.
export type Window = {
  index: number;
  start: number;
  end: number;
  text: string;
  head: string;
  tail: string;
  sceneCount: number;
  forced: boolean;
};
