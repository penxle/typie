import { error } from '@sveltejs/kit';
import { loadRoundView } from '$lib/server/round-view.ts';
import { createDb } from '../../../../../core/db.ts';
import { evaluationById } from '../../../../../core/registry.ts';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const view = await loadRoundView(createDb(platform.env.DB), params.id);
  if (!view) error(404, 'round not found');

  // 코어는 집계의 모양을 모른다 — 세대 id만 넘기고 화면이 모듈의 Summary를 끼운다.
  const resolved = evaluationById(view.round.evaluationId);
  return {
    view,
    generationId: resolved?.generation.id ?? null,
    evaluationLabel: resolved?.evaluation.label ?? view.round.evaluationId,
    stageCount: resolved?.evaluation.stages.length ?? 1,
  };
};
