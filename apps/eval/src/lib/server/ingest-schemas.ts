import { z } from 'zod';

export const corpusPayloadSchema = z.object({
  corpusVersion: z.string().min(1),
  documents: z
    .array(
      z.object({
        id: z.string().min(1),
        refId: z.string().min(1),
        content: z.string().min(1),
        characterCount: z.number().int().nonnegative(),
      }),
    )
    .min(1),
});

export const runPayloadSchema = z.object({
  runId: z.string().min(1),
  variantLabel: z.string().min(1),
  round: z.string().min(1),
  corpusVersion: z.string().min(1),
  meta: z.record(z.string(), z.unknown()).optional(),
  sets: z
    .array(
      z.object({
        documentId: z.string().min(1),
        feedbacks: z.array(
          z.object({
            startText: z.string(),
            endText: z.string(),
            matchStart: z.number().int().nullable(),
            matchEnd: z.number().int().nullable(),
            category: z.string().nullable(),
            body: z.string().min(1),
          }),
        ),
      }),
    )
    .min(1),
});

export type CorpusPayload = z.infer<typeof corpusPayloadSchema>;
export type RunPayload = z.infer<typeof runPayloadSchema>;
