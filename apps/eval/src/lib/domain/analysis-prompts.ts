// 재설계 파이프라인의 프롬프트 묶음 타입. 기존 3단계 구조(StageKey)와 분리해 둔다 —
// 그쪽은 프로덕션 프롬프트 적용 경로에 묶여 있어 함께 확장하면 파급이 크다.

export type AnalysisStageKey = 'survey' | 'review' | 'dedupe' | 'verify' | 'compose';

export type AnalysisStagePrompt = {
  system: string;
  model: string;
  effort: string | null;
  temperature?: number | null;
};

// compose는 피드백 편성과 총평 작성 두 번을 도므로 프롬프트도 둘이다.
//
// background와 verify는 선택이다 — 그 프롬프트가 없는 세트는 해당 단계를 아예 건너뛴다.
// 기존 세트를 그대로 두고 켠/끈 세트를 따로 만들어 대조하기 위해서다.
//
// verify를 끄면 근거 확인이 피드백 쓰기 안으로 들어간다. 검증은 지적마다 원문을 다시
// 보내느라 비용의 39%를 쓰는데 실제로 걷어내는 건 8%였고, 피드백 쓰기는 어차피 한 번은
// 도는 단계라 원문을 한 번만 실으면 같은 확인을 훨씬 싸게 할 수 있다.
export type AnalysisPromptContent = {
  survey: AnalysisStagePrompt;
  review: AnalysisStagePrompt;
  dedupe: AnalysisStagePrompt;
  compose: AnalysisStagePrompt;
  composeReview: AnalysisStagePrompt;
  background?: AnalysisStagePrompt;
  // 장르 변형(세계관 AU)의 문법 정리. background가 변형을 감지했을 때만 쓰인다.
  genre?: AnalysisStagePrompt;
  verify?: AnalysisStagePrompt;
  // 완성된 피드백이 원고에 근거를 두는지 확인한다. 없으면 그 확인 없이 내보낸다.
  selfcheck?: AnalysisStagePrompt;
  // 문서 수준 비평 계획. 있으면 검토가 계획의 축에 귀속된다.
  plan?: AnalysisStagePrompt;
  // 계획의 교차 벤더 검수. plan이 있을 때만 의미가 있다.
  planReview?: AnalysisStagePrompt;
  // Editorial 파이프라인(EditorialWorkflow) 전용 단계들.
  research?: AnalysisStagePrompt;
  execute?: AnalysisStagePrompt;
  // 문면 층위(문장 결·원고 사고) 단독 소유 스테이지.
  local?: AnalysisStagePrompt;
  // 사후 판정 워크플로(JudgeWorkflow) 전용 — 분석 파이프라인은 읽지 않는다.
  defense?: AnalysisStagePrompt;
  // 교차 벤더 심판(RulingWorkflow) 전용.
  ruling?: AnalysisStagePrompt;
};

export const ANALYSIS_STAGES: (keyof AnalysisPromptContent)[] = ['survey', 'review', 'dedupe', 'compose', 'composeReview'];
