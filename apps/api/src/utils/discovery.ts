import { redis } from '#/cache.ts';
import { db } from '#/db/index.ts';
import { buildDiscoveryTagsQuery, DISCOVERY_TAG_LIMIT } from './discovery-core.ts';

const CACHE_KEY = 'discovery:tags';
const CACHE_TTL_SECONDS = 300;

type DiscoveryTagRow = { name: string; count: number };

export const getDiscoveryTags = async (): Promise<DiscoveryTagRow[]> => {
  let cached: string | null;
  try {
    cached = await redis.get(CACHE_KEY);
  } catch {
    cached = null;
  }

  if (cached) {
    return JSON.parse(cached) as DiscoveryTagRow[];
  }

  const rows = await buildDiscoveryTagsQuery(db, { limit: DISCOVERY_TAG_LIMIT });

  try {
    await redis.set(CACHE_KEY, JSON.stringify(rows), 'EX', CACHE_TTL_SECONDS);
  } catch {
    return rows;
  }

  return rows;
};
