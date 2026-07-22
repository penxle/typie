import { z } from 'zod';

export const runCreateSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('sampling'), corpusVersion: z.string().min(1), size: z.number().int().min(1) }),
  z.object({ kind: z.literal('pipeline'), promptVariantId: z.string().min(1), corpusVersion: z.string().min(1) }),
]);

export type RunCreatePayload = z.infer<typeof runCreateSchema>;
