import crypto from 'node:crypto';
import { z } from 'zod';

export const verifyInternalKey = (header: string | undefined, key: string): boolean => {
  if (!header) return false;
  const token = header.startsWith('Bearer ') ? header.slice(7) : header;
  const a = Buffer.from(token);
  const b = Buffer.from(key);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
};

export const hangulRatio = (text: string): number => {
  const chars = [...text.replaceAll(/\s/g, '')];
  if (chars.length === 0) return 0;
  return chars.filter((ch) => /[가-힣ㄱ-ㅎㅏ-ㅣ]/.test(ch)).length / chars.length;
};

export const promptUpdateSchema = z.object({
  model: z.string().min(1),
  effort: z.string().nullable(),
  systemPrompt: z.string().min(1),
  toolDescriptions: z.record(z.string(), z.unknown()),
});

export const pushSchema = z.object({ documentId: z.string().min(1), title: z.string().min(1), body: z.string().min(1) });
