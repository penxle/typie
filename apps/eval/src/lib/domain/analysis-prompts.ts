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
export type AnalysisPromptContent = {
  survey: AnalysisStagePrompt;
  review: AnalysisStagePrompt;
  dedupe: AnalysisStagePrompt;
  verify: AnalysisStagePrompt;
  compose: AnalysisStagePrompt;
  composeReview: AnalysisStagePrompt;
};

export const ANALYSIS_STAGES: (keyof AnalysisPromptContent)[] = ['survey', 'review', 'dedupe', 'verify', 'compose', 'composeReview'];
