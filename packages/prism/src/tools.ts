import { z } from 'zod';

export const ASK_USER_TOOL = 'ask-user';

export const AskQuestionsSchema = z.object({
  questions: z.array(
    z.object({
      question: z.string(),
      hint: z.string(),
      multi: z.boolean(),
      options: z.array(z.object({ label: z.string(), description: z.string().optional() })),
    }),
  ),
});
export type AskQuestion = z.infer<typeof AskQuestionsSchema>['questions'][number];

export const AskAnswersSchema = z.object({ answers: z.array(z.object({ question: z.string(), choice: z.array(z.string()) })) });
export type AskAnswer = z.infer<typeof AskAnswersSchema>['answers'][number];

export type ToolResolver = 'user' | 'client';

export const TOOL_RESOLVERS: Record<string, ToolResolver> = {
  [ASK_USER_TOOL]: 'user',
  'confirm-review': 'user',
  'list-open-documents': 'client',
};

export const toolResolver = (tool: string): ToolResolver => TOOL_RESOLVERS[tool] ?? 'user';
