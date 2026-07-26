import { z } from 'zod';

export const corpusRoundPayloadSchema = z.discriminatedUnion('stage', [
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('screening'),
    corpusVersion: z.string().min(1),
    variantLabels: z.array(z.string().min(1)).min(2),
    baselineLabel: z.string().min(1),
    overlapRatio: z.number().min(0).max(1).default(0.2),
    expectedEvaluators: z.number().int().min(1).optional(),
  }),
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('confirmation'),
    corpusVersion: z.string().min(1),
    v0Label: z.string().min(1),
    candidateLabel: z.string().min(1),
    documentIds: z.array(z.string().min(1)).optional(),
  }),
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('absolute'),
    corpusVersion: z.string().min(1),
    label: z.string().min(1),
    requiredJudgments: z.number().int().min(1).default(2),
    // 이 비율만큼의 문서는 상한 없이 열어 전원이 본다 — 판정자 간 일치도를 재는 구간.
    overlapRatio: z.number().min(0).max(1).default(0),
    expectedEvaluators: z.number().int().min(1).optional(),
    documentIds: z.array(z.string().min(1)).optional(),
  }),
]);

export type CorpusRoundPayload = z.infer<typeof corpusRoundPayloadSchema>;
