import { z } from 'zod';

export const applySchema = z.object({
  promptVariantId: z.string().min(1),
  stage: z.enum(['summarize', 'meta', 'analyze']),
});

export const rollbackSchema = z.object({
  applyId: z.string().min(1),
});

export type ApplyPayload = z.infer<typeof applySchema>;
export type RollbackPayload = z.infer<typeof rollbackSchema>;
