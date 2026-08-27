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

export type ToolPolicy = 'READ_ONLY' | 'STANDARD' | 'FULL';
export type ToolResolver = 'user' | 'client' | 'server';
export type ToolTier = 'read' | 'safe' | 'destructive';
export type ToolMeta = { resolver: ToolResolver; tier?: ToolTier };

export const TOOL_META: Record<string, ToolMeta> = {
  [ASK_USER_TOOL]: { resolver: 'user' },
  'confirm-review': { resolver: 'user' },
  'list-open-documents': { resolver: 'client' },
  'search-entities': { resolver: 'server', tier: 'read' },
  'list-entities': { resolver: 'server', tier: 'read' },
  'read-document': { resolver: 'server', tier: 'read' },
  'list-notes': { resolver: 'server', tier: 'read' },
  'read-note': { resolver: 'server', tier: 'read' },
  'read-stats': { resolver: 'server', tier: 'read' },
  'read-goals': { resolver: 'server', tier: 'read' },
  'read-sharing': { resolver: 'server', tier: 'read' },
  'read-comments': { resolver: 'server', tier: 'read' },
  'list-trash': { resolver: 'server', tier: 'read' },
  'list-icons': { resolver: 'server', tier: 'read' },
  'open-document': { resolver: 'server', tier: 'read' },
  'create-folders': { resolver: 'server', tier: 'safe' },
  'create-documents': { resolver: 'server', tier: 'safe' },
  'update-documents': { resolver: 'server', tier: 'safe' },
  'update-folders': { resolver: 'server', tier: 'safe' },
  'move-entities': { resolver: 'server', tier: 'safe' },
  'duplicate-documents': { resolver: 'server', tier: 'safe' },
  'create-notes': { resolver: 'server', tier: 'safe' },
  'update-notes': { resolver: 'server', tier: 'safe' },
  'attach-notes': { resolver: 'server', tier: 'safe' },
  'detach-notes': { resolver: 'server', tier: 'safe' },
  'set-goals': { resolver: 'server', tier: 'safe' },
  'update-icons': { resolver: 'server', tier: 'safe' },
  'recover-entities': { resolver: 'server', tier: 'safe' },
  'delete-entities': { resolver: 'user', tier: 'destructive' },
  'delete-notes': { resolver: 'user', tier: 'destructive' },
  'delete-goals': { resolver: 'user', tier: 'destructive' },
  'update-sharing': { resolver: 'user', tier: 'destructive' },
  'save-document': { resolver: 'user', tier: 'destructive' },
};

export const toolResolver = (tool: string): ToolResolver => TOOL_META[tool]?.resolver ?? 'user';

export const effectiveResolver = (tool: string, policy: ToolPolicy): ToolResolver => {
  const meta = TOOL_META[tool];
  if (meta === undefined) return 'user';
  if (meta.tier === undefined) return meta.resolver;
  if (meta.tier === 'destructive') return policy === 'STANDARD' ? 'user' : 'server';
  return 'server';
};

export const serveVerdict = (tool: string, policy: ToolPolicy): 'execute' | 'deny' | null => {
  if (effectiveResolver(tool, policy) !== 'server') return null;
  const tier = TOOL_META[tool]?.tier;
  if (tier === undefined) return null;
  if (tier === 'read') return 'execute';
  return policy === 'READ_ONLY' ? 'deny' : 'execute';
};

export const ToolFailureSchema = z.object({
  ok: z.literal(false),
  code: z.enum(['declined', 'denied', 'error']),
  message: z.string(),
});
export type ToolFailure = z.infer<typeof ToolFailureSchema>;
export const toolFailure = (code: ToolFailure['code'], message: string): ToolFailure => ({ ok: false, code, message });

export const DECLINED_MESSAGE = '작가가 이 행동을 하지 않기로 했어요 — 이유를 캐묻지 말고 대화를 이어가세요.';

export const ApproveInputSchema = z.object({ approve: z.boolean() });
