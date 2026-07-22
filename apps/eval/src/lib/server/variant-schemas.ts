import { z } from 'zod';

const stagePromptSchema = z.object({
  system: z.string().min(1),
  tools: z.record(z.string(), z.unknown()),
  model: z.string().min(1),
  effort: z.string().nullable(),
});

const variantContentSchema = z.object({
  summarize: stagePromptSchema,
  meta: stagePromptSchema,
  analyze: stagePromptSchema,
});

export const variantCreateSchema = z.object({
  label: z.string().min(1),
  note: z.string().nullable().optional(),
  content: variantContentSchema,
});

export type VariantCreatePayload = z.infer<typeof variantCreateSchema>;
