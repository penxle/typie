import { z } from 'zod';

export const roundPayloadSchema = z.discriminatedUnion('stage', [
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('screening'),
    documents: z.array(z.object({ documentId: z.string().min(1), setIds: z.array(z.string().min(1)).min(2) })).min(1),
    overlapRatio: z.number().min(0).max(1).default(0.2),
  }),
  z.object({
    roundId: z.string().min(1),
    stage: z.literal('confirmation'),
    documents: z.array(z.object({ documentId: z.string().min(1), v0SetId: z.string().min(1), candidateSetId: z.string().min(1) })).min(1),
  }),
]);

export type RoundPayload = z.infer<typeof roundPayloadSchema>;
