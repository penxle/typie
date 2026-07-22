import { z } from 'zod';

export const corpusRoundPayloadSchema = z.discriminatedUnion('stage', [
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('screening'),
    corpusVersion: z.string().min(1),
    variantLabels: z.array(z.string().min(1)).min(2),
    overlapRatio: z.number().min(0).max(1).default(0.2),
    sanityRatio: z.number().min(0).max(1).default(0.05),
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
]);

export type CorpusRoundPayload = z.infer<typeof corpusRoundPayloadSchema>;
