import { createHash } from 'node:crypto';
import * as Sentry from '@sentry/node';
import { LlmAnalysisRunState, LlmCallState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { db, LlmAnalysisRuns, LlmCallUsage } from '#/db/index.ts';
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

type CreateUsageTrackerParams = {
  userId: string;
  documentId: string | null;
  text: string;
  chunkCount: number;
};

export const createUsageTracker = async ({ userId, documentId, text, chunkCount }: CreateUsageTrackerParams) => {
  const identity = textIdentity(text);

  let runId: string | null = null;
  try {
    const [row] = await db
      .insert(LlmAnalysisRuns)
      .values({
        userId,
        documentId,
        textLength: identity.textLength,
        chunkCount,
        prefixHash: identity.prefixHash,
        fullHash: identity.fullHash,
        state: LlmAnalysisRunState.RUNNING,
      })
      .returning({ id: LlmAnalysisRuns.id });
    runId = row.id;
  } catch (err: unknown) {
    Sentry.captureException(err);
  }

  let buffer: UsageRecord[] = [];

  // buffer를 먼저 새 배열로 교체하고 insert한다 — await 중에 들어온 record는 새 buffer에 쌓여 다음 flush로 넘어간다
  const flush = async () => {
    if (!runId || buffer.length === 0) {
      return;
    }

    const entries = buffer;
    buffer = [];
    try {
      await db.insert(LlmCallUsage).values(entries.map((entry) => ({ ...entry, runId })));
    } catch (err: unknown) {
      Sentry.captureException(err);
    }
  };

  return {
    record: (entry: UsageRecord) => {
      buffer.push(entry);
    },
    flush,
    finish: async (state: LlmAnalysisRunState) => {
      // 중단 시 여러 호출이 동시에 거부되는데, 첫 flush가 insert를 await하는 동안 도착한 record는
      // 교체된 buffer에 쌓인다. 두 번째 flush가 그것을 거둔다 — 비어 있으면 무동작이라 정상 경로엔 비용이 없다.
      await flush();
      await flush();
      if (!runId) {
        return;
      }

      try {
        await db.update(LlmAnalysisRuns).set({ state, endedAt: dayjs() }).where(eq(LlmAnalysisRuns.id, runId));
      } catch (err: unknown) {
        Sentry.captureException(err);
      }
    },
  };
};
