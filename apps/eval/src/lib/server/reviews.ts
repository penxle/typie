// cspell:ignore ZWSP

import { and, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { buildModelConfig } from '../feedback/tiers.ts';
import { FeedbackSessions, ManuscriptVersions, Reviews } from './db/index.ts';
import { fetchManuscript } from './ingest.ts';
import { createInternalApi } from './internal-api.ts';
import { cancelWorkflow, newPrismWorkflowId, startWorkflow } from './prism.ts';
import type { TierName, TierOverrides } from '../feedback/tiers.ts';
import type { Db } from './db/index.ts';

type Env = App.Platform['env'];

// 글자 수는 typie 정본 정의를 따른다 — ZWSP 제거 후 grapheme cluster, 공백 포함. editor-resource/src/character_count.rs
export const countChars = (content: string): number => [...new Intl.Segmenter().segment(content.replaceAll('\u{200B}', ''))].length;

export const buildStartRows = (input: {
  refId: string;
  email: string;
  title: string | null;
  content: string;
  prismWorkflowId: string;
  now: Date;
  tier?: TierName;
  overrides?: TierOverrides;
}) => {
  const sessionId = nanoid();
  // 티어 확정은 여기 한 곳이다 — 행에 저장하는 값과 워크플로 이름이 갈라지지 않는다.
  const tier = input.tier ?? 'high';
  return {
    session: { id: sessionId, refId: input.refId, title: input.title, testerEmail: input.email, createdAt: input.now },
    version: {
      sessionId,
      version: 1,
      content: input.content,
      charCount: countChars(input.content),
      importedAt: input.now,
    },
    review: {
      sessionId,
      round: 1,
      prismWorkflowId: input.prismWorkflowId,
      status: 'running' as const,
      manuscriptVersion: 1,
      startedAt: input.now,
      tier,
      modelConfig: buildModelConfig(tier, input.overrides),
    },
  };
};

export const startFeedbackSession = async (
  db: Db,
  env: Env,
  input: { refId: string; email: string; tier?: TierName; overrides?: TierOverrides },
): Promise<{ sessionId: string } | { error: string }> => {
  const api = createInternalApi(env.INTERNAL_API_BASE, env.INTERNAL_API_KEY);
  const manuscript = await fetchManuscript(api, input.refId);
  if ('error' in manuscript) return manuscript;

  const rows = buildStartRows({ ...input, ...manuscript, prismWorkflowId: newPrismWorkflowId(), now: new Date() });
  await db.batch([
    db.insert(FeedbackSessions).values(rows.session),
    db.insert(ManuscriptVersions).values(rows.version),
    db.insert(Reviews).values(rows.review),
  ]);

  try {
    await startWorkflow(env, {
      workflowId: rows.review.prismWorkflowId,
      workflow: rows.review.tier,
      input: {
        manuscriptPath: 'manuscript/v1.txt',
        // sparse — 무오버라이드 리뷰는 prism 기본값을 따른다(키 자체를 싣지 않는다)
        ...(input.overrides && Object.keys(input.overrides).length > 0 && { overrides: input.overrides }),
      },
      files: [{ path: 'manuscript/v1.txt', content: manuscript.content }],
    });
  } catch (err) {
    // 구 run-service 관례 승계: 시작 실패는 행에 귀속하고 화면이 사유를 보여 준다
    await db
      .update(Reviews)
      .set({ status: 'failed', error: String(err).slice(0, 1000), finishedAt: new Date() })
      .where(and(eq(Reviews.sessionId, rows.session.id), eq(Reviews.round, 1)));
  }
  return { sessionId: rows.session.id };
};

export const requestCancel = async (env: Env, prismWorkflowId: string): Promise<void> => {
  await cancelWorkflow(env, prismWorkflowId);
};
