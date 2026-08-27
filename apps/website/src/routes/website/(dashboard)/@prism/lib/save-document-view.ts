import { z } from 'zod';

const PATH = /^documents\/([^/]+)\.xml$/;

const InputSchema = z.object({
  path: z.string().regex(PATH),
  summary: z.string().trim().min(1),
});

export const saveDocumentView = (input: unknown): { documentId: string; summary: string } | null => {
  const parsed = InputSchema.safeParse(input);
  if (!parsed.success) return null;

  const documentId = PATH.exec(parsed.data.path)?.[1];
  return documentId === undefined ? null : { documentId, summary: parsed.data.summary };
};
