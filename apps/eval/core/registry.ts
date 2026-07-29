import { ANALYSIS_MANIFEST } from '../generations/analysis/manifest.ts';
import { EDITORIAL_MANIFEST } from '../generations/editorial/manifest.ts';
import type { EvaluationSpec, GenerationManifest } from './contracts.ts';

// 세대 추가는 디렉토리 하나 + 여기 한 줄. 삭제는 그 반대. 코어의 다른 어디에도 세대 id가
// 등장해서는 안 된다.
export const GENERATIONS: GenerationManifest[] = [EDITORIAL_MANIFEST, ANALYSIS_MANIFEST];

export const generationById = (id: string): GenerationManifest | null => GENERATIONS.find((g) => g.id === id) ?? null;

export const qualifiedEvaluationId = (generationId: string, evaluationId: string): string => `${generationId}/${evaluationId}`;

export const evaluationById = (qualified: string): { generation: GenerationManifest; evaluation: EvaluationSpec } | null => {
  const [generationId, evaluationId] = qualified.split('/');
  if (!generationId || !evaluationId) return null;
  const generation = generationById(generationId);
  const evaluation = generation?.evaluations.find((e) => e.id === evaluationId);
  return generation && evaluation ? { generation, evaluation } : null;
};
