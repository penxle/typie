import { TRIAXIAL } from './evaluations/triaxial.ts';
import type { GenerationManifest } from '../../core/contracts.ts';

// 동결 세대 — 더 이상 실행하지 않는다(run/ 없음). 이 선언은 과거 데이터의 비용표 행 이름,
// 항목 종류 라벨, 평가 정의를 위해서만 존재한다.
//
// phases는 구 실행이 남긴 stage 값과 일치해야 한다 — 그래야 이관 후 비용표가 그려진다.
export const ANALYSIS_MANIFEST: GenerationManifest = {
  id: 'analysis',
  label: '분석 (동결)',
  status: 'frozen',
  phases: [
    { key: 'survey', label: '작품 파악' },
    { key: 'background', label: '배경 조사' },
    { key: 'genre', label: '장르 관습' },
    { key: 'plan', label: '비평 계획' },
    { key: 'planReview', label: '계획 검수' },
    { key: 'review', label: '짚을 곳 찾기' },
    { key: 'dedupe', label: '중복 묶기' },
    { key: 'verify', label: '근거 확인' },
    { key: 'compose', label: '피드백 다듬기' },
    { key: 'composeReview', label: '총평 작성' },
    { key: 'selfcheck', label: '자체 점검' },
  ],
  itemKinds: [
    { key: 'characterization', label: '작품 파악' },
    { key: 'finding', label: '지적' },
    { key: 'strength', label: '강점' },
    { key: 'pattern', label: '반복되는 무늬' },
    { key: 'priority', label: '먼저 손댈 것' },
  ],
  facets: [
    { key: 'category', label: '분류', groupBy: true },
    { key: 'theme', label: '주제' },
  ],
  evaluations: [TRIAXIAL],
  // 이 세대는 단계 산출물을 남기지 않는다.
  artifacts: null,
};
