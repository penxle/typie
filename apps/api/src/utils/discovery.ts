import { redis } from '#/cache.ts';
import { db } from '#/db/index.ts';
import { buildDiscoveryTagsQuery, DISCOVERY_TAG_LIMIT } from './discovery-core.ts';

const CACHE_TTL_SECONDS = 300;

type DiscoveryTagRow = { name: string; count: number };

const getCachedTags = async (key: string, limit: number | undefined): Promise<DiscoveryTagRow[]> => {
  let cached: string | null;
  try {
    cached = await redis.get(key);
  } catch {
    cached = null;
  }

  if (cached) {
    return JSON.parse(cached) as DiscoveryTagRow[];
  }

  const rows = await buildDiscoveryTagsQuery(db, { limit });

  try {
    await redis.set(key, JSON.stringify(rows), 'EX', CACHE_TTL_SECONDS);
  } catch {
    return rows;
  }

  return rows;
};

export const getDiscoveryTags = async (): Promise<DiscoveryTagRow[]> => await getCachedTags('discovery:tags', DISCOVERY_TAG_LIMIT);

export const getAllDiscoveryTags = async (): Promise<DiscoveryTagRow[]> => await getCachedTags('discovery:tags:all', undefined);
