import { editorialRunner } from '../../generations/editorial/run/index.ts';
import type { GenerationRunner } from '../../core/worker/run-contracts.ts';

// 세대 추가는 디렉토리 하나 + 여기 한 줄. 동결 세대는 등록하지 않는다 — 실행이 없으므로.
export const RUNNERS: Record<string, GenerationRunner> = {
  editorial: editorialRunner,
};
