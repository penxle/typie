import assert from 'node:assert/strict';
import test from 'node:test';
import { getTagSuggestions, invalidateSiteTagSuggestions, TagSuggestStoreError } from './tag-suggest.ts';
import type { TagRow, TagSuggestDeps, TagSuggestStore } from './tag-suggest.ts';

type Fake = {
  store: TagSuggestStore;
  strings: Map<string, string>;
  zsets: Map<string, TagRow[]>;
  ops: string[];
  failing: boolean;
};

const createFakeStore = (): Fake => {
  const strings = new Map<string, string>();
  const zsets = new Map<string, TagRow[]>();
  const ops: string[] = [];
  const fake: Fake = {
    strings,
    zsets,
    ops,
    failing: false,
    store: {
      get: async (key) => {
        if (fake.failing) throw new TagSuggestStoreError('down');
        ops.push(`get ${key}`);
        return strings.get(key) ?? null;
      },
      del: async (key) => {
        if (fake.failing) throw new TagSuggestStoreError('down');
        ops.push(`del ${key}`);
        strings.delete(key);
      },
      readTop: async (keys, count) => {
        if (fake.failing) throw new TagSuggestStoreError('down');
        ops.push(`readTop ${keys.join(',')} ${count}`);
        return keys.map((key) => (zsets.get(key) ?? []).toSorted((a, b) => b.count - a.count).slice(0, count));
      },
      writeBuild: async ({ dataKeys, currentKey, buildId }) => {
        if (fake.failing) throw new TagSuggestStoreError('down');
        for (const { key, members } of dataKeys) zsets.set(key, members);
        ops.push(`writeBuild ${dataKeys.length} keys`);
        strings.set(currentKey, buildId);
        ops.push(`set ${currentKey} ${buildId}`);
      },
    },
  };
  return fake;
};

const createDeps = (fake: Fake, rows: { site: TagRow[]; global: TagRow[] }) => {
  const calls = { site: 0, global: 0 };
  let builds = 0;
  const deps: TagSuggestDeps = {
    store: fake.store,
    loadSiteTags: async () => {
      calls.site += 1;
      return rows.site;
    },
    loadGlobalTags: async () => {
      calls.global += 1;
      return rows.global;
    },
    findSiteId: async (publicationId) => (publicationId === 'PUB1' ? 'S1' : null),
    newBuildId: () => `b${(builds += 1)}`,
  };
  return { deps, calls };
};

const rows = {
  site: [
    { name: '봄', count: 3 },
    { name: '바다 여행', count: 1 },
  ],
  global: [
    { name: '바다', count: 50 },
    { name: '봄', count: 40 },
  ],
};

test('rebuilds both scopes on a cold cache, writing the pointer after the data', async () => {
  const fake = createFakeStore();
  const { deps, calls } = createDeps(fake, rows);

  const result = await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);

  assert.deepEqual(calls, { site: 1, global: 1 });
  assert.deepEqual(result.mine, [
    { name: '봄', count: 3 },
    { name: '바다 여행', count: 1 },
  ]);
  assert.deepEqual(result.popular, [{ name: '바다', count: 50 }]);

  const siteWrite = fake.ops.findIndex((op) => op.startsWith('writeBuild'));
  const siteSet = fake.ops.findIndex((op) => op.startsWith('set tag-suggest:site:S1:current'));
  assert.ok(siteWrite !== -1 && siteSet > siteWrite);
  assert.equal(fake.strings.get('tag-suggest:site:S1:current'), 'b1');
  assert.equal(fake.strings.get('tag-suggest:global:current'), 'b2');
});

test('serves from the cache without touching the database once pointers exist', async () => {
  const fake = createFakeStore();
  const { deps, calls } = createDeps(fake, rows);
  await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);

  const result = await getTagSuggestions({ siteId: 'S1', query: '바', exclude: [] }, deps);

  assert.deepEqual(calls, { site: 1, global: 1 });
  assert.deepEqual(result.mine, [{ name: '바다 여행', count: 1 }]);
  assert.deepEqual(result.popular, [{ name: '바다', count: 50 }]);
});

test('matches later tokens by prefix', async () => {
  const fake = createFakeStore();
  const { deps } = createDeps(fake, rows);

  const result = await getTagSuggestions({ siteId: 'S1', query: '여행', exclude: [] }, deps);

  assert.deepEqual(result.mine, [{ name: '바다 여행', count: 1 }]);
  assert.deepEqual(result.popular, []);
});

test('an empty site still gets a pointer so the next read skips the database', async () => {
  const fake = createFakeStore();
  const { deps, calls } = createDeps(fake, { site: [], global: rows.global });

  await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);
  const result = await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);

  assert.equal(calls.site, 1);
  assert.deepEqual(result.mine, []);
  assert.equal(fake.strings.get('tag-suggest:site:S1:current'), 'b1');
});

test('returns empty sections when the store fails', async () => {
  const fake = createFakeStore();
  const { deps } = createDeps(fake, rows);
  fake.failing = true;

  const result = await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);

  assert.deepEqual(result, { mine: [], popular: [] });
});

test('database errors propagate', async () => {
  const fake = createFakeStore();
  const { deps } = createDeps(fake, rows);
  deps.loadSiteTags = async () => {
    throw new Error('db down');
  };

  await assert.rejects(getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps), /db down/);
});

test('invalidation deletes the site pointer and the next read rebuilds under a new build', async () => {
  const fake = createFakeStore();
  const { deps, calls } = createDeps(fake, rows);
  await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);

  await invalidateSiteTagSuggestions('PUB1', deps);
  assert.equal(fake.strings.has('tag-suggest:site:S1:current'), false);
  assert.equal(fake.strings.get('tag-suggest:global:current'), 'b2');

  await getTagSuggestions({ siteId: 'S1', query: '', exclude: [] }, deps);
  assert.deepEqual(calls, { site: 2, global: 1 });
  assert.equal(fake.strings.get('tag-suggest:site:S1:current'), 'b3');
});

test('invalidation is a no-op for an unknown publication', async () => {
  const fake = createFakeStore();
  const { deps } = createDeps(fake, rows);

  await invalidateSiteTagSuggestions('PUB-none', deps);

  assert.deepEqual(fake.ops, []);
});

test('invalidation propagates store errors', async () => {
  const fake = createFakeStore();
  const { deps } = createDeps(fake, rows);
  fake.failing = true;

  await assert.rejects(invalidateSiteTagSuggestions('PUB1', deps), TagSuggestStoreError);
});
