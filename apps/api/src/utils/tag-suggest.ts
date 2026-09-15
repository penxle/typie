import { randomUUID } from 'node:crypto';
import {
  globalCurrentKey,
  globalDataKey,
  mergeSuggestions,
  prefixesOf,
  queryKey,
  readCount,
  siteCurrentKey,
  siteDataKey,
  TAG_SUGGEST_GLOBAL_CURRENT_TTL_SECONDS,
  TAG_SUGGEST_GLOBAL_DATA_TTL_SECONDS,
  TAG_SUGGEST_SITE_CURRENT_TTL_SECONDS,
  TAG_SUGGEST_SITE_DATA_TTL_SECONDS,
} from './tag-suggest-core.ts';
import type { TagSuggestions } from './tag-suggest-core.ts';

export type TagRow = { name: string; count: number };

export class TagSuggestStoreError extends Error {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = 'TagSuggestStoreError';
  }
}

export type TagSuggestStore = {
  get: (key: string) => Promise<string | null>;
  del: (key: string) => Promise<void>;
  readTop: (keys: string[], count: number) => Promise<TagRow[][]>;
  writeBuild: (input: {
    dataKeys: { key: string; members: TagRow[] }[];
    dataTtl: number;
    currentKey: string;
    buildId: string;
    currentTtl: number;
  }) => Promise<void>;
};

export type TagSuggestDeps = {
  store: TagSuggestStore;
  loadSiteTags: (siteId: string) => Promise<TagRow[]>;
  loadGlobalTags: () => Promise<TagRow[]>;
  findSiteId: (publicationId: string) => Promise<string | null>;
  newBuildId: () => string;
};

const wrap = async <T>(run: () => Promise<T>): Promise<T> => {
  try {
    return await run();
  } catch (err) {
    throw new TagSuggestStoreError('tag suggest store failed', { cause: err });
  }
};

const parseWithScores = (flat: string[]): TagRow[] => {
  const rows: TagRow[] = [];
  for (let index = 0; index + 1 < flat.length; index += 2) {
    rows.push({ name: flat[index], count: Number(flat[index + 1]) });
  }
  return rows;
};

const redisStore: TagSuggestStore = {
  get: async (key) => {
    const { redis } = await import('#/cache.ts');
    return await wrap(() => redis.get(key));
  },
  del: async (key) => {
    const { redis } = await import('#/cache.ts');
    await wrap(() => redis.del(key));
  },
  readTop: async (keys, count) => {
    const { redis } = await import('#/cache.ts');
    return await wrap(async () => {
      const pipeline = redis.pipeline();
      for (const key of keys) pipeline.zrevrange(key, 0, count - 1, 'WITHSCORES');
      const results = (await pipeline.exec()) ?? [];
      return results.map(([err, value]) => {
        if (err) throw err;
        return parseWithScores(value as string[]);
      });
    });
  },
  writeBuild: async ({ dataKeys, dataTtl, currentKey, buildId, currentTtl }) => {
    const { redis } = await import('#/cache.ts');
    await wrap(async () => {
      const pipeline = redis.pipeline();
      for (const { key, members } of dataKeys) {
        pipeline.zadd(key, ...members.flatMap((member) => [member.count, member.name]));
        pipeline.expire(key, dataTtl);
      }
      pipeline.set(currentKey, buildId, 'EX', currentTtl);
      const results = (await pipeline.exec()) ?? [];
      for (const [err] of results) if (err) throw err;
    });
  },
};

const defaultDeps: TagSuggestDeps = {
  store: redisStore,
  loadSiteTags: async (siteId) => {
    const { db } = await import('#/db/index.ts');
    const { buildSiteTagsQuery } = await import('./publication-view-core.ts');
    return await buildSiteTagsQuery(db, { siteId });
  },
  loadGlobalTags: async () => {
    const { db } = await import('#/db/index.ts');
    const { buildDiscoveryTagsQuery } = await import('./discovery-core.ts');
    return await buildDiscoveryTagsQuery(db, {});
  },
  findSiteId: async (publicationId) => {
    const { eq } = await import('drizzle-orm');
    const { db, Publications } = await import('#/db/index.ts');
    return await db
      .select({ siteId: Publications.siteId })
      .from(Publications)
      .where(eq(Publications.id, publicationId))
      .then((rows) => rows[0]?.siteId ?? null);
  },
  newBuildId: () => randomUUID(),
};

type Scope = {
  currentKey: string;
  dataKey: (buildId: string, prefix: string) => string;
  currentTtl: number;
  dataTtl: number;
  load: () => Promise<TagRow[]>;
};

const siteScope = (siteId: string, deps: TagSuggestDeps): Scope => ({
  currentKey: siteCurrentKey(siteId),
  dataKey: (buildId, prefix) => siteDataKey(siteId, buildId, prefix),
  currentTtl: TAG_SUGGEST_SITE_CURRENT_TTL_SECONDS,
  dataTtl: TAG_SUGGEST_SITE_DATA_TTL_SECONDS,
  load: () => deps.loadSiteTags(siteId),
});

const globalScope = (deps: TagSuggestDeps): Scope => ({
  currentKey: globalCurrentKey(),
  dataKey: globalDataKey,
  currentTtl: TAG_SUGGEST_GLOBAL_CURRENT_TTL_SECONDS,
  dataTtl: TAG_SUGGEST_GLOBAL_DATA_TTL_SECONDS,
  load: () => deps.loadGlobalTags(),
});

const ensureBuild = async (scope: Scope, deps: TagSuggestDeps): Promise<string> => {
  const current = await deps.store.get(scope.currentKey);
  if (current) return current;

  const rows = await scope.load();
  const buildId = deps.newBuildId();
  const byPrefix = new Map<string, TagRow[]>();
  for (const row of rows) {
    for (const prefix of prefixesOf(row.name)) {
      const members = byPrefix.get(prefix) ?? [];
      members.push(row);
      byPrefix.set(prefix, members);
    }
  }

  await deps.store.writeBuild({
    dataKeys: [...byPrefix].map(([prefix, members]) => ({ key: scope.dataKey(buildId, prefix), members })),
    dataTtl: scope.dataTtl,
    currentKey: scope.currentKey,
    buildId,
    currentTtl: scope.currentTtl,
  });

  return buildId;
};

export const getTagSuggestions = async (
  input: { siteId: string; query: string; exclude: string[] },
  deps: TagSuggestDeps = defaultDeps,
): Promise<TagSuggestions> => {
  try {
    const site = siteScope(input.siteId, deps);
    const global = globalScope(deps);
    const [siteBuild, globalBuild] = await Promise.all([ensureBuild(site, deps), ensureBuild(global, deps)]);

    const prefix = queryKey(input.query);
    const [mine, popular] = await deps.store.readTop(
      [site.dataKey(siteBuild, prefix), global.dataKey(globalBuild, prefix)],
      readCount(input.exclude.length),
    );

    return mergeSuggestions({ mine: mine ?? [], popular: popular ?? [], exclude: input.exclude });
  } catch (err) {
    if (err instanceof TagSuggestStoreError) return { mine: [], popular: [] };
    throw err;
  }
};

export const invalidateSiteTagSuggestions = async (publicationId: string, deps: TagSuggestDeps = defaultDeps): Promise<void> => {
  const siteId = await deps.findSiteId(publicationId);
  if (!siteId) return;
  await deps.store.del(siteCurrentKey(siteId));
};
