import { error, json } from '@sveltejs/kit';
import { createInternalApi } from '$lib/server/internal-api.ts';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);

  try {
    const prompts = await api.current();
    return json({ prompts });
  } catch (err) {
    error(502, String(err).slice(0, 200));
  }
};
