import assert from 'node:assert/strict';
import test from 'node:test';
import { enqueueDiscoveryPublicationSync, enqueueDiscoverySiteSync, enqueueDiscoverySyncForDocumentIds } from './discovery-index.ts';

test('publication sync enqueues one job per unique publication id in order', async () => {
  const jobs: { name: string; id: string }[] = [];
  await enqueueDiscoveryPublicationSync(['PUB0B', 'PUB0A', 'PUB0B'], {
    findPublicationIdsByDocumentIds: async () => [],
    enqueuePublicationIndexJob: async (id) => {
      jobs.push({ name: 'search:index:publication', id });
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- only the publication jobs are under test here
    enqueueSiteIndexJob: async () => {},
  });
  assert.deepEqual(jobs, [
    { name: 'search:index:publication', id: 'PUB0B' },
    { name: 'search:index:publication', id: 'PUB0A' },
  ]);
});

test('site sync enqueues one job per unique site id and nothing for an empty list', async () => {
  const jobs: string[] = [];
  const deps = {
    findPublicationIdsByDocumentIds: async () => [],
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- only the site jobs are under test here
    enqueuePublicationIndexJob: async () => {},
    enqueueSiteIndexJob: async (id: string) => {
      jobs.push(id);
    },
  };
  await enqueueDiscoverySiteSync(['S0A', 'S0A', 'S0B'], deps);
  await enqueueDiscoverySiteSync([], deps);
  assert.deepEqual(jobs, ['S0A', 'S0B']);
});

test('document sync resolves publications for the documents and enqueues only those', async () => {
  const jobs: string[] = [];
  await enqueueDiscoverySyncForDocumentIds(['DOC0A', 'DOC0B', 'DOC0A'], {
    findPublicationIdsByDocumentIds: async (documentIds) => {
      assert.deepEqual(documentIds, ['DOC0A', 'DOC0B']);
      return ['PUB0A'];
    },
    enqueuePublicationIndexJob: async (id) => {
      jobs.push(id);
    },
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- only the publication jobs are under test here
    enqueueSiteIndexJob: async () => {},
  });
  assert.deepEqual(jobs, ['PUB0A']);
});
