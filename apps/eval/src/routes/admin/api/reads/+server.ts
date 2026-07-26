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
  if (intake.accepted.length === 0) {
    return json({ accepted: [], rejected: intake.rejected, run: null });
  }

  const run = await spawnPersonalRun(db, platform.env, {
    promptSetId: parsed.data.promptSetId,
    documentIds: intake.accepted.map((d) => d.id),
  });
  if ('error' in run) {
    // 문서는 이미 들어갔다. 지우지 않고 남긴다 — 같은 id로 다시 시도하면 '이미 들여온 글'로
    // 걸러지고, 실행만 어드민 화면에서 다시 걸 수 있다.
    error(400, run.error);
  }

  return json({ accepted: intake.accepted, rejected: intake.rejected, run });
};
