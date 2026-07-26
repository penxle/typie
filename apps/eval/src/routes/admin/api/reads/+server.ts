import { error, json } from '@sveltejs/kit';
import { z } from 'zod';
import { createDb } from '$lib/server/db/index.ts';
import { parseJsonBody } from '$lib/server/http.ts';
import { createInternalApi } from '$lib/server/internal-api.ts';
import { intakePersonalDocuments, spawnPersonalRun } from '$lib/server/personal.ts';
import type { RequestHandler } from './$types';

const payloadSchema = z.object({
  documentIds: z.array(z.string().min(1)).min(1).max(20),
  promptSetId: z.string().min(1),
});

export const POST: RequestHandler = async ({ request, platform }) => {
  const parsed = payloadSchema.safeParse(await parseJsonBody(request));
  if (!parsed.success) {
    error(400, parsed.error.message);
  }

  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);
  const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);

  const intake = await intakePersonalDocuments(db, api, parsed.data.documentIds);
  // 새로 들인 것과 이미 있던 것을 함께 돌린다 — 같은 글을 다른 프롬프트 세트로 다시 돌리는 것이
  // 이 기능의 주된 쓰임이라, 이미 있다고 실행하지 않으면 되돌릴 길이 없다.
  const targets = [...intake.accepted, ...intake.reused];
  if (targets.length === 0) {
    return json({ accepted: [], reused: [], rejected: intake.rejected, run: null });
  }

  const run = await spawnPersonalRun(db, platform.env, {
    promptSetId: parsed.data.promptSetId,
    documentIds: targets.map((d) => d.id),
  });
  if ('error' in run) {
    // 문서는 이미 들어갔다. 지우지 않고 남긴다 — 같은 id로 다시 눌러 실행만 다시 걸 수 있다.
    error(400, run.error);
  }

  return json({ accepted: intake.accepted, reused: intake.reused, rejected: intake.rejected, run });
};
