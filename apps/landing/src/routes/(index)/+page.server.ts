import { STATS_QUERY } from '$lib/stats';
import type { Stats } from '$lib/stats';
import type { PageServerLoad } from './$types';

const FALLBACK_API_ORIGIN = 'https://api.typie.co';

export const load: PageServerLoad = async ({ platform, fetch }) => {
  const origin = platform?.env?.API_ORIGIN ?? FALLBACK_API_ORIGIN;

  try {
    const response = await fetch(`${origin}/graphql`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ query: STATS_QUERY }),
    });
    const json = (await response.json()) as { data?: { stats?: Stats } };
    return { stats: json.data?.stats ?? null };
  } catch {
    return { stats: null };
  }
};
