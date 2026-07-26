import { z } from 'zod';

export const runCreateSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('sampling'), corpusVersion: z.string().min(1), size: z.number().int().min(1) }),
  z.object({
    kind: z.literal('pipeline'),
    promptVariantId: z.string().min(1),
    corpusVersion: z.string().min(1),
    // 부분집합 실행 — 모델·프롬프트 파일럿에서 코퍼스 일부만 돌린다. 생략하면 코퍼스 전체.
    documentIds: z.array(z.string().min(1)).min(1).optional(),
  }),
  z.object({
    kind: z.literal('analysis'),
    promptSetId: z.string().min(1),
    corpusVersion: z.string().min(1),
    documentIds: z.array(z.string().min(1)).min(1).optional(),
  }),
]);

export type RunCreatePayload = z.infer<typeof runCreateSchema>;
