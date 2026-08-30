import { createHash } from 'node:crypto';
import type { LlmCallState } from '@typie/lib/enums';
import type OpenAI from 'openai';

const PREFIX_LENGTH = 1000;

const normalize = (text: string) => text.trim().replaceAll(/\s+/gu, ' ');

const hash16 = (text: string) => createHash('sha256').update(text).digest('hex').slice(0, 16);

export const textIdentity = (text: string) => {
  const normalized = normalize(text);

  return {
    textLength: [...text].length,
    prefixHash: hash16([...normalized].slice(0, PREFIX_LENGTH).join('')),
    fullHash: hash16(normalized),
  };
};

type CompletionUsage = NonNullable<OpenAI.Chat.Completions.ChatCompletionChunk['usage']>;

export const extractUsage = (usage: CompletionUsage | null | undefined) => ({
  inputTokens: usage?.prompt_tokens ?? null,
  outputTokens: usage?.completion_tokens ?? null,
  cachedInputTokens: usage?.prompt_tokens_details?.cached_tokens ?? null,
  reasoningTokens: usage?.completion_tokens_details?.reasoning_tokens ?? null,
  totalTokens: usage?.total_tokens ?? null,
});

export const extractGatewayHeaders = (response: Response) => ({
  cacheStatus: response.headers.get('cf-aig-cache-status'),
  gatewayLogId: response.headers.get('cf-aig-log-id'),
});

export type UsageRecord = {
  phase: string;
  model: string;
  inputTokens: number | null;
  outputTokens: number | null;
  cachedInputTokens: number | null;
  reasoningTokens: number | null;
  totalTokens: number | null;
  inputChars: number;
  durationMs: number;
  cacheStatus: string | null;
  gatewayLogId: string | null;
  state: LlmCallState;
};
